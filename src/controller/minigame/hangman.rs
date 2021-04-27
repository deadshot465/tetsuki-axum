use crate::model::{
    EmbedObject, HangmanData, Message, MinigameProgressRequest, MinigameProgressResponse,
    MinigameRequestUser, MinigameStatus, TORAHIKO_COLOR,
};
use crate::shared::{
    create_new_followup_url, BASE_URL, CREATE_MESSAGE_ENDPOINT, EDIT_DELETE_MESSAGE_ENDPOINT,
    JSON_HEADER,
};
use actix_web::{post, HttpResponse, Responder};
use chrono::{DateTime, Utc};
use dashmap::DashMap;
use futures::{StreamExt, TryStreamExt};
use once_cell::sync::OnceCell;
use rand::prelude::*;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

const MAX_ATTEMPTS: i32 = 10;
const HANGMAN_THUMBNAIL: &str =
    "https://cdn.discordapp.com/attachments/700003813981028433/736202279983513671/unnamed.png";

static WORDS: OnceCell<Vec<String>> = OnceCell::new();
static ONGOING_GAMES: OnceCell<Arc<DashMap<u64, Vec<(MinigameRequestUser, Mutex<HangmanData>)>>>> =
    OnceCell::new();

pub async fn start_hangman(
    user: MinigameRequestUser,
    channel_id: u64,
    application_id: u64,
    interaction_token: String,
) -> anyhow::Result<()> {
    // Set desired word.
    let word = {
        let mut rng = rand::thread_rng();
        WORDS
            .get_or_init(|| {
                let bytes = std::fs::read("./asset/game/words.json")
                    .expect("Failed to read words from the disk.");
                serde_json::from_slice::<Vec<String>>(&bytes)
                    .expect("Failed to deserialize words from JSON.")
            })
            .choose(&mut rng)
            .cloned()
            .unwrap_or_default()
    };

    // Construct followup URL and message.
    let followup_url = create_new_followup_url(application_id, &interaction_token);
    let mut followup_message = std::collections::HashMap::new();
    followup_message.insert(
        "content",
        format!("There are {} letters in this word.", word.chars().count()),
    );

    // Wait for 2 seconds to continue.
    actix_web::rt::time::sleep(Duration::from_secs(2)).await;
    let client = awc::Client::default();
    client
        .post(&followup_url)
        .send_json(&followup_message)
        .await
        .expect("Failed to send followup URL request.");
    actix_web::rt::time::sleep(Duration::from_secs(2)).await;
    let mut followup_message = std::collections::HashMap::new();
    let description = format!("You have {} attempts left.", MAX_ATTEMPTS);
    let title: String = word
        .chars()
        .map(|_| "\\_".to_string())
        .collect::<Vec<_>>()
        .join(" ");
    let embed_object = EmbedObject::new()
        .author(&user.nickname, Some(user.avatar_url.clone()), None, None)
        .color(TORAHIKO_COLOR)
        .description(&description)
        .title(&title)
        .thumbnail(HANGMAN_THUMBNAIL, None, None, None)
        .footer(
            "Hangman original Python version made by: @Kirito#9286",
            None,
            None,
        );
    followup_message.insert("embeds", vec![embed_object.clone()]);
    let original_embed_message = client
        .post(&followup_url)
        .send_json(&followup_message)
        .await
        .expect("Failed to send followup URL request.")
        .json::<Message>()
        .await?;
    let original_embed_id = original_embed_message.id;

    actix_web::rt::time::sleep(Duration::from_secs(1)).await;
    let mut followup_message = std::collections::HashMap::new();
    followup_message.insert("content", "Input a letter: ");
    client
        .post(&followup_url)
        .send_json(&followup_message)
        .await
        .expect("Failed to send followup URL request.");

    {
        let ongoing_games = ONGOING_GAMES.get_or_init(|| Arc::new(DashMap::new()));
        let mut channel = ongoing_games.entry(channel_id).or_insert_with(Vec::new);
        channel.push((
            user,
            Mutex::new(HangmanData {
                attempts: 0,
                previous_guesses: vec![],
                word,
                last_reply_time: Utc::now(),
                original_embed: embed_object,
                original_embed_id,
            }),
        ));
    }

    Ok(())
}

#[post("/minigame/progress")]
pub async fn handle_hangman(
    request_data: actix_web::web::Json<MinigameProgressRequest>,
) -> impl Responder {
    let ongoing_games = ONGOING_GAMES.get_or_init(|| Arc::new(DashMap::new()));
    let actix_web::web::Json(data) = request_data;
    match data {
        MinigameProgressRequest::Hangman {
            user_id,
            message,
            channel_id,
            guild_id,
            message_id,
        } => {
            let mut game_stale = false;
            if let Some(gaming_members) = ongoing_games.get(&channel_id) {
                if let Some((_, data)) = gaming_members.iter().find(|(m, _)| m.user_id == user_id) {
                    let data_lock = data.lock().expect("Failed to lock user's hangman data.");
                    if (Utc::now() - data_lock.last_reply_time.clone()).num_seconds() > 60 {
                        game_stale = true;
                    }
                }
            }

            if game_stale {
                if let Some(mut gaming_members) = ongoing_games.get_mut(&channel_id) {
                    gaming_members.retain(|(m, _)| m.user_id != user_id);
                }
                HttpResponse::Ok().json(MinigameProgressResponse {
                    status: MinigameStatus::Stale,
                })
            } else {
                progress_hangman(user_id, message, channel_id, guild_id, message_id)
                    .await
                    .expect("Failed to progress the hangman game.")
            }
        }
        _ => HttpResponse::BadRequest().finish(),
    }
}

async fn progress_hangman(
    user_id: u64,
    message: String,
    channel_id: u64,
    guild_id: u64,
    message_id: u64,
) -> anyhow::Result<HttpResponse> {
    let ongoing_games = ONGOING_GAMES.get_or_init(|| Arc::new(DashMap::new()));
    let gaming_members = ongoing_games
        .get(&channel_id)
        .expect("Failed to get gaming members in the channel.");
    let (member, data) = gaming_members
        .iter()
        .find(|(m, _)| m.user_id == user_id)
        .expect("Failed to get the member to process among existing members.");

    let client = awc::Client::default();
    let delete_url = (BASE_URL.to_string() + EDIT_DELETE_MESSAGE_ENDPOINT)
        .replace("{channel_id}", &channel_id.to_string())
        .replace("{message_id}", &message_id.to_string());
    let bot_token = std::env::var("TORA_TOKEN")?;

    // Delete provided message.
    client
        .delete(&delete_url)
        .append_header(("Authorization", format!("Bot {}", &bot_token)))
        .send()
        .await
        .expect("Failed to delete the message.");

    let create_message_url = (BASE_URL.to_string() + CREATE_MESSAGE_ENDPOINT)
        .replace("{channel_id}", &channel_id.to_string());

    let mut message_data = HashMap::new();
    // Check if the provided answer is an ASCII alphabetic letter.
    if let Some(char) = message.chars().next() {
        if !char.is_ascii_alphabetic() {
            message_data.insert(
                "content",
                format!(
                    "{} The answer has to be an English letter!\nInput a letter:",
                    format!("<@!{}>", user_id)
                ),
            );
            client
                .post(&create_message_url)
                .append_header(("Authorization", format!("Bot {}", &bot_token)))
                .append_header(JSON_HEADER.clone())
                .send_json(&message_data)
                .await
                .expect("Failed to post a message to the channel.");
            return Ok(HttpResponse::Ok().finish());
        } else if message.trim().chars().count() != 1 {
            message_data.insert(
                "content",
                format!(
                    "{} The answer has to be only one letter!\nInput a letter:",
                    format!("<@!{}>", user_id)
                ),
            );
            client
                .post(&create_message_url)
                .append_header(("Authorization", format!("Bot {}", &bot_token)))
                .append_header(JSON_HEADER.clone())
                .send_json(&message_data)
                .await
                .expect("Failed to post a message to the channel.");
            return Ok(HttpResponse::Ok().finish());
        }

        {
            let mut data_lock = data.lock().expect("Failed to lock user's hangman data.");
            // Add the letter to the existing guesses and deduplicate.
            data_lock
                .previous_guesses
                .push(char.to_ascii_uppercase().to_string());
            data_lock.previous_guesses.dedup();
            // Update last reply time.
            data_lock.last_reply_time = Utc::now();
        }
    }

    Ok(HttpResponse::Ok().finish())
}

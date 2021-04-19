use crate::model::{EmbedObject, MinigameRequestUser, TORAHIKO_COLOR};
use crate::shared::create_new_followup_url;
use chrono::{DateTime, Utc};
use dashmap::DashMap;
use futures::{StreamExt, TryStreamExt};
use once_cell::sync::OnceCell;
use rand::prelude::*;
use std::sync::Arc;
use std::time::Duration;

const MAX_ATTEMPTS: i32 = 10;
const HANGMAN_THUMBNAIL: &str =
    "https://cdn.discordapp.com/attachments/700003813981028433/736202279983513671/unnamed.png";

static WORDS: OnceCell<Vec<String>> = OnceCell::new();
static ONGOING_GAMES: OnceCell<Arc<DashMap<u64, Vec<(MinigameRequestUser, DateTime<Utc>)>>>> =
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
    followup_message.insert(
        "embeds",
        vec![EmbedObject::new()
            .author(&user.nickname, Some(user.avatar_url.clone()), None, None)
            .color(TORAHIKO_COLOR)
            .description(&description)
            .title(&title)
            .thumbnail(HANGMAN_THUMBNAIL, None, None, None)
            .footer(
                "Hangman original Python version made by: @Kirito#9286",
                None,
                None,
            )],
    );
    client
        .post(&followup_url)
        .send_json(&followup_message)
        .await
        .expect("Failed to send followup URL request.");
    {
        let ongoing_games = ONGOING_GAMES.get_or_init(|| Arc::new(DashMap::new()));
        let mut channel = ongoing_games.entry(channel_id).or_insert_with(Vec::new);
        channel.push((user, Utc::now()));
    }

    Ok(())
}

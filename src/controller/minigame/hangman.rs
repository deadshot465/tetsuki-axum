use crate::shared::create_new_followup_url;
use futures::TryStreamExt;
use once_cell::sync::OnceCell;
use rand::prelude::*;
use std::time::Duration;

static WORDS: OnceCell<Vec<String>> = OnceCell::new();

pub async fn start_hangman(
    user_id: u64,
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
    actix_web::rt::time::sleep(Duration::from_secs(2));
    let client = awc::Client::default();
    let response = client
        .post(&followup_url)
        .send_json(&followup_message)
        .await
        .expect("Failed to send followup URL request.");
    response.and_then(|res| {
        println!(
            "{:?}",
            serde_json::from_slice::<String>(&res).expect("Failed to deserialize JSON response.")
        );
        futures::future::ok(())
    });

    Ok(())
}

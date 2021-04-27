use crate::model::EmbedObject;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(tag = "type")]
pub enum MinigameRequest {
    Quiz {
        user: MinigameRequestUser,
        channel_id: u64,
        application_id: u64,
        interaction_token: String,
    },
    Hangman {
        user: MinigameRequestUser,
        channel_id: u64,
        application_id: u64,
        interaction_token: String,
    },
    Tictactoe {
        user: MinigameRequestUser,
        channel_id: u64,
        application_id: u64,
        interaction_token: String,
    },
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct MinigameRequestUser {
    pub user_id: u64,
    pub nickname: String,
    pub avatar_url: String,
}

#[derive(Deserialize, Serialize, Debug, Copy, Clone)]
pub enum MinigameStatus {
    InProgress,
    Stale,
}

#[derive(Deserialize, Serialize, Debug, Copy, Clone)]
pub struct MinigameProgressResponse {
    pub status: MinigameStatus,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(tag = "type")]
pub enum MinigameProgressRequest {
    Quiz {
        user_id: u64,
        message: String,
        channel_id: u64,
        guild_id: u64,
        message_id: u64,
    },
    Hangman {
        user_id: u64,
        message: String,
        channel_id: u64,
        guild_id: u64,
        message_id: u64,
    },
    Tictactoe {
        user_id: u64,
        message: String,
        channel_id: u64,
        guild_id: u64,
        message_id: u64,
    },
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct HangmanData {
    pub attempts: u8,
    pub previous_guesses: Vec<String>,
    pub word: String,
    pub last_reply_time: DateTime<Utc>,
    pub original_embed: EmbedObject,
    pub original_embed_id: String,
}

use serde::{Deserialize, Serialize};

pub mod embed;

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Message {
    pub id: String,
    pub message_reference: MessageReference,
}

impl Default for Message {
    fn default() -> Self {
        Self::new()
    }
}

impl Message {
    pub fn new() -> Self {
        Message {
            id: String::new(),
            message_reference: MessageReference::new(),
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct MessageReference {
    pub channel_id: String,
    pub guild_id: String,
    pub message_id: String,
}

impl Default for MessageReference {
    fn default() -> Self {
        Self::new()
    }
}

impl MessageReference {
    pub fn new() -> Self {
        MessageReference {
            channel_id: String::new(),
            guild_id: String::new(),
            message_id: String::new(),
        }
    }
}

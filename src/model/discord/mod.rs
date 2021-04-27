use serde::{Deserialize, Serialize};

mod embed;
mod game;
pub use embed::*;
pub use game::*;

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Message {
    pub id: String,
}

impl Default for Message {
    fn default() -> Self {
        Self::new()
    }
}

impl Message {
    pub fn new() -> Self {
        Message { id: String::new() }
    }
}

use azure_data_cosmos::CosmosEntity;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct MessageInfo {
    pub bot_id: String,
    pub user_id: String,
    pub user_name: Option<String>,
    pub message: String,
    pub post_at: String,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct MessageRecord {
    pub id: String,
    pub bot_id: String,
    pub user_id: String,
    pub user_name: Option<String>,
    pub message: String,
    pub post_at: String,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct MessageRecordSimple {
    pub user_name: String,
    pub message: String,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct GetCompletionRequest {
    pub bot_id: String,
    pub user_id: String,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct GetCompletionResponse {
    pub bot_id: String,
    pub user_id: String,
    pub messages: Vec<String>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct GetMessageRequest {
    pub bot_id: String,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct GetMessageResponse {
    pub bot_id: String,
    pub messages: Vec<MessageRecordSimple>,
}

impl Into<MessageRecord> for MessageInfo {
    fn into(self) -> MessageRecord {
        MessageRecord {
            id: "".to_string(),
            bot_id: self.bot_id,
            user_id: self.user_id,
            user_name: self.user_name,
            message: self.message,
            post_at: self.post_at,
        }
    }
}

impl CosmosEntity for MessageRecord {
    type Entity = String;

    fn partition_key(&self) -> Self::Entity {
        self.id.clone()
    }
}

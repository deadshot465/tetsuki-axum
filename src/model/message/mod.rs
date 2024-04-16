use azure_data_cosmos::CosmosEntity;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct MessageInfo {
    pub bot_id: String,
    pub user_id: String,
    pub user_name: Option<String>,
    pub generated_by: String,
    pub message: String,
    pub message_type: String,
    pub channel_id: String,
    pub post_at: String,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct MessageRecord {
    pub id: String,
    pub bot_id: String,
    pub user_id: String,
    pub user_name: Option<String>,
    pub generated_by: String,
    pub message: String,
    pub message_type: String,
    pub channel_id: String,
    pub post_at: String,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct MessageRecordSimple {
    pub user_id: String,
    pub user_name: String,
    pub message: String,
    pub message_type: String,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct CompletionRecordSimple {
    pub message_type: String,
    pub message: String,
    pub generated_by: String,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct GetCompletionRequest {
    pub bot_id: String,
    pub user_id: String,
    pub channel_id: String,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct GetCompletionResponse {
    pub bot_id: String,
    pub user_id: String,
    pub messages: Vec<CompletionRecordSimple>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct GetMessageRequest {
    pub bot_id: String,
    pub channel_id: String,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct GetMessageResponse {
    pub bot_id: String,
    pub messages: Vec<MessageRecordSimple>,
}

impl From<MessageInfo> for MessageRecord {
    fn from(value: MessageInfo) -> Self {
        MessageRecord {
            id: "".to_string(),
            bot_id: value.bot_id,
            user_id: value.user_id,
            user_name: value.user_name,
            message: value.message,
            message_type: value.message_type,
            channel_id: value.channel_id,
            post_at: value.post_at,
            generated_by: value.generated_by,
        }
    }
}

impl CosmosEntity for MessageRecord {
    type Entity = String;

    fn partition_key(&self) -> Self::Entity {
        self.id.clone()
    }
}

pub const BASE_URL: &str = "https://discord.com/api/v8";
pub const CREATE_MESSAGE_ENDPOINT: &str = "/channels/{channel_id}/messages";
pub const EDIT_DELETE_MESSAGE_ENDPOINT: &str = "/channels/{channel_id}/messages/{message_id}";

pub fn create_new_followup_url(application_id: u64, token: &str) -> String {
    format!(
        "https://discord.com/api/webhooks/{}/{}",
        application_id, token
    )
}

use crate::model::MinigameRequestUser;
use crate::shared::create_new_followup_url;

pub async fn start_quiz(
    user: MinigameRequestUser,
    channel_id: u64,
    application_id: u64,
    interaction_token: String,
) -> anyhow::Result<()> {
    let followup_url = create_new_followup_url(application_id, &interaction_token);

    Ok(())
}

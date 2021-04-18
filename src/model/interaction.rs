use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct InteractionFollowupUrlData {
    pub application_id: u64,
    pub interaction_token: String,
}

use serde::{Serialize, Deserialize};

#[derive(Deserialize, Serialize, sqlx::FromRow, Debug, Clone)]
pub struct PlayerRecordPayload {
    pub id: Option<i64>,
    pub player_name: String,
    pub score: i32,
    pub stage: i32,
}
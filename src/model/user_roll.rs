use crate::model::mal_character::MalCharacter;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Clone, Debug, Default)]
pub struct UserRoll {
    #[serde(rename = "Id")]
    pub roll_id: i32,
    #[serde(rename = "UserId")]
    pub user_id: String,
    #[serde(rename = "MalCharacterId")]
    pub mal_character_id: i32,
    #[serde(rename = "CreatedAt")]
    pub created_at: String,
    pub id: String,
}

#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub struct GetRollResult {
    pub user_roll: UserRoll,
    pub mal_character: MalCharacter,
}

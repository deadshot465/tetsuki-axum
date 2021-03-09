use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, sqlx::FromRow)]
#[sqlx(rename_all = "PascalCase")]
#[serde(rename = "PascalCase")]
pub struct DialogInfo {
    pub id: Option<i64>,
    pub background: String,
    pub character: String,
    pub text: String,
}

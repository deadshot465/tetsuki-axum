use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, sqlx::FromRow, Clone)]
#[sqlx(rename_all = "PascalCase")]
#[serde(rename_all = "PascalCase")]
pub struct UserCredit {
    pub id: Option<i64>,
    pub username: String,
    pub user_id: String,
    pub credits: i32,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct UserCreditUpdateInfo {
    pub credit: i32,
    pub user_id: String,
}

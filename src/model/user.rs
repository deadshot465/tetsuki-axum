use gremlin_client::derive::{FromGMap, FromGValue};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, FromGValue, FromGMap, Deserialize, Serialize)]
pub struct User {
    pub username: String,
    pub user_id: String,
    pub pk: String,
}

#[derive(Deserialize, Serialize, Debug, Clone, FromGMap, FromGValue)]
pub struct UserCredit {
    pub username: String,
    pub user_id: String,
    pub credits: i64,
}

#[derive(Deserialize, Serialize, Debug, Clone, FromGMap, FromGValue)]
pub struct Credit {
    pub amount: i64,
    pub pk: String,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct CreditUpdateInfo {
    pub credit: i32,
    pub user_id: String,
}

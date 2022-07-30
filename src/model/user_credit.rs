use azure_data_cosmos::CosmosEntity;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, sqlx::FromRow, Clone, Debug, Default)]
pub struct UserCredit {
    pub id: String,
    pub username: String,
    pub user_id: String,
    pub credits: i32,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct UserCreditUpdateInfo {
    pub credit: i32,
}

#[derive(Copy, Clone, Debug)]
pub enum UserCreditUpdateOpt {
    Plus,
    Minus,
}

impl CosmosEntity for UserCredit {
    type Entity = String;

    fn partition_key(&self) -> Self::Entity {
        self.id.clone()
    }
}

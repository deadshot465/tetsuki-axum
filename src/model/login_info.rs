use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct LoginCredential {
    pub user_name: String,
    pub password: String,
}

#[derive(Deserialize, Serialize)]
pub struct LoginResponse {
    pub token: String,
    pub expiry: String,
}

use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Clone)]
pub struct Configuration {
    pub database_url: String,
    pub jwt_secret: String,
    pub bot_user_name: String,
    pub bot_user_pass: String,
    pub web_driver_address: String,
    pub server_bind_point: String,
    pub server_address: String,
}

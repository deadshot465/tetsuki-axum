use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct Configuration {
    pub database_url: String,
    pub jwt_secret: String,
    pub bot_user_name: String,
    pub bot_user_pass: String,
    pub web_driver_address: String,
    pub server_bind_point: String,
    pub server_address: String,
    pub dialog_quality: i32,
    pub log_level: String,
    pub cosmos_db_primary_key: String,
    pub cosmos_db_database_name: String,
    pub cosmos_db_account: String,
    pub swc_publication_endpoints: Vec<String>,
    pub swc_check_interval: i32,
}

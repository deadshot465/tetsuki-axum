use crate::model::configuration::Configuration;
use crate::shared::constants::CONFIG_DIRECTORY;
use once_cell::sync::Lazy;

pub static CONFIGURATION: Lazy<Configuration> =
    Lazy::new(|| initialize().expect("Failed to initialize configuration."));

const CONFIGURATION_FILE_NAME: &str = "/config.toml";

fn initialize() -> anyhow::Result<Configuration> {
    if !std::path::Path::new(CONFIG_DIRECTORY).exists() {
        std::fs::create_dir(CONFIG_DIRECTORY)?;
    }

    let configuration_path = String::from(CONFIG_DIRECTORY) + CONFIGURATION_FILE_NAME;
    if !std::path::Path::new(&configuration_path).exists() {
        // Read from environment variables
        let configuration = Configuration {
            database_url: std::env::var("DATABASE_URL")?,
            jwt_secret: std::env::var("JWT_SECRET")?,
            bot_user_name: std::env::var("BOT_USERNAME")?,
            bot_user_pass: std::env::var("BOT_USERPASS")?,
            web_driver_address: std::env::var("WEB_DRIVER_ADDRESS")?,
            server_bind_point: std::env::var("SERVER_BIND_POINT")?,
            server_address: std::env::var("SERVER_ADDRESS")?,
            dialog_quality: 75,
            log_level: "DEBUG".to_string(),
            cosmos_db_primary_key: std::env::var("COSMOS_DB_PRIMARY_KEY")?,
            cosmos_db_database_name: std::env::var("COSMOS_DB_DATABASE_NAME")?,
            cosmos_db_account: std::env::var("COSMOS_DB_ACCOUNT")?,
            swc_publication_endpoints: vec![],
            swc_check_interval: 3,
        };
        let serialized_toml = toml::to_string_pretty(&configuration)?;
        std::fs::write(&configuration_path, serialized_toml)?;
        Ok(configuration)
    } else {
        let toml = std::fs::read_to_string(&configuration_path)?;
        let deserialized_toml = toml::from_str::<Configuration>(&toml)?;
        Ok(deserialized_toml)
    }
}

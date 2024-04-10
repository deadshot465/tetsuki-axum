use once_cell::sync::Lazy;

pub mod configuration;
pub mod constants;
pub mod save_file;
pub mod swc_notifier;
pub mod swc_scraper;
pub mod util;
pub mod web_driver;

pub static HTTP_CLIENT: Lazy<reqwest::Client> = Lazy::new(reqwest::Client::new);

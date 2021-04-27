mod discord;
mod web_driver;
pub use discord::*;
use once_cell::sync::OnceCell;
pub use web_driver::*;

pub const JSON_HEADER: (&str, &str) = ("Content-Type", "application/json");

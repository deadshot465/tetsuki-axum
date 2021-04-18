mod discord;
mod web_driver;
pub use discord::*;
use once_cell::sync::OnceCell;
pub use web_driver::*;

//static HTTP_CLIENT: OnceCell<awc::Client> = OnceCell::new();

/*pub fn get_default_http_client() -> &'static awc::Client {
    HTTP_CLIENT.get_or_init(|| awc::Client::default())
}*/

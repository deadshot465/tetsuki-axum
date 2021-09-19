use crate::controller::credit_controller::config_credit_controller;
use crate::controller::dialog_controller::config_dialog_controller;
use crate::controller::login_controller::login;
use crate::db::initialize_db;
use crate::middleware::authentication::Authentication;
use crate::shared::configuration::CONFIGURATION;
use actix_web::{App, HttpServer};

mod controller;
mod db;
mod middleware;
mod model;
mod shared;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    if let Err(e) = dotenv::dotenv() {
        log::error!(".env file not found.");
    }

    let pool = initialize_db()
        .await
        .expect("Failed to initialize database connection.");

    HttpServer::new(move || {
        App::new()
            .wrap(Authentication)
            .app_data(pool.clone())
            .configure(config_credit_controller)
            .configure(config_dialog_controller)
            .service(login)
            .service(actix_files::Files::new("/asset/dialog", "./asset/dialog"))
    })
    .bind(&CONFIGURATION.server_bind_point)?
    .run()
    .await?;
    Ok(())
}

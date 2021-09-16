use crate::controller::credit_controller::config_credit_controller;
use crate::controller::dialog_controller::config_dialog_controller;
use crate::controller::login_controller::login;
use crate::controller::minigame::hangman::handle_hangman;
use crate::db::initialize_db;
use crate::middleware::authentication::Authentication;
use actix_web::{App, HttpServer};

mod controller;
mod db;
mod middleware;
mod model;
mod shared;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let pool = initialize_db()
        .await
        .expect("Failed to initialize database connection.");

    HttpServer::new(move || {
        App::new()
            .wrap(Authentication)
            .app_data(pool.clone())
            .configure(config_credit_controller)
            .configure(config_dialog_controller)
            .configure(config_minigame_controller)
            .service(login)
            .service(handle_hangman)
            .service(actix_files::Files::new("/asset/dialog", "./asset/dialog"))
    })
    .bind("0.0.0.0:8080")?
    .run()
    .await?;
    Ok(())
}

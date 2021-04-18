use crate::controller::{
    config_credit_controller, config_dialog_controller, config_minigame_controller, login,
};
use crate::db::initialize_db;
use crate::middleware::Authentication;
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
            .data(pool.clone())
            .configure(config_credit_controller)
            .configure(config_dialog_controller)
            .configure(config_minigame_controller)
            .service(login)
            .service(actix_files::Files::new("/asset/dialog", "./asset/dialog"))
    })
    .bind("0.0.0.0:8080")?
    .run()
    .await?;
    Ok(())
}

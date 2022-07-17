use crate::controller::credit_controller::config_credit_controller;
use crate::controller::dialog_controller::config_dialog_controller;
use crate::controller::login_controller::login;
use crate::controller::lottery_controller::config_lottery_controller;
use crate::controller::record_controller::config_record_controller;
use crate::db::initialize_db;
use crate::middleware::authentication::Authentication;
use crate::shared::configuration::CONFIGURATION;
use actix_web::web::Data;
use actix_web::{App, HttpServer};
use env_logger::Builder;
use log::LevelFilter;

mod controller;
mod db;
mod middleware;
mod model;
mod shared;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    if let Err(e) = dotenv::dotenv() {
        log::error!(".env file not found: {}", e);
    }

    let log_level = match CONFIGURATION.log_level.as_str() {
        "DEBUG" => LevelFilter::Debug,
        "INFO" => LevelFilter::Info,
        "WARN" => LevelFilter::Warn,
        "ERROR" => LevelFilter::Error,
        "TRACE" => LevelFilter::Trace,
        "OFF" => LevelFilter::Off,
        _ => LevelFilter::Debug,
    };

    Builder::new()
        .filter_level(log_level)
        .default_format()
        .init();

    HttpServer::new(move || {
        App::new()
            .wrap(Authentication)
            .app_data(Data::new(
                initialize_db().expect("Failed to initialize database connection."),
            ))
            .configure(config_credit_controller)
            .configure(config_dialog_controller)
            .configure(config_record_controller)
            .configure(config_lottery_controller)
            .service(login)
            .service(actix_files::Files::new("/asset/dialog", "./asset/dialog"))
    })
    .bind(&CONFIGURATION.server_bind_point)?
    .run()
    .await?;
    Ok(())
}

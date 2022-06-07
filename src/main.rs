use crate::controller::credit_controller::config_credit_controller;
use crate::controller::dialog_controller::config_dialog_controller;
use crate::controller::login_controller::login;
use crate::controller::record_controller::config_record_controller;
use crate::db::initialize_db;
use crate::middleware::authentication::Authentication;
use crate::shared::configuration::CONFIGURATION;
use actix_web::web::Data;
use actix_web::{App, HttpServer};
use env_logger::Builder;
use gremlin_client::aio::GremlinClient;
use gremlin_client::{ConnectionOptions, GraphSON};
use log::LevelFilter;
use std::sync::Arc;

mod controller;
mod db;
mod middleware;
mod model;
mod shared;

#[actix_web::main]
async fn main() -> anyhow::Result<()> {
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

    let username = format!(
        "/dbs/{}/colls/{}",
        &CONFIGURATION.cosmos_db_name, &CONFIGURATION.cosmos_db_container_name
    );

    let connection_options = ConnectionOptions::builder()
        .host(CONFIGURATION.cosmos_db_host.clone())
        .port(CONFIGURATION.cosmos_db_port)
        .credentials(&username, &CONFIGURATION.cosmos_db_primary_key)
        .ssl(true)
        .serializer(GraphSON::V2)
        .deserializer(GraphSON::V1)
        .build();

    let gremlin_client = GremlinClient::connect(connection_options)
        .await
        .map_err(|e| log::error!("Failed to connect to Azure Cosmos DB: {:?}", e))
        .map(Arc::new)
        .expect("Failed to connect to Azure Cosmos DB.");

    HttpServer::new(move || {
        App::new()
            .wrap(Authentication)
            .app_data(Data::new(
                initialize_db()
                    .map_err(|e| log::error!("Failed to initialize database connection: {}", e))
                    .expect("Failed to initialize database connection."),
            ))
            .app_data(Data::from(gremlin_client.clone()))
            .configure(config_credit_controller)
            .configure(config_dialog_controller)
            .configure(config_record_controller)
            .service(login)
            .service(actix_files::Files::new("/asset/dialog", "./asset/dialog"))
    })
    .bind(&CONFIGURATION.server_bind_point)?
    .run()
    .await?;

    Ok(())
}

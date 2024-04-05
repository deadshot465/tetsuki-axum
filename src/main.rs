use axum::routing::{get, get_service, patch, post};
use axum::Router;
use tower_http::services::ServeDir;
use tracing::Level;

use crate::controller::credit_controller::{
    add_credit, add_user, delete_user, get_all_user_credits, get_single_user_credits, reduce_credit,
};
use crate::controller::dialog_controller::{generate_dialog, get_dialog_options};
use crate::controller::login_controller::login;
use crate::controller::lottery_controller::{
    add_lottery, delete_lotteries, get_all_lotteries, get_daily_reward, get_user_lotteries,
    get_weekly_reward,
};
use crate::controller::mal_character_controller::{
    get_all_mal_characters, get_mal_character, post_mal_character,
};
use crate::controller::roll_controller::{
    get_all_rolls, get_all_user_rolls, get_user_roll_by_id, post_user_roll,
};
use crate::model::app_state::AppState;
use crate::shared::configuration::CONFIGURATION;
use crate::shared::swc_notifier::{
    initialize_slime_notification, initialize_tartarus_notification,
};
use crate::shared::swc_scraper::initialize_scraper;
use crate::shared::util::initialize_clients;

mod controller;
mod db;
mod middleware;
mod model;
mod shared;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    if let Err(e) = dotenv::dotenv() {
        eprintln!(".env file not found: {}", e);
    }

    let log_level = match CONFIGURATION.log_level.as_str() {
        "DEBUG" => Level::DEBUG,
        "INFO" => Level::INFO,
        "WARN" => Level::WARN,
        "ERROR" => Level::ERROR,
        "TRACE" => Level::TRACE,
        _ => Level::DEBUG,
    };

    let subscriber = tracing_subscriber::FmtSubscriber::builder()
        .with_max_level(log_level)
        .pretty()
        .finish();

    if let Err(e) = tracing::subscriber::set_global_default(subscriber) {
        eprintln!("Initializing tracing failed: {}", e);
    }

    tokio::spawn(async move {
        initialize_scraper().await;
    });

    tokio::spawn(async move {
        initialize_tartarus_notification().await;
    });

    tokio::spawn(async move {
        initialize_slime_notification().await;
    });

    let state = AppState {
        cosmos_db: initialize_clients(),
    };

    let app = Router::new()
        .route("/credit", get(get_all_user_credits).post(add_user))
        .route(
            "/credit/:user_id",
            get(get_single_user_credits).delete(delete_user),
        )
        .route("/credit/:user_id/plus", patch(add_credit))
        .route("/credit/:user_id/minus", patch(reduce_credit))
        .route("/dialog", get(get_dialog_options).post(generate_dialog))
        .route("/lottery", get(get_all_lotteries))
        .route(
            "/lottery/:user_id",
            get(get_user_lotteries).delete(delete_lotteries),
        )
        .route("/lottery/:user_id/daily", get(get_daily_reward))
        .route("/lottery/:user_id/weekly", get(get_weekly_reward))
        .route("/lottery/:user_id/new", post(add_lottery))
        .route(
            "/mal_character",
            get(get_all_mal_characters).post(post_mal_character),
        )
        .route("/mal_character/:id", get(get_mal_character))
        .route("/user_roll", get(get_all_rolls))
        .route("/user_roll/:user_id", get(get_all_user_rolls))
        .route("/user_roll/:user_id/new", post(post_user_roll))
        .route("/user_roll/:user_id/:roll_id", get(get_user_roll_by_id))
        .route("/login", post(login))
        .nest_service("/asset", get_service(ServeDir::new("./asset")))
        .nest_service("/upload", get_service(ServeDir::new("./upload")))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(&CONFIGURATION.server_bind_point).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

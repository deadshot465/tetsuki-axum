use crate::controller::mal_character_controller::{MAL_CHARACTERS, inner_get_all_mal_characters};
use crate::model::app_state::AppState;
use crate::model::claim::Claim;
use crate::model::cosmos_db::CosmosDb;
use crate::model::errors::ServerError;
use crate::model::user_roll::{GetRollResult, UserRoll};
use crate::shared::configuration::CONFIGURATION;
use crate::shared::util::{add_document, get_documents, query_document_within_container};
use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use azure_data_cosmos::{ContainerClient, Query};
use uuid::Uuid;

const USER_ROLLS: &str = "UserRolls";

pub async fn post_user_roll(
    _claim: Claim,
    _user_id: Path<String>,
    State(state): State<AppState>,
    Json(mut payload): Json<UserRoll>,
) -> Response {
    let cosmos_db = state.cosmos_db;
    if payload.id.is_empty() {
        payload.id = Uuid::new_v4().to_string();
    }

    let db_client = cosmos_db
        .client
        .database_client(&CONFIGURATION.cosmos_db_database_name);
    let item_id = payload.id.as_str();

    match add_document(&db_client, USER_ROLLS, item_id, payload.clone()).await {
        Ok(_) => (StatusCode::CREATED, Json(payload)).into_response(),
        Err(e) => {
            let error_message = format!("Failed to insert user roll into database: {}", e);
            tracing::error!("{}", &error_message);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ServerError::with_message(error_message)),
            )
                .into_response()
        }
    }
}

pub async fn get_all_rolls(_claim: Claim, State(state): State<AppState>) -> Response {
    let cosmos_db = state.cosmos_db;
    let db_client = cosmos_db
        .client
        .database_client(&CONFIGURATION.cosmos_db_database_name);
    let query_result = get_documents::<UserRoll, _>(&db_client, USER_ROLLS)
        .await
        .unwrap_or_default();
    (StatusCode::OK, Json(query_result)).into_response()
}

pub async fn get_all_user_rolls(
    _claim: Claim,
    Path(user_id): Path<String>,
    State(state): State<AppState>,
) -> Response {
    let cosmos_db = state.cosmos_db;
    let query_result = inner_get_all_user_rolls_with_names(user_id, cosmos_db).await;
    (StatusCode::OK, Json(query_result)).into_response()
}

pub async fn get_user_roll_by_id(
    _claim: Claim,
    Path(user_id): Path<String>,
    Path(roll_id): Path<i32>,
    State(state): State<AppState>,
) -> Response {
    let cosmos_db = state.cosmos_db;
    let all_user_rolls_with_names = inner_get_all_user_rolls_with_names(user_id, cosmos_db).await;
    let result = all_user_rolls_with_names
        .into_iter()
        .find(|res| res.user_roll.roll_id == roll_id);

    match result {
        Some(res) => (StatusCode::OK, Json(res)).into_response(),
        None => (
            StatusCode::NOT_FOUND,
            Json(ServerError::with_message(
                "Cannot find the specified roll within user's rolls.",
            )),
        )
            .into_response(),
    }
}

async fn inner_get_all_user_rolls_with_names(
    user_id: String,
    cosmos_db: CosmosDb,
) -> Vec<GetRollResult> {
    let db_client = cosmos_db
        .client
        .database_client(&CONFIGURATION.cosmos_db_database_name);
    let roll_container = db_client.container_client(USER_ROLLS, None).await;
    let mal_character_container = db_client.container_client(MAL_CHARACTERS, None).await;

    if let Err(e) = roll_container {
        let error_message = format!("Failed to get roll container: {}", e);
        tracing::error!("{}", &error_message);
        return Vec::new();
    }

    if let Err(e) = mal_character_container {
        let error_message = format!("Failed to get mal character container: {}", e);
        tracing::error!("{}", &error_message);
        return Vec::new();
    }

    let roll_container = roll_container.expect("Failed to get container.");
    let mal_character_container = mal_character_container.expect("Failed to get container.");

    let query_result = inner_get_all_user_rolls(&roll_container, user_id).await;
    let mal_characters = inner_get_all_mal_characters(&mal_character_container).await;
    query_result
        .into_iter()
        .map(|roll| GetRollResult {
            user_roll: roll.clone(),
            mal_character: mal_characters
                .iter()
                .find(|character| character.character_id == roll.mal_character_id)
                .cloned()
                .unwrap_or_default(),
        })
        .collect::<Vec<_>>()
}

async fn inner_get_all_user_rolls(container: &ContainerClient, user_id: String) -> Vec<UserRoll> {
    let query = Query::from("SELECT * FROM UserRolls u WHERE u.UserId = @user_id")
        .with_parameter("@user_id", user_id)
        .expect("Failed to build query.");

    query_document_within_container::<UserRoll, _>(container, query)
        .await
        .unwrap_or_default()
}

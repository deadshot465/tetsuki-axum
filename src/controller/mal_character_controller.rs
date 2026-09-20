use crate::model::app_state::AppState;
use crate::model::claim::Claim;
use crate::model::errors::ServerError;
use crate::model::mal_character::MalCharacter;
use crate::shared::configuration::CONFIGURATION;
use crate::shared::util::{add_document, query_document, query_document_within_container};
use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use azure_data_cosmos::ContainerClient;
use azure_data_cosmos::feed::Query;
use uuid::Uuid;

pub const MAL_CHARACTERS: &str = "MalCharacters";

pub async fn inner_get_all_mal_characters(container: &ContainerClient) -> Vec<MalCharacter> {
    let query = Query::from(format!("SELECT * FROM {} m", MAL_CHARACTERS));
    query_document_within_container::<MalCharacter, _>(container, query)
        .await
        .unwrap_or_default()
}

pub async fn get_all_mal_characters(_claim: Claim, State(state): State<AppState>) -> Response {
    let cosmos_db = state.cosmos_db;
    let db_client = cosmos_db
        .client
        .database_client(&CONFIGURATION.cosmos_db_database_name);
    match db_client.container_client(MAL_CHARACTERS, None).await {
        Err(e) => {
            let error_message = format!("Failed to list all mal characters: {}", e);
            tracing::error!("{}", &error_message);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ServerError::with_message(error_message)),
            )
                .into_response()
        }
        Ok(container) => {
            let query_result = inner_get_all_mal_characters(&container).await;
            (StatusCode::OK, Json(query_result)).into_response()
        }
    }
}

pub async fn get_mal_character(
    _claim: Claim,
    Path(id): Path<i32>,
    State(state): State<AppState>,
) -> Response {
    let cosmos_db = state.cosmos_db;
    let query = Query::from("SELECT * FROM MalCharacters m WHERE m.Id = @id")
        .with_parameter("@id", id)
        .expect("Failed to build query.");

    let db_client = cosmos_db
        .client
        .database_client(&CONFIGURATION.cosmos_db_database_name);

    let query_result = query_document::<MalCharacter, _, _>(&db_client, MAL_CHARACTERS, query)
        .await
        .and_then(|v| v.first().cloned());

    match query_result {
        None => (
            StatusCode::NOT_FOUND,
            Json(ServerError::with_message(
                "The specified mal character is not found.",
            )),
        )
            .into_response(),
        Some(mal_character) => (StatusCode::OK, Json(mal_character)).into_response(),
    }
}

pub async fn post_mal_character(
    _claim: Claim,
    State(state): State<AppState>,
    Json(mut payload): Json<MalCharacter>,
) -> Response {
    let cosmos_db = state.cosmos_db;
    if payload.id.is_empty() {
        payload.id = Uuid::new_v4().to_string()
    }

    let db_client = cosmos_db
        .client
        .database_client(&CONFIGURATION.cosmos_db_database_name);

    match add_document(&db_client, MAL_CHARACTERS, &payload.id, payload.clone()).await {
        Ok(_) => (StatusCode::CREATED, Json(payload)).into_response(),
        Err(e) => {
            let error_message = format!("Failed to insert mal character into database: {}", e);
            tracing::error!("{}", &error_message);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ServerError::with_message(error_message)),
            )
                .into_response()
        }
    }
}

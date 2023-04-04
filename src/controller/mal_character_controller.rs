use crate::model::app_state::AppState;
use crate::model::claim::Claim;
use crate::model::errors::ServerError;
use crate::model::mal_character::MalCharacter;
use crate::shared::util::{add_document, query_document, query_document_within_collection};
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use azure_data_cosmos::prelude::{CollectionClient, Param, Query};
use uuid::Uuid;

pub const MAL_CHARACTERS: &str = "MalCharacters";

pub async fn inner_get_all_mal_characters(collection: &CollectionClient) -> Vec<MalCharacter> {
    let query = Query::new(format!("SELECT * FROM {} m", MAL_CHARACTERS));
    query_document_within_collection::<MalCharacter, _>(collection, query, true)
        .await
        .unwrap_or_default()
}

pub async fn get_all_mal_characters(_claim: Claim, State(state): State<AppState>) -> Response {
    let cosmos_db = state.cosmos_db;
    let collection = cosmos_db.database.collection_client(MAL_CHARACTERS);
    let query_result = inner_get_all_mal_characters(&collection).await;
    (StatusCode::OK, Json(query_result)).into_response()
}

pub async fn get_mal_character(
    _claim: Claim,
    Path(id): Path<i32>,
    State(state): State<AppState>,
) -> Response {
    let cosmos_db = state.cosmos_db;
    let query = Query::with_params(
        "SELECT * FROM MalCharacters m WHERE m.Id = @id".into(),
        vec![Param::new("@id".into(), id)],
    );

    let query_result =
        query_document::<MalCharacter, _, _>(&cosmos_db.database, MAL_CHARACTERS, query, true)
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

    match add_document(&cosmos_db.database, MAL_CHARACTERS, payload.clone()).await {
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

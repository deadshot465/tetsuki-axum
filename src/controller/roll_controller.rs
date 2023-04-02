use crate::controller::mal_character_controller::{inner_get_all_mal_characters, MAL_CHARACTERS};
use crate::model::app_state::AppState;
use crate::model::cosmos_db::CosmosDb;
use crate::model::errors::ServerError;
use crate::model::user_roll::{GetRollResult, UserRoll};
use crate::shared::util::{add_document, get_documents, query_document_within_collection};
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use azure_data_cosmos::prelude::{CollectionClient, Param, Query};
use uuid::Uuid;

const USER_ROLLS: &str = "UserRolls";

pub async fn post_user_roll(
    _user_id: Path<String>,
    State(state): State<AppState>,
    Json(mut payload): Json<UserRoll>,
) -> Response {
    let cosmos_db = state.cosmos_db;
    if payload.id.is_empty() {
        payload.id = Uuid::new_v4().to_string();
    }

    match add_document(&cosmos_db.database, USER_ROLLS, payload.clone()).await {
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

pub async fn get_all_rolls(State(state): State<AppState>) -> Response {
    let cosmos_db = state.cosmos_db;
    let query_result = get_documents::<UserRoll, _>(&cosmos_db.database, USER_ROLLS)
        .await
        .unwrap_or_default();
    (StatusCode::OK, Json(query_result)).into_response()
}

pub async fn get_all_user_rolls(
    Path(user_id): Path<String>,
    State(state): State<AppState>,
) -> Response {
    let cosmos_db = state.cosmos_db;
    let query_result = inner_get_all_user_rolls_with_names(user_id, cosmos_db).await;
    (StatusCode::OK, Json(query_result)).into_response()
}

pub async fn get_user_roll_by_id(
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
    let roll_collection = cosmos_db.database.collection_client(USER_ROLLS);
    let mal_character_collection = cosmos_db.database.collection_client(MAL_CHARACTERS);
    let query_result = inner_get_all_user_rolls(&roll_collection, user_id).await;
    let mal_characters = inner_get_all_mal_characters(&mal_character_collection).await;
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

async fn inner_get_all_user_rolls(collection: &CollectionClient, user_id: String) -> Vec<UserRoll> {
    let query = Query::with_params(
        "SELECT * FROM UserRolls u WHERE u.UserId = @user_id".into(),
        vec![Param::new("@user_id".into(), user_id)],
    );

    query_document_within_collection::<UserRoll, _>(collection, query, true)
        .await
        .unwrap_or_default()
}

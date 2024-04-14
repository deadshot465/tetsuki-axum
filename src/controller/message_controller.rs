use crate::model::app_state::AppState;
use crate::model::claim::Claim;
use crate::model::errors::ServerError;
use crate::model::message::{
    GetCompletionRequest, GetCompletionResponse, GetMessageRequest, GetMessageResponse,
    MessageInfo, MessageRecord, MessageRecordSimple,
};
use crate::shared::util::{add_document_into_collection, query_document_within_collection};
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use azure_data_cosmos::prelude::{Param, Query};
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;
use uuid::Uuid;

const CHAT_COMPLETION_RECORDS: &str = "ChatCompletionRecords";
const CHAT_MESSAGE_RECORDS: &str = "ChatMessageRecords";

const MAX_CONTEXT_WORD_COUNT: usize = 1_000;

pub async fn post_completion_record(
    _claim: Claim,
    state: State<AppState>,
    payload: Json<MessageInfo>,
) -> Response {
    post_record(state, payload, CHAT_COMPLETION_RECORDS.into()).await
}

pub async fn post_message_record(
    _claim: Claim,
    state: State<AppState>,
    payload: Json<MessageInfo>,
) -> Response {
    post_record(state, payload, CHAT_MESSAGE_RECORDS.into()).await
}

pub async fn get_completion_records(
    _claim: Claim,
    State(state): State<AppState>,
    Json(payload): Json<GetCompletionRequest>,
) -> Response {
    let cosmos_db = state.cosmos_db;
    let completion_collection = cosmos_db
        .database
        .collection_client(CHAT_COMPLETION_RECORDS);

    let query = Query::with_params(
        format!(
            "SELECT * FROM {} c WHERE c.user_id = @user_id AND c.bot_id = @bot_id AND c.channel_id = @channel_id",
            CHAT_COMPLETION_RECORDS
        ),
        vec![
            Param::new("@user_id".into(), payload.user_id.clone()),
            Param::new("@bot_id".into(), payload.bot_id.clone()),
            Param::new("@channel_id".into(), payload.channel_id.clone()),
        ],
    );
    let completion_records =
        query_document_within_collection::<MessageRecord, _>(&completion_collection, query, true)
            .await;

    match completion_records {
        None => (
            StatusCode::OK,
            Json(GetCompletionResponse {
                bot_id: payload.bot_id,
                user_id: payload.user_id,
                messages: vec![],
            }),
        )
            .into_response(),
        Some(mut records) => {
            records.sort_by(|rec_1, rec_2| {
                let rec_1_post_at = OffsetDateTime::parse(&rec_1.post_at, &Rfc3339)
                    .unwrap_or(OffsetDateTime::UNIX_EPOCH);
                let rec_2_post_at = OffsetDateTime::parse(&rec_2.post_at, &Rfc3339)
                    .unwrap_or(OffsetDateTime::UNIX_EPOCH);
                rec_2_post_at.cmp(&rec_1_post_at)
            });
            let mut word_count = 0_usize;
            let mut messages = records
                .into_iter()
                .take_while(|rec| {
                    let char_count = rec.message.chars().count();
                    if word_count + char_count <= MAX_CONTEXT_WORD_COUNT {
                        word_count += char_count;
                        true
                    } else {
                        false
                    }
                })
                .map(|rec| rec.message)
                .collect::<Vec<_>>();
            messages.reverse();

            (
                StatusCode::OK,
                Json(GetCompletionResponse {
                    bot_id: payload.bot_id,
                    user_id: payload.user_id,
                    messages,
                }),
            )
                .into_response()
        }
    }
}

pub async fn get_message_records(
    _claim: Claim,
    State(state): State<AppState>,
    Json(payload): Json<GetMessageRequest>,
) -> Response {
    let cosmos_db = state.cosmos_db;
    let message_collection = cosmos_db.database.collection_client(CHAT_MESSAGE_RECORDS);

    let query = Query::with_params(
        format!(
            "SELECT * FROM {} c WHERE c.bot_id = @bot_id AND c.channel_id = @channel_id",
            CHAT_MESSAGE_RECORDS
        ),
        vec![
            Param::new("@bot_id".into(), payload.bot_id.clone()),
            Param::new("@channel_id".into(), payload.channel_id.clone()),
        ],
    );
    let message_records =
        query_document_within_collection::<MessageRecord, _>(&message_collection, query, true)
            .await;

    match message_records {
        None => (
            StatusCode::OK,
            Json(GetMessageResponse {
                bot_id: payload.bot_id,
                messages: vec![],
            }),
        )
            .into_response(),
        Some(mut records) => {
            records.sort_by(|rec_1, rec_2| {
                let rec_1_post_at = OffsetDateTime::parse(&rec_1.post_at, &Rfc3339)
                    .unwrap_or(OffsetDateTime::UNIX_EPOCH);
                let rec_2_post_at = OffsetDateTime::parse(&rec_2.post_at, &Rfc3339)
                    .unwrap_or(OffsetDateTime::UNIX_EPOCH);
                rec_2_post_at.cmp(&rec_1_post_at)
            });
            let mut word_count = 0_usize;
            let mut messages = records
                .into_iter()
                .take_while(|rec| {
                    let char_count = rec.message.chars().count();
                    if word_count + char_count <= MAX_CONTEXT_WORD_COUNT && rec.user_name.is_some()
                    {
                        word_count += char_count;
                        true
                    } else {
                        false
                    }
                })
                .map(|rec| MessageRecordSimple {
                    user_id: rec.user_id,
                    user_name: rec.user_name.unwrap_or_default(),
                    message: rec.message,
                })
                .collect::<Vec<_>>();
            messages.reverse();

            (
                StatusCode::OK,
                Json(GetMessageResponse {
                    bot_id: payload.bot_id,
                    messages,
                }),
            )
                .into_response()
        }
    }
}

async fn post_record(
    State(state): State<AppState>,
    Json(payload): Json<MessageInfo>,
    collection_name: String,
) -> Response {
    let cosmos_db = state.cosmos_db;
    let collection = cosmos_db.database.collection_client(collection_name);

    match validate_message_info(payload) {
        Ok(payload) => {
            let new_document = MessageRecord {
                id: Uuid::new_v4().to_string(),
                ..payload.into()
            };

            match add_document_into_collection(&collection, new_document).await {
                Ok(_) => StatusCode::CREATED.into_response(),
                Err(e) => {
                    let error_message = format!("Failed to add completion record: {}", e);
                    tracing::error!("{}", &error_message);
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(ServerError::with_message(error_message)),
                    )
                        .into_response()
                }
            }
        }
        Err(e) => (StatusCode::BAD_REQUEST, Json(e)).into_response(),
    }
}

fn validate_message_info(payload: MessageInfo) -> Result<MessageInfo, ServerError> {
    Ok(payload)
        .and_then(|p| {
            if p.bot_id.is_empty() {
                Err(ServerError::with_message("Bot id cannot be empty."))
            } else {
                Ok(p)
            }
        })
        .and_then(|p| {
            if p.user_id.is_empty() {
                Err(ServerError::with_message("User ID cannot be empty."))
            } else {
                Ok(p)
            }
        })
        .and_then(|p| {
            if let Some(ref name) = p.user_name {
                if name.is_empty() {
                    Err(ServerError::with_message("User name cannot be empty."))
                } else {
                    Ok(p)
                }
            } else {
                Ok(p)
            }
        })
        .and_then(|p| {
            if p.message.is_empty() {
                Err(ServerError::with_message("Message cannot be empty."))
            } else {
                Ok(p)
            }
        })
        .and_then(|p| {
            if p.channel_id.is_empty() {
                Err(ServerError::with_message("Channel ID cannot be empty."))
            } else {
                Ok(p)
            }
        })
        .and_then(|p| {
            if p.post_at.is_empty() {
                Err(ServerError::with_message("Post time cannot be empty."))
            } else {
                Ok(p)
            }
        })
}

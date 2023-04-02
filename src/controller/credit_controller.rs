use crate::model::app_state::AppState;
use crate::model::errors::ServerError;
use crate::model::user_credit::{UserCredit, UserCreditUpdateInfo, UserCreditUpdateOpt};
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use azure_data_cosmos::prelude::{Param, Query};

use crate::shared::util::{
    add_document, adjust_credit, get_documents, query_document, query_document_within_collection,
};

pub const USER_CREDITS: &str = "UserCredits";

pub async fn get_all_user_credits(State(state): State<AppState>) -> Response {
    let cosmos_db = state.cosmos_db;

    if let Some(credits) = get_documents::<UserCredit, _>(&cosmos_db.database, USER_CREDITS).await {
        (StatusCode::OK, Json(credits)).into_response()
    } else {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ServerError {
                error_message: "Failed to retrieve user credits.".to_string(),
            }),
        )
            .into_response()
    }
}

pub async fn get_single_user_credits(
    Path(user_id): Path<String>,
    State(state): State<AppState>,
) -> Response {
    let cosmos_db = state.cosmos_db;

    let query = Query::with_params(
        format!(
            "SELECT * FROM {} u WHERE u.user_id = @user_id",
            USER_CREDITS
        ),
        vec![Param::new("@user_id".into(), user_id)],
    );

    if let Some(query_result) =
        query_document::<UserCredit, _, _>(&cosmos_db.database, USER_CREDITS, query, true).await
    {
        (
            StatusCode::OK,
            Json(query_result.first().cloned().unwrap_or_default()),
        )
            .into_response()
    } else {
        (
            StatusCode::NOT_FOUND,
            Json(ServerError {
                error_message: "The specified user's credit info is not found.".into(),
            }),
        )
            .into_response()
    }
}

pub async fn add_user(
    State(state): State<AppState>,
    Json(user_credit): Json<UserCredit>,
) -> Response {
    if user_credit.username.is_empty() || user_credit.user_id.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(ServerError::with_message(
                "Either the user ID or the username is empty.",
            )),
        )
            .into_response();
    } else if user_credit.credits < 0 {
        return (
            StatusCode::BAD_REQUEST,
            Json(ServerError::with_message(
                "The amount of credits has to be greater than 0.",
            )),
        )
            .into_response();
    }

    let query = Query::with_params(
        format!(
            "SELECT * FROM {} u WHERE u.user_id = @user_id",
            USER_CREDITS
        ),
        vec![Param::new("@user_id".into(), user_credit.user_id.clone())],
    );

    let cosmos_db = state.cosmos_db;

    let query_result =
        query_document::<UserCredit, _, _>(&cosmos_db.database, USER_CREDITS, query, true).await;
    if query_result.is_some() {
        return (
            StatusCode::BAD_REQUEST,
            Json(ServerError::with_message(
                "Specified user already exists. Use PATCH to update user's information.",
            )),
        )
            .into_response();
    }

    match add_document(&cosmos_db.database, USER_CREDITS, user_credit.clone()).await {
        Ok(_) => (StatusCode::CREATED, Json(user_credit)).into_response(),
        Err(e) => {
            let error_message = format!("{}", e);
            tracing::error!("{}", &error_message);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ServerError::with_message(error_message)),
            )
                .into_response()
        }
    }
}

pub async fn add_credit(
    Path(user_id): Path<String>,
    State(state): State<AppState>,
    Json(user_credit): Json<UserCreditUpdateInfo>,
) -> Response {
    let cosmos_db = state.cosmos_db;
    adjust_credit(
        &cosmos_db.database,
        user_id,
        user_credit,
        UserCreditUpdateOpt::Plus,
    )
    .await
}

pub async fn reduce_credit(
    Path(user_id): Path<String>,
    State(state): State<AppState>,
    Json(user_credit): Json<UserCreditUpdateInfo>,
) -> Response {
    let cosmos_db = state.cosmos_db;
    adjust_credit(
        &cosmos_db.database,
        user_id,
        user_credit,
        UserCreditUpdateOpt::Minus,
    )
    .await
}

pub async fn delete_user(Path(user_id): Path<String>, State(state): State<AppState>) -> Response {
    let cosmos_db = state.cosmos_db;
    let collection = cosmos_db.database.collection_client(USER_CREDITS);

    let query = Query::with_params(
        format!(
            "SELECT * FROM {} u WHERE u.user_id = @user_id",
            USER_CREDITS
        ),
        vec![Param::new("@user_id".into(), user_id)],
    );
    let query_result = query_document_within_collection::<UserCredit, _>(&collection, query, true)
        .await
        .and_then(|result| result.first().cloned());

    if let Some(result) = query_result {
        let document = collection.document_client(result.id.clone(), &result.id);
        match document {
            Ok(doc) => match doc.delete_document().into_future().await {
                Ok(_) => StatusCode::NO_CONTENT.into_response(),
                Err(e) => {
                    let error_message = format!("Failed to delete user credit: {}", e);
                    tracing::error!("{}", &error_message);
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(ServerError::with_message(error_message)),
                    )
                        .into_response()
                }
            },
            Err(e) => {
                let error_message = format!("{}", e);
                tracing::error!("{}", &error_message);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ServerError::with_message(error_message)),
                )
                    .into_response()
            }
        }
    } else {
        (
            StatusCode::NOT_FOUND,
            Json(ServerError::with_message(
                "The specified user doesn't exist.",
            )),
        )
            .into_response()
    }
}

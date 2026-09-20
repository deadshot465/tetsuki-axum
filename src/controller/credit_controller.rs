use crate::model::app_state::AppState;
use crate::model::claim::Claim;
use crate::model::errors::ServerError;
use crate::model::user_credit::{UserCredit, UserCreditUpdateInfo, UserCreditUpdateOpt};
use crate::shared::configuration::CONFIGURATION;
use crate::shared::util::{
    add_document, adjust_credit, get_documents, query_document, query_document_within_container,
};
use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use azure_data_cosmos::feed::Query;

pub const USER_CREDITS: &str = "UserCredits";

pub async fn get_all_user_credits(_claim: Claim, State(state): State<AppState>) -> Response {
    let cosmos_db = state.cosmos_db;
    let db_client = cosmos_db
        .client
        .database_client(&CONFIGURATION.cosmos_db_database_name);

    if let Some(credits) = get_documents::<UserCredit, _>(&db_client, USER_CREDITS).await {
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
    _claim: Claim,
    Path(user_id): Path<String>,
    State(state): State<AppState>,
) -> Response {
    let cosmos_db = state.cosmos_db;

    let db_client = cosmos_db
        .client
        .database_client(&CONFIGURATION.cosmos_db_database_name);

    let query = Query::from(format!(
        "SELECT * FROM {} u WHERE u.user_id = @user_id",
        USER_CREDITS
    ))
    .with_parameter("@user_id", user_id)
    .expect("Failed to build query.");

    if let Some(query_result) =
        query_document::<UserCredit, _, _>(&db_client, USER_CREDITS, query).await
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
    _claim: Claim,
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

    let query = Query::from(format!(
        "SELECT * FROM {} u WHERE u.user_id = @user_id",
        USER_CREDITS
    ))
    .with_parameter("@user_id", user_credit.user_id.clone())
    .expect("Failed to build query.");

    let cosmos_db = state.cosmos_db;
    let db_client = cosmos_db
        .client
        .database_client(&CONFIGURATION.cosmos_db_database_name);

    let query_result = query_document::<UserCredit, _, _>(&db_client, USER_CREDITS, query).await;
    if query_result.is_some() {
        return (
            StatusCode::BAD_REQUEST,
            Json(ServerError::with_message(
                "Specified user already exists. Use PATCH to update user's information.",
            )),
        )
            .into_response();
    }

    let item_id = user_credit.id.as_str();

    match add_document(&db_client, USER_CREDITS, item_id, user_credit.clone()).await {
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
    _claim: Claim,
    Path(user_id): Path<String>,
    State(state): State<AppState>,
    Json(user_credit): Json<UserCreditUpdateInfo>,
) -> Response {
    let cosmos_db = state.cosmos_db;
    let db_client = cosmos_db
        .client
        .database_client(&CONFIGURATION.cosmos_db_database_name);
    adjust_credit(&db_client, user_id, user_credit, UserCreditUpdateOpt::Plus).await
}

pub async fn reduce_credit(
    _claim: Claim,
    Path(user_id): Path<String>,
    State(state): State<AppState>,
    Json(user_credit): Json<UserCreditUpdateInfo>,
) -> Response {
    let cosmos_db = state.cosmos_db;
    let db_client = cosmos_db
        .client
        .database_client(&CONFIGURATION.cosmos_db_database_name);
    adjust_credit(&db_client, user_id, user_credit, UserCreditUpdateOpt::Minus).await
}

pub async fn delete_user(
    _claim: Claim,
    Path(user_id): Path<String>,
    State(state): State<AppState>,
) -> Response {
    let cosmos_db = state.cosmos_db;
    let db_client = cosmos_db
        .client
        .database_client(&CONFIGURATION.cosmos_db_database_name);
    let container = db_client.container_client(USER_CREDITS, None).await;

    if let Err(e) = container {
        let error_message = format!("Failed to delete user: {}", e);
        tracing::error!("{}", &error_message);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ServerError::with_message(error_message)),
        )
            .into_response();
    }

    let container = container.expect("Failed to get container.");

    let query = Query::from(format!(
        "SELECT * FROM {} u WHERE u.user_id = @user_id",
        USER_CREDITS
    ))
    .with_parameter("@user_id", user_id)
    .expect("Failed to build query.");

    let query_result = query_document_within_container::<UserCredit, _>(&container, query)
        .await
        .and_then(|result| result.first().cloned());

    if let Some(result) = query_result {
        match container.delete_item("id", &result.id, None).await {
            Err(e) => {
                let error_message = format!("Failed to delete user: {}", e);
                tracing::error!("{}", &error_message);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ServerError::with_message(error_message)),
                )
                    .into_response()
            }
            Ok(_) => StatusCode::NO_CONTENT.into_response(),
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

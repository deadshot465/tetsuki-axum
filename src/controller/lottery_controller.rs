use crate::controller::credit_controller::USER_CREDITS;
use crate::model::app_state::AppState;
use crate::model::claim::Claim;
use crate::model::cosmos_db::CosmosDb;
use crate::model::errors::ServerError;
use crate::model::lottery::{UserLottery, UserLotteryUpdateInfo};
use crate::model::user_credit::{UserCredit, UserCreditUpdateInfo, UserCreditUpdateOpt};
use crate::shared::configuration::CONFIGURATION;
use crate::shared::util::{
    add_document, add_document_into_container, adjust_credit, adjust_credit_in_collection,
    get_documents, query_document, query_document_within_container,
};
use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use azure_data_cosmos::feed::Query;
use std::ops::Add;
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;
use uuid::Uuid;

const USER_LOTTERIES: &str = "UserLotteries";

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
enum RewardType {
    Daily,
    Weekly,
}

pub async fn get_daily_reward(
    _claim: Claim,
    Path(user_id): Path<String>,
    State(state): State<AppState>,
) -> Response {
    let cosmos_db = state.cosmos_db;
    get_reward(user_id, RewardType::Daily, cosmos_db).await
}

pub async fn get_weekly_reward(
    _claim: Claim,
    Path(user_id): Path<String>,
    State(state): State<AppState>,
) -> Response {
    let cosmos_db = state.cosmos_db;
    get_reward(user_id, RewardType::Weekly, cosmos_db).await
}

pub async fn get_all_lotteries(_claim: Claim, State(state): State<AppState>) -> Response {
    let cosmos_db = state.cosmos_db;
    let db_client = cosmos_db
        .client
        .database_client(&CONFIGURATION.cosmos_db_database_name);

    if let Some(lotteries) = get_documents::<UserLottery, _>(&db_client, USER_LOTTERIES).await {
        (StatusCode::OK, Json(lotteries)).into_response()
    } else {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ServerError::with_message(
                "Failed to retrieve all users' lotteries.",
            )),
        )
            .into_response()
    }
}

pub async fn get_user_lotteries(
    _claim: Claim,
    Path(user_id): Path<String>,
    State(state): State<AppState>,
) -> Response {
    let cosmos_db = state.cosmos_db;
    let query = Query::from(format!(
        "SELECT * FROM {} u WHERE u.user_id = @user_id",
        USER_LOTTERIES
    ))
    .with_parameter("@user_id", user_id)
    .expect("Failed to build query.");

    let db_client = cosmos_db
        .client
        .database_client(&CONFIGURATION.cosmos_db_database_name);

    if let Some(query_result) =
        query_document::<UserLottery, _, _>(&db_client, USER_LOTTERIES, query).await
    {
        let user_lottery = query_result.first().cloned().unwrap_or_default();
        (StatusCode::OK, Json(user_lottery)).into_response()
    } else {
        (
            StatusCode::NOT_FOUND,
            Json(ServerError::with_message(
                "The corresponding user's lottery info is not found.",
            )),
        )
            .into_response()
    }
}

pub async fn add_lottery(
    _claim: Claim,
    Path(user_id): Path<String>,
    State(state): State<AppState>,
    Json(mut payload): Json<UserLotteryUpdateInfo>,
) -> Response {
    let cosmos_db = state.cosmos_db;

    let db_client = cosmos_db
        .client
        .database_client(&CONFIGURATION.cosmos_db_database_name);

    if payload
        .lotteries
        .iter()
        .any(|lottery| lottery.len() != 6 || lottery.iter().any(|n| *n < 1 || *n > 49))
    {
        return (
            StatusCode::BAD_REQUEST,
            Json(ServerError::with_message(
                "Each lottery has to contain exactly 6 numbers, each of which is between 1 and 49.",
            )),
        )
            .into_response();
    }

    let lottery_count = payload.lotteries.len();

    let lottery_container = db_client.container_client(USER_LOTTERIES, None).await;

    if let Err(e) = lottery_container {
        let error_message = format!("Failed to get lottery container: {}", e);
        tracing::error!("{}", &error_message);
        return (StatusCode::INTERNAL_SERVER_ERROR, error_message).into_response();
    }

    let lottery_container = lottery_container.expect("Failed to get lottery container.");

    let credit_container = db_client.container_client(USER_CREDITS, None).await;

    if let Err(e) = credit_container {
        let error_message = format!("Failed to get credit container: {}", e);
        tracing::error!("{}", &error_message);
        return (StatusCode::INTERNAL_SERVER_ERROR, error_message).into_response();
    }

    let credit_container = credit_container.expect("Failed to get credit container.");

    let query = Query::from(format!(
        "SELECT * FROM {} u WHERE u.user_id = @user_id",
        USER_CREDITS
    ))
    .with_parameter("@user_id", user_id.clone())
    .expect("Failed to build query.");

    let user_credit = query_document_within_container::<UserCredit, _>(&credit_container, query)
        .await
        .and_then(|res| res.first().cloned());

    match user_credit {
        None => {
            return (
                StatusCode::NOT_FOUND,
                Json(ServerError::with_message(
                    "The specified user's credit info is not found.",
                )),
            )
                .into_response();
        }
        Some(credit) => {
            if credit.credits - (10 * lottery_count as i32) < 0 {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(ServerError::with_message(
                        "The specified user doesn't have enough credits.",
                    )),
                )
                    .into_response();
            }

            if credit.username.as_str() != payload.username.as_str() {
                let new_document = UserCredit {
                    username: payload.username.clone(),
                    ..credit
                };

                let item_id = new_document.id.clone();

                if let Err(e) =
                    add_document_into_container(&credit_container, &item_id, new_document).await
                {
                    tracing::error!(
                        "Failed to update user's name during lottery purchase: {}",
                        e
                    );
                }
            }
        }
    }

    let query = Query::from(format!(
        "SELECT * FROM {} u WHERE u.user_id = @user_id",
        USER_LOTTERIES
    ))
    .with_parameter("@user_id", user_id.clone())
    .expect("Failed to build query.");

    for lottery in payload.lotteries.iter_mut() {
        lottery.sort_unstable();
    }

    match query_document_within_container::<UserLottery, _>(&lottery_container, query).await {
        None => {
            let new_document = UserLottery {
                id: Uuid::new_v4().to_string(),
                user_id: user_id.clone(),
                next_daily_time: OffsetDateTime::now_utc()
                    .add(time::Duration::days(1))
                    .format(&Rfc3339)
                    .unwrap_or_default(),
                next_weekly_time: OffsetDateTime::now_utc()
                    .add(time::Duration::days(7))
                    .format(&Rfc3339)
                    .unwrap_or_default(),
                lotteries: payload.lotteries,
            };

            let item_id = new_document.id.as_str();

            match add_document_into_container(&lottery_container, item_id, new_document.clone())
                .await
            {
                Ok(_) => {
                    adjust_credit_in_collection(
                        &credit_container,
                        user_id.clone(),
                        UserCreditUpdateInfo {
                            credit: (10 * lottery_count) as i32,
                        },
                        UserCreditUpdateOpt::Minus,
                    )
                    .await;
                    (StatusCode::CREATED, Json(new_document)).into_response()
                }
                Err(e) => {
                    let error_message = format!("Failed to add a new lottery: {}", e);
                    tracing::error!("{}", &error_message);
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(ServerError::with_message(error_message)),
                    )
                        .into_response()
                }
            }
        }
        Some(res) => {
            let mut user_lottery = res.first().cloned().unwrap_or_default();
            user_lottery.lotteries.append(&mut payload.lotteries);
            let new_document = UserLottery {
                lotteries: user_lottery.lotteries,
                ..user_lottery
            };

            let item_id = new_document.id.as_str();

            match add_document_into_container(&lottery_container, item_id, new_document.clone())
                .await
            {
                Ok(_) => {
                    adjust_credit_in_collection(
                        &credit_container,
                        user_id.clone(),
                        UserCreditUpdateInfo {
                            credit: (10 * lottery_count) as i32,
                        },
                        UserCreditUpdateOpt::Minus,
                    )
                    .await;
                    (StatusCode::CREATED, Json(new_document)).into_response()
                }
                Err(e) => {
                    let error_message = format!("Failed to add a new lottery: {}", e);
                    tracing::error!("{}", &error_message);
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(ServerError::with_message(error_message)),
                    )
                        .into_response()
                }
            }
        }
    }
}

pub async fn delete_lotteries(
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
        USER_LOTTERIES
    ))
    .with_parameter("@user_id", user_id.clone())
    .expect("Failed to build query.");

    let query_result = query_document::<UserLottery, _, _>(&db_client, USER_LOTTERIES, query)
        .await
        .and_then(|v| v.first().cloned());

    match query_result {
        None => (
            StatusCode::NOT_FOUND,
            Json(ServerError::with_message(
                "The specified user is not found.",
            )),
        )
            .into_response(),
        Some(user_lottery) => {
            let new_document = UserLottery {
                lotteries: vec![],
                ..user_lottery
            };

            let item_id = new_document.id.clone();

            match add_document(&db_client, USER_LOTTERIES, &item_id, new_document).await {
                Ok(_) => StatusCode::NO_CONTENT.into_response(),
                Err(e) => {
                    let error_message = format!(
                        "Failed to remove all lotteries from the user {}: {}",
                        user_id, e
                    );
                    tracing::error!("{}", &error_message);
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(ServerError::with_message(error_message)),
                    )
                        .into_response()
                }
            }
        }
    }
}

async fn get_reward(user_id: String, reward_type: RewardType, cosmos_db: CosmosDb) -> Response {
    let query = Query::from(format!(
        "SELECT * FROM {} u WHERE u.user_id = @user_id",
        USER_LOTTERIES
    ))
    .with_parameter("@user_id", user_id.clone())
    .expect("Failed to build query.");

    let db_client = cosmos_db
        .client
        .database_client(&CONFIGURATION.cosmos_db_database_name);

    match query_document::<UserLottery, _, _>(&db_client, "UserLotteries", query).await {
        None => (
            StatusCode::NOT_FOUND,
            Json(ServerError::with_message(
                "The specified user is not found.",
            )),
        )
            .into_response(),
        Some(result) => {
            let user_lottery = result.first().cloned().unwrap_or_default();
            let next_reward_time = match reward_type {
                RewardType::Daily => user_lottery.next_daily_time.clone(),
                RewardType::Weekly => user_lottery.next_weekly_time.clone(),
            };
            if OffsetDateTime::now_utc()
                > OffsetDateTime::parse(&next_reward_time, &Rfc3339)
                    .unwrap_or(OffsetDateTime::UNIX_EPOCH)
            {
                let response = update_credits(&user_lottery, reward_type, &cosmos_db).await;

                let new_document = UserLottery {
                    next_daily_time: if reward_type == RewardType::Daily {
                        OffsetDateTime::now_utc()
                            .add(time::Duration::days(1))
                            .format(&Rfc3339)
                            .unwrap_or_default()
                    } else {
                        user_lottery.next_daily_time.clone()
                    },
                    next_weekly_time: if reward_type == RewardType::Weekly {
                        OffsetDateTime::now_utc()
                            .add(time::Duration::days(7))
                            .format(&Rfc3339)
                            .unwrap_or_default()
                    } else {
                        user_lottery.next_weekly_time.clone()
                    },
                    ..user_lottery
                };

                let item_id = new_document.id.clone();

                match add_document(&db_client, USER_LOTTERIES, &item_id, new_document).await {
                    Ok(_) => response,
                    Err(e) => {
                        let error_message = format!("Failed to update next reward time: {}", e);
                        tracing::error!("{}", &error_message);
                        (
                            StatusCode::INTERNAL_SERVER_ERROR,
                            Json(ServerError::with_message(error_message)),
                        )
                            .into_response()
                    }
                }
            } else {
                (StatusCode::ACCEPTED, Json(user_lottery)).into_response()
            }
        }
    }
}

async fn update_credits(
    user_lottery: &UserLottery,
    reward_type: RewardType,
    cosmos_db: &CosmosDb,
) -> Response {
    let db_client = cosmos_db
        .client
        .database_client(&CONFIGURATION.cosmos_db_database_name);
    adjust_credit(
        &db_client,
        user_lottery.user_id.clone(),
        UserCreditUpdateInfo {
            credit: match reward_type {
                RewardType::Daily => 10,
                RewardType::Weekly => 70,
            },
        },
        UserCreditUpdateOpt::Plus,
    )
    .await
}

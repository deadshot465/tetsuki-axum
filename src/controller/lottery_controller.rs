use crate::controller::credit_controller::USER_CREDITS;
use crate::model::errors::ServerError;
use crate::model::lottery::{UserLottery, UserLotteryUpdateInfo};
use crate::model::user_credit::{UserCredit, UserCreditUpdateInfo, UserCreditUpdateOpt};
use crate::shared::util::{add_document, adjust_credit, get_documents, query_document};
use actix_web::web::{Path, ServiceConfig};
use actix_web::{get, post, HttpResponse, Responder};
use azure_core::error::Error;
use azure_data_cosmos::operations::CreateDocumentResponse;
use azure_data_cosmos::prelude::{Param, Query};
use std::ops::Add;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;
use uuid::Uuid;

const USER_LOTTERIES: &str = "UserLotteries";

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
enum RewardType {
    Daily,
    Weekly,
}

pub fn config_lottery_controller(cfg: &mut ServiceConfig) {
    cfg.service(get_daily_reward)
        .service(get_weekly_reward)
        .service(get_all_lotteries)
        .service(get_user_lotteries)
        .service(add_lottery);
}

#[get("/lottery/{user_id}/daily")]
async fn get_daily_reward(user_id: Path<String>) -> impl Responder {
    get_reward(user_id, RewardType::Daily).await
}

#[get("/lottery/{user_id}/weekly")]
async fn get_weekly_reward(user_id: Path<String>) -> impl Responder {
    get_reward(user_id, RewardType::Weekly).await
}

#[get("/lottery")]
async fn get_all_lotteries() -> impl Responder {
    if let Some(lotteries) = get_documents::<UserLottery, _>(USER_LOTTERIES).await {
        HttpResponse::Ok().json(lotteries)
    } else {
        HttpResponse::InternalServerError().json(ServerError::with_message(
            "Failed to retrieve all users' lotteries.",
        ))
    }
}

#[get("/lottery/{user_id}")]
async fn get_user_lotteries(user_id: Path<String>) -> impl Responder {
    let query = Query::with_params(
        format!(
            "SELECT * FROM {} u WHERE u.user_id = @user_id",
            USER_LOTTERIES
        ),
        vec![Param::new("@user_id".into(), user_id.into_inner())],
    );

    if let Some(query_result) = query_document::<UserLottery, _, _>(USER_LOTTERIES, query).await {
        let user_lottery = query_result.first().cloned().unwrap_or_default();
        HttpResponse::Ok().json(user_lottery)
    } else {
        HttpResponse::NotFound().json(ServerError::with_message(
            "The corresponding user's lottery info is not found.",
        ))
    }
}

#[post("/lottery/{user_id}/new")]
async fn add_lottery(
    user_id: Path<String>,
    payload: actix_web::web::Json<UserLotteryUpdateInfo>,
) -> impl Responder {
    if payload
        .lotteries
        .iter()
        .any(|lottery| lottery.len() != 6 || lottery.iter().any(|n| *n < 1 || *n > 49))
    {
        return HttpResponse::BadRequest().json(ServerError::with_message(
            "Each lottery has to contain exactly 6 numbers, each of which is between 1 and 49.",
        ));
    }

    let user_id = user_id.into_inner();
    let lottery_count = payload.lotteries.len();

    let query = Query::with_params(
        format!(
            "SELECT * FROM {} u WHERE u.user_id = @user_id",
            USER_CREDITS
        ),
        vec![Param::new("@user_id".into(), user_id.clone())],
    );

    let user_credit = query_document::<UserCredit, _, _>(USER_CREDITS, query)
        .await
        .and_then(|res| res.first().cloned());

    match user_credit {
        None => {
            return HttpResponse::NotFound().json(ServerError::with_message(
                "The specified user's credit info is not found.",
            ));
        }
        Some(credit) => {
            if credit.credits - (10 * lottery_count as i32) < 0 {
                return HttpResponse::BadRequest().json(ServerError::with_message(
                    "The specified user doesn't have enough credits.",
                ));
            }
        }
    }

    let query = Query::with_params(
        format!(
            "SELECT * FROM {} u WHERE u.user_id = @user_id",
            USER_LOTTERIES
        ),
        vec![Param::new("@user_id".into(), user_id.clone())],
    );

    let mut payload = payload.into_inner();

    match query_document::<UserLottery, _, _>(USER_LOTTERIES, query).await {
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

            match add_document(USER_LOTTERIES, new_document.clone()).await {
                Ok(_) => {
                    adjust_credit(
                        user_id.clone(),
                        UserCreditUpdateInfo {
                            credit: (10 * lottery_count) as i32,
                            user_id,
                        },
                        UserCreditUpdateOpt::Minus,
                    )
                    .await;
                    HttpResponse::Created().json(new_document)
                }
                Err(e) => {
                    let error_message = format!("Failed to add a new lottery: {}", e);
                    log::error!("{}", &error_message);
                    HttpResponse::InternalServerError()
                        .json(ServerError::with_message(error_message))
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

            match add_document(USER_LOTTERIES, new_document.clone()).await {
                Ok(_) => {
                    adjust_credit(
                        user_id.clone(),
                        UserCreditUpdateInfo {
                            credit: (10 * lottery_count) as i32,
                            user_id,
                        },
                        UserCreditUpdateOpt::Minus,
                    )
                    .await;
                    HttpResponse::Created().json(new_document)
                }
                Err(e) => {
                    let error_message = format!("Failed to add a new lottery: {}", e);
                    log::error!("{}", &error_message);
                    HttpResponse::InternalServerError()
                        .json(ServerError::with_message(error_message))
                }
            }
        }
    }
}

async fn get_reward(user_id: Path<String>, reward_type: RewardType) -> impl Responder {
    let query = Query::with_params(
        format!(
            "SELECT * FROM {} u WHERE u.user_id = @user_id",
            USER_LOTTERIES
        ),
        vec![Param::new("@user_id".into(), user_id.into_inner())],
    );

    match query_document::<UserLottery, _, _>("UserLotteries", query).await {
        None => HttpResponse::NotFound().json(ServerError::with_message(
            "The specified user is not found.",
        )),
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
                let response = update_credits(&user_lottery, reward_type).await;

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

                match add_document(USER_LOTTERIES, new_document).await {
                    Ok(_) => response,
                    Err(e) => {
                        let error_message = format!("Failed to update next reward time: {}", e);
                        log::error!("{}", &error_message);
                        HttpResponse::InternalServerError()
                            .json(ServerError::with_message(error_message))
                    }
                }
            } else {
                HttpResponse::NoContent().finish()
            }
        }
    }
}

async fn update_credits(user_lottery: &UserLottery, reward_type: RewardType) -> HttpResponse {
    adjust_credit(
        user_lottery.user_id.clone(),
        UserCreditUpdateInfo {
            credit: match reward_type {
                RewardType::Daily => 10,
                RewardType::Weekly => 70,
            },
            user_id: user_lottery.user_id.clone(),
        },
        UserCreditUpdateOpt::Plus,
    )
    .await
}

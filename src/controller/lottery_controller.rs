use crate::controller::credit_controller::USER_CREDITS;
use crate::model::cosmos_db::CosmosDb;
use crate::model::errors::ServerError;
use crate::model::lottery::{UserLottery, UserLotteryUpdateInfo};
use crate::model::user_credit::{UserCredit, UserCreditUpdateInfo, UserCreditUpdateOpt};
use crate::shared::util::{
    add_document, add_document_into_collection, adjust_credit, adjust_credit_in_collection,
    get_documents, query_document, query_document_within_collection,
};
use actix_web::web::{Data, Path, ServiceConfig};
use actix_web::{delete, get, post, HttpResponse, Responder};
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
        .service(add_lottery)
        .service(delete_lotteries);
}

#[get("/lottery/{user_id}/daily")]
async fn get_daily_reward(user_id: Path<String>, cosmos_db: Data<CosmosDb>) -> impl Responder {
    get_reward(user_id, RewardType::Daily, cosmos_db).await
}

#[get("/lottery/{user_id}/weekly")]
async fn get_weekly_reward(user_id: Path<String>, cosmos_db: Data<CosmosDb>) -> impl Responder {
    get_reward(user_id, RewardType::Weekly, cosmos_db).await
}

#[get("/lottery")]
async fn get_all_lotteries(cosmos_db: Data<CosmosDb>) -> impl Responder {
    if let Some(lotteries) =
        get_documents::<UserLottery, _>(&cosmos_db.database, USER_LOTTERIES).await
    {
        HttpResponse::Ok().json(lotteries)
    } else {
        HttpResponse::InternalServerError().json(ServerError::with_message(
            "Failed to retrieve all users' lotteries.",
        ))
    }
}

#[get("/lottery/{user_id}")]
async fn get_user_lotteries(user_id: Path<String>, cosmos_db: Data<CosmosDb>) -> impl Responder {
    let query = Query::with_params(
        format!(
            "SELECT * FROM {} u WHERE u.user_id = @user_id",
            USER_LOTTERIES
        ),
        vec![Param::new("@user_id".into(), user_id.into_inner())],
    );

    if let Some(query_result) =
        query_document::<UserLottery, _, _>(&cosmos_db.database, USER_LOTTERIES, query, true).await
    {
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
    cosmos_db: Data<CosmosDb>,
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
    let lottery_collection = cosmos_db.database.collection_client(USER_LOTTERIES);
    let credit_collection = cosmos_db.database.collection_client(USER_CREDITS);

    let query = Query::with_params(
        format!(
            "SELECT * FROM {} u WHERE u.user_id = @user_id",
            USER_CREDITS
        ),
        vec![Param::new("@user_id".into(), user_id.clone())],
    );

    let user_credit =
        query_document_within_collection::<UserCredit, _>(&credit_collection, query, true)
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

            if credit.username.as_str() != payload.username.as_str() {
                let new_document = UserCredit {
                    username: payload.username.clone(),
                    ..credit
                };

                if let Err(e) = add_document_into_collection(&credit_collection, new_document).await
                {
                    log::error!(
                        "Failed to update user's name during lottery purchase: {}",
                        e
                    );
                }
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
    for lottery in payload.lotteries.iter_mut() {
        lottery.sort_unstable();
    }

    match query_document_within_collection::<UserLottery, _>(&lottery_collection, query, true).await
    {
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

            match add_document_into_collection(&lottery_collection, new_document.clone()).await {
                Ok(_) => {
                    adjust_credit_in_collection(
                        &credit_collection,
                        user_id.clone(),
                        UserCreditUpdateInfo {
                            credit: (10 * lottery_count) as i32,
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

            match add_document_into_collection(&lottery_collection, new_document.clone()).await {
                Ok(_) => {
                    adjust_credit_in_collection(
                        &credit_collection,
                        user_id.clone(),
                        UserCreditUpdateInfo {
                            credit: (10 * lottery_count) as i32,
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

#[delete("/lottery/{user_id}")]
async fn delete_lotteries(user_id: Path<String>, cosmos_db: Data<CosmosDb>) -> impl Responder {
    let user_id = user_id.into_inner();
    let query = Query::with_params(
        format!(
            "SELECT * FROM {} u WHERE u.user_id = @user_id",
            USER_LOTTERIES
        ),
        vec![Param::new("@user_id".into(), user_id.clone())],
    );

    let query_result =
        query_document::<UserLottery, _, _>(&cosmos_db.database, USER_LOTTERIES, query, true)
            .await
            .and_then(|v| v.first().cloned());

    match query_result {
        None => HttpResponse::NotFound().json(ServerError::with_message(
            "The specified user is not found.",
        )),
        Some(user_lottery) => {
            let new_document = UserLottery {
                lotteries: vec![],
                ..user_lottery
            };

            match add_document(&cosmos_db.database, USER_LOTTERIES, new_document).await {
                Ok(_) => HttpResponse::NoContent().finish(),
                Err(e) => {
                    let error_message = format!(
                        "Failed to remove all lotteries from the user {}: {}",
                        user_id, e
                    );
                    log::error!("{}", &error_message);
                    HttpResponse::InternalServerError()
                        .json(ServerError::with_message(error_message))
                }
            }
        }
    }
}

async fn get_reward(
    user_id: Path<String>,
    reward_type: RewardType,
    cosmos_db: Data<CosmosDb>,
) -> impl Responder {
    let user_id = user_id.into_inner();
    let query = Query::with_params(
        format!(
            "SELECT * FROM {} u WHERE u.user_id = @user_id",
            USER_LOTTERIES
        ),
        vec![Param::new("@user_id".into(), user_id.clone())],
    );

    match query_document::<UserLottery, _, _>(&cosmos_db.database, "UserLotteries", query, true)
        .await
    {
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

                match add_document(&cosmos_db.database, USER_LOTTERIES, new_document).await {
                    Ok(_) => response,
                    Err(e) => {
                        let error_message = format!("Failed to update next reward time: {}", e);
                        log::error!("{}", &error_message);
                        HttpResponse::InternalServerError()
                            .json(ServerError::with_message(error_message))
                    }
                }
            } else {
                HttpResponse::Accepted().json(user_lottery)
            }
        }
    }
}

async fn update_credits(
    user_lottery: &UserLottery,
    reward_type: RewardType,
    cosmos_db: &Data<CosmosDb>,
) -> HttpResponse {
    adjust_credit(
        &cosmos_db.database,
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

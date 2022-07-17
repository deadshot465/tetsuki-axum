use crate::model::errors::ServerError;
use crate::model::user_credit::{UserCredit, UserCreditUpdateInfo, UserCreditUpdateOpt};
use actix_web::web::{Path, ServiceConfig};
use actix_web::{get, patch, post, HttpResponse, Responder};

use crate::shared::util::{add_document, get_documents, query_document};

pub fn config_credit_controller(cfg: &mut ServiceConfig) {
    cfg.service(get_all_user_credits)
        .service(get_single_user_credits)
        .service(add_credit)
        .service(reduce_credit);
    /*.service(add_user);*/
}

#[get("/credit")]
async fn get_all_user_credits() -> impl Responder {
    if let Some(credits) = get_documents::<UserCredit, _>("UserCredits").await {
        HttpResponse::Ok().json(credits)
    } else {
        HttpResponse::InternalServerError().json(ServerError {
            error_message: "Failed to retrieve user credits.".to_string(),
        })
    }
}

#[get("/credit/{user_id}")]
async fn get_single_user_credits(user_id: Path<String>) -> impl Responder {
    let query = format!(
        r#"SELECT * FROM UserCredits u WHERE u.user_id = "{}""#,
        user_id.into_inner()
    );

    if let Some(query_result) = query_document::<UserCredit, _, _>("UserCredits", query).await {
        HttpResponse::Ok().json(query_result.first().cloned().unwrap_or_default())
    } else {
        HttpResponse::NotFound().json(ServerError {
            error_message: "Failed to retrieve corresponding user's credit.".into(),
        })
    }
}

/*#[post("/credit")]
async fn add_user(
    request: actix_web::web::Json<UserCredit>,
    data: Data<Pool<Postgres>>,
) -> impl Responder {
    if request.username.is_empty() || request.user_id.is_empty() {
        return HttpResponse::BadRequest().body("Either the userId or the username is empty.");
    } else if request.credits < 0 {
        return HttpResponse::BadRequest().body("The amount of credits has to be greater than 0.");
    }

    let _ = sqlx::query(
        r#"INSERT INTO "UserCredits" ("Username", "UserId", "Credits") VALUES ($1, $2, $3)"#,
    )
    .bind(&request.username)
    .bind(&request.user_id)
    .bind(&request.credits)
    .execute(&**data)
    .await
    .expect("Failed to insert into database.");

    HttpResponse::Created().json((&*request).clone())
}*/

#[patch("/credit/{user_id}/plus")]
async fn add_credit(
    user_id: Path<String>,
    request: actix_web::web::Json<UserCreditUpdateInfo>,
) -> impl Responder {
    adjust_credit(user_id, request, UserCreditUpdateOpt::Plus).await
}

#[patch("/credit/{user_id}/minus")]
async fn reduce_credit(
    user_id: Path<String>,
    request: actix_web::web::Json<UserCreditUpdateInfo>,
) -> impl Responder {
    adjust_credit(user_id, request, UserCreditUpdateOpt::Minus).await
}

async fn adjust_credit(
    user_id: Path<String>,
    request: actix_web::web::Json<UserCreditUpdateInfo>,
    opt: UserCreditUpdateOpt,
) -> impl Responder {
    let query = format!(
        r#"SELECT * FROM UserCredits u WHERE u.user_id = "{}""#,
        user_id.into_inner()
    );

    let query_result = query_document::<UserCredit, _, _>("UserCredits", query).await;
    if query_result.is_none() {
        return HttpResponse::NotFound().json(ServerError {
            error_message: "Cannot update user's credit because the specified user doesn't exist."
                .into(),
        });
    }

    let query_result = query_result
        .and_then(|v| v.first().cloned())
        .unwrap_or_default();
    let new_document = UserCredit {
        credits: match opt {
            UserCreditUpdateOpt::Plus => query_result.credits + request.credit,
            UserCreditUpdateOpt::Minus => query_result.credits - request.credit,
        },
        ..query_result
    };

    match add_document("UserCredits", new_document).await {
        Ok(_) => HttpResponse::Ok().finish(),
        Err(e) => {
            let error_message = format!("{}", e);
            log::error!("{}", &error_message);
            HttpResponse::InternalServerError().json(ServerError { error_message })
        }
    }
}

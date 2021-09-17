use crate::model::user_credit::{UserCredit, UserCreditUpdateInfo};
use actix_web::web::{Data, Path, ServiceConfig};
use actix_web::{get, patch, post, HttpResponse, Responder};
use sqlx::{Pool, Postgres};

pub fn config_credit_controller(cfg: &mut ServiceConfig) {
    cfg.service(get_all_user_credits)
        .service(get_single_user_credits)
        .service(add_user)
        .service(add_credit)
        .service(reduce_credit);
}

#[get("/credit")]
async fn get_all_user_credits(data: Data<Pool<Postgres>>) -> impl Responder {
    let query_result = sqlx::query_as::<_, UserCredit>("SELECT * FROM \"UserCredits\"")
        .fetch_all(&**data)
        .await
        .expect("Failed to query from the database.");
    let serialized =
        serde_json::to_string_pretty(&query_result).expect("Failed to serialize to JSON.");
    HttpResponse::Ok().body(serialized)
}

#[get("/credit/{user_id}")]
async fn get_single_user_credits(user_id: Path<i32>, data: Data<Pool<Postgres>>) -> impl Responder {
    let query_result = sqlx::query_as::<_, UserCredit>(&format!(
        "SELECT * FROM \"UserCredits\" WHERE \"Id\" = {}",
        user_id
    ))
    .fetch_one(&**data)
    .await
    .expect("Failed to get user credit.");
    let serialized =
        serde_json::to_string_pretty(&query_result).expect("Failed to serialize to JSON.");
    HttpResponse::Ok().body(serialized)
}

#[post("/credit")]
pub async fn add_user(
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

    HttpResponse::Ok().body(String::new())
}

#[patch("/credit/{user_id}/plus")]
pub async fn add_credit(
    user_id: Path<String>,
    request: actix_web::web::Json<UserCreditUpdateInfo>,
    data: Data<Pool<Postgres>>,
) -> impl Responder {
    let _ =
        sqlx::query(r#"UPDATE "UserCredits" SET "Credits" = "Credits" + $1 WHERE "UserId" = $2"#)
            .bind(request.credit)
            .bind((&*user_id).clone())
            .execute(&**data)
            .await
            .expect("Failed to update user's credit in the database.");
    HttpResponse::Ok().body(String::new())
}

#[patch("/credit/{user_id}/minus")]
pub async fn reduce_credit(
    user_id: Path<String>,
    request: actix_web::web::Json<UserCreditUpdateInfo>,
    data: Data<Pool<Postgres>>,
) -> impl Responder {
    let _ =
        sqlx::query(r#"UPDATE "UserCredits" SET "Credits" = "Credits" - $1 WHERE "UserId" = $2"#)
            .bind(request.credit)
            .bind((&*user_id).clone())
            .execute(&**data)
            .await
            .expect("Failed to update user's credit in the database.");
    HttpResponse::Ok().body(String::new())
}

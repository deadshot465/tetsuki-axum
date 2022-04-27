use actix_web::{get, HttpResponse, post, Responder};
use actix_web::web::{Data, ServiceConfig};
use sqlx::{Pool, Postgres};
use crate::model::games::witchs_bar::PlayerRecordPayload;

pub fn config_record_controller(cfg: &mut ServiceConfig) {
    cfg.service(get_all_witchs_bar_records)
        .service(post_witchs_bar_record);
}

#[get("/games/witchs_bar")]
async fn get_all_witchs_bar_records(data: Data<Pool<Postgres>>) -> impl Responder {
    let query_result = sqlx::query_as::<_, PlayerRecordPayload>("SELECT * FROM \"WitchsBarRecords\"")
        .fetch_all(&**data)
        .await;

    match query_result {
        Ok(result) => {
            match serde_json::to_vec(&result) {
                Ok(payload) => {
                    HttpResponse::Ok().body(payload)
                }
                Err(e) => {
                    let error_msg = format!("An error occurred when serializing the payload: {}", e);
                    log::error!("{}", &error_msg);
                    HttpResponse::InternalServerError().body(error_msg)
                }
            }
        }
        Err(e) => {
            let error_msg = format!("An error occurred when querying from the database: {:?}", &e);
            log::error!("{}", &error_msg);
            HttpResponse::InternalServerError().body(error_msg)
        }
    }
}

#[post("/games/witchs_bar")]
async fn post_witchs_bar_record(payload: actix_web::web::Json<PlayerRecordPayload>, data: Data<Pool<Postgres>>) -> impl Responder {
    if payload.player_name.is_empty() {
        return HttpResponse::BadRequest().body("The player name cannot be empty!");
    }

    let result = sqlx::query(
        r#"INSERT INTO "WitchsBarRecords" ("player_name", "score") VALUES ($1, $2)"#
    )
        .bind(&payload.player_name)
        .bind(&payload.score)
        .execute(&**data)
        .await;

    if let Err(e) = result {
        let error_msg = format!("An error occurred when inserting into the database: {:?}", &e);
        log::error!("{}", &error_msg);
        HttpResponse::InternalServerError().body(error_msg)
    } else {
        HttpResponse::Created().json(&*payload)
    }
}

use actix_web::{get, HttpResponse, post, Responder};
use actix_web::web::{Data, Path, ServiceConfig};
use sqlx::{Pool, Postgres};
use crate::model::games::witchs_bar::PlayerRecordPayload;

pub fn config_record_controller(cfg: &mut ServiceConfig) {
    cfg.service(get_all_witchs_bar_records)
        .service(post_witchs_bar_record)
        .service(get_stage_witchs_bar_records);
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
            let error_msg = format!("An error occurred when querying the database: {:?}", &e);
            log::error!("{}", &error_msg);
            HttpResponse::InternalServerError().body(error_msg)
        }
    }
}

#[get("/games/witchs_bar/{stage_id}")]
async fn get_stage_witchs_bar_records(stage_id: Path<i32>, data: Data<Pool<Postgres>>) -> impl Responder {
    let query_result = sqlx::query_as::<_, PlayerRecordPayload>("SELECT * FROM \"WitchsBarRecords\" WHERE \"stage\" = $1")
        .bind(&*stage_id)
        .fetch_optional(&**data)
        .await;

    match query_result {
        Ok(opt) => {
            match opt {
                None => {
                    let error_msg = format!("Cannot find corresponding records for the specified level {}.", stage_id.into_inner());
                    HttpResponse::NotFound().body(error_msg)
                }
                Some(payload) => {
                    HttpResponse::Ok().json(payload)
                }
            }
        }
        Err(e) => {
            let error_msg = format!("An error occurred when querying the database: {:?}", &e);
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
        r#"INSERT INTO "WitchsBarRecords" ("player_name", "score", "stage") VALUES ($1, $2, $3)"#
    )
        .bind(&payload.player_name)
        .bind(&payload.score)
        .bind(&payload.stage)
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

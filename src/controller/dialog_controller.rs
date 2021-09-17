use crate::shared::web_driver::get_dialog;
use actix_web::body::*;
use actix_web::web::ServiceConfig;
use actix_web::{get, HttpResponse, Responder};

pub fn config_dialog_controller(cfg: &mut ServiceConfig) {
    cfg.service(generate_dialog);
}

#[get("/dialog")]
async fn generate_dialog() -> impl Responder {
    let result = get_dialog().await.expect("Failed!");
    HttpResponse::Ok().body(AnyBody::Bytes(actix_web::web::Bytes::from(result)))
}

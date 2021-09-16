use crate::shared::get_dialog;
use actix_web::web::{Bytes, ServiceConfig};
use actix_web::{get, HttpResponse, Responder};

pub fn config_dialog_controller(cfg: &mut ServiceConfig) {
    cfg.service(generate_dialog);
}

#[get("/dialog")]
async fn generate_dialog() -> impl Responder {
    let result = get_dialog().await.expect("Failed!");
    HttpResponse::Ok().body(Bytes(actix_web::web::Bytes::from(result)))
}

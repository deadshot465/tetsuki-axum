use crate::model::cosmos_db::CosmosDb;
use crate::model::errors::ServerError;
use crate::model::mal_character::MalCharacter;
use crate::shared::util::{add_document, query_document, query_document_within_collection};
use actix_web::web::{Data, Path, ServiceConfig};
use actix_web::{get, post, HttpResponse, Responder};
use azure_data_cosmos::prelude::{CollectionClient, Param, Query};
use uuid::Uuid;

pub const MAL_CHARACTERS: &str = "MalCharacters";

pub fn config_mal_character_controller(cfg: &mut ServiceConfig) {
    cfg.service(get_mal_character)
        .service(post_mal_character)
        .service(get_all_mal_characters);
}

pub async fn inner_get_all_mal_characters(collection: &CollectionClient) -> Vec<MalCharacter> {
    let query = Query::new(format!("SELECT * FROM {} m", MAL_CHARACTERS));
    query_document_within_collection::<MalCharacter, _>(collection, query, true)
        .await
        .unwrap_or_default()
}

#[get("/mal_character")]
async fn get_all_mal_characters(cosmos_db: Data<CosmosDb>) -> impl Responder {
    let collection = cosmos_db.database.collection_client(MAL_CHARACTERS);
    let query_result = inner_get_all_mal_characters(&collection).await;
    HttpResponse::Ok().json(query_result)
}

#[get("/mal_character/{id}")]
async fn get_mal_character(id: Path<i32>, cosmos_db: Data<CosmosDb>) -> impl Responder {
    let id = id.into_inner();
    let query = Query::with_params(
        "SELECT * FROM MalCharacters m WHERE m.Id = @id".into(),
        vec![Param::new("@id".into(), id)],
    );

    let query_result =
        query_document::<MalCharacter, _, _>(&cosmos_db.database, MAL_CHARACTERS, query, true)
            .await
            .and_then(|v| v.first().cloned());

    match query_result {
        None => HttpResponse::NotFound().json(ServerError::with_message(
            "The specified mal character is not found.",
        )),
        Some(mal_character) => HttpResponse::Ok().json(mal_character),
    }
}

#[post("/mal_character")]
async fn post_mal_character(
    payload: actix_web::web::Json<MalCharacter>,
    cosmos_db: Data<CosmosDb>,
) -> impl Responder {
    let mut payload = payload.into_inner();
    if payload.id.is_empty() {
        payload.id = Uuid::new_v4().to_string()
    }

    match add_document(&cosmos_db.database, MAL_CHARACTERS, payload.clone()).await {
        Ok(_) => HttpResponse::Created().json(payload),
        Err(e) => {
            let error_message = format!("Failed to insert mal character into database: {}", e);
            log::error!("{}", &error_message);
            HttpResponse::InternalServerError().json(ServerError::with_message(error_message))
        }
    }
}

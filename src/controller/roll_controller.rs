use crate::controller::mal_character_controller::{inner_get_all_mal_characters, MAL_CHARACTERS};
use crate::model::errors::ServerError;
use crate::model::user_roll::{GetRollResult, UserRoll};
use crate::shared::util::{
    add_document, initialize_clients, query_document, query_document_within_collection,
};
use actix_web::web::{Path, ServiceConfig};
use actix_web::{get, post, HttpResponse, Responder};
use azure_data_cosmos::prelude::{CollectionClient, Param, Query};
use std::collections::HashMap;
use uuid::Uuid;

const USER_ROLLS: &str = "UserRolls";

pub fn config_roll_controller(cfg: &mut ServiceConfig) {
    cfg.service(post_user_roll)
        .service(get_last_roll_id)
        .service(get_all_user_rolls)
        .service(get_user_roll_by_id);
}

#[post("/user_roll/{user_id}/new")]
async fn post_user_roll(
    _user_id: Path<String>,
    payload: actix_web::web::Json<UserRoll>,
) -> impl Responder {
    let mut payload = payload.into_inner();
    if payload.id.is_empty() {
        payload.id = Uuid::new_v4().to_string();
    }

    match add_document(USER_ROLLS, payload.clone()).await {
        Ok(_) => HttpResponse::Created().json(payload),
        Err(e) => {
            let error_message = format!("Failed to insert user roll into database: {}", e);
            log::error!("{}", &error_message);
            HttpResponse::InternalServerError().json(ServerError::with_message(error_message))
        }
    }
}

#[get("/user_roll/{user_id}")]
async fn get_all_user_rolls(user_id: Path<String>) -> impl Responder {
    let query_result = inner_get_all_user_rolls_with_names(user_id.into_inner()).await;
    HttpResponse::Ok().json(query_result)
}

#[get("/user_roll/{user_id}/{roll_id}")]
async fn get_user_roll_by_id(path: Path<(String, i32)>) -> impl Responder {
    let (user_id, roll_id) = path.into_inner();
    let all_user_rolls_with_names = inner_get_all_user_rolls_with_names(user_id).await;
    let result = all_user_rolls_with_names
        .into_iter()
        .find(|res| res.user_roll.roll_id == roll_id);

    match result {
        Some(res) => HttpResponse::Ok().json(res),
        None => HttpResponse::NotFound().json(ServerError::with_message(
            "Cannot find the specified roll within user's rolls.",
        )),
    }
}

#[get("/user_roll/ids/last")]
async fn get_last_roll_id() -> impl Responder {
    let query = Query::new(format!("SELECT MAX(u.Id) FROM {} u", USER_ROLLS));
    let query_result = query_document::<HashMap<String, i32>, _, _>(USER_ROLLS, query, false)
        .await
        .and_then(|v| v.first().cloned());

    match query_result {
        Some(map) => {
            let last_id = map["$1"];
            let mut result = HashMap::new();
            result.insert("last_id".to_string(), last_id);
            log::debug!("{:?}", &result);
            HttpResponse::Ok().json(result)
        }
        None => HttpResponse::NoContent().finish(),
    }
}

async fn inner_get_all_user_rolls_with_names(user_id: String) -> Vec<GetRollResult> {
    let (_client, database) = initialize_clients();
    let roll_collection = database.collection_client(USER_ROLLS);
    let mal_character_collection = database.collection_client(MAL_CHARACTERS);
    let query_result = inner_get_all_user_rolls(&roll_collection, user_id).await;
    let mal_characters = inner_get_all_mal_characters(&mal_character_collection).await;
    query_result
        .into_iter()
        .map(|roll| GetRollResult {
            user_roll: roll.clone(),
            mal_character: mal_characters
                .iter()
                .find(|character| character.character_id == roll.mal_character_id)
                .cloned()
                .unwrap_or_default(),
        })
        .collect::<Vec<_>>()
}

async fn inner_get_all_user_rolls(collection: &CollectionClient, user_id: String) -> Vec<UserRoll> {
    let query = Query::with_params(
        "SELECT * FROM UserRolls u WHERE u.UserId = @user_id".into(),
        vec![Param::new("@user_id".into(), user_id)],
    );

    query_document_within_collection::<UserRoll, _>(collection, query, true)
        .await
        .unwrap_or_default()
}

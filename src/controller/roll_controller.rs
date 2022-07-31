use crate::controller::mal_character_controller::{inner_get_all_mal_characters, MAL_CHARACTERS};
use crate::model::cosmos_db::CosmosDb;
use crate::model::errors::ServerError;
use crate::model::user_roll::{GetRollResult, UserRoll};
use crate::shared::util::{add_document, get_documents, query_document_within_collection};
use actix_web::web::{Data, Path, ServiceConfig};
use actix_web::{get, post, HttpResponse, Responder};
use azure_data_cosmos::prelude::{CollectionClient, Param, Query};
use uuid::Uuid;

const USER_ROLLS: &str = "UserRolls";

pub fn config_roll_controller(cfg: &mut ServiceConfig) {
    cfg.service(post_user_roll)
        .service(get_all_rolls)
        .service(get_all_user_rolls)
        .service(get_user_roll_by_id);
}

#[post("/user_roll/{user_id}/new")]
async fn post_user_roll(
    _user_id: Path<String>,
    payload: actix_web::web::Json<UserRoll>,
    cosmos_db: Data<CosmosDb>,
) -> impl Responder {
    let mut payload = payload.into_inner();
    if payload.id.is_empty() {
        payload.id = Uuid::new_v4().to_string();
    }

    match add_document(&cosmos_db.database, USER_ROLLS, payload.clone()).await {
        Ok(_) => HttpResponse::Created().json(payload),
        Err(e) => {
            let error_message = format!("Failed to insert user roll into database: {}", e);
            log::error!("{}", &error_message);
            HttpResponse::InternalServerError().json(ServerError::with_message(error_message))
        }
    }
}

#[get("/user_roll")]
async fn get_all_rolls(cosmos_db: Data<CosmosDb>) -> impl Responder {
    let query_result = get_documents::<UserRoll, _>(&cosmos_db.database, USER_ROLLS)
        .await
        .unwrap_or_default();
    HttpResponse::Ok().json(query_result)
}

#[get("/user_roll/{user_id}")]
async fn get_all_user_rolls(user_id: Path<String>, cosmos_db: Data<CosmosDb>) -> impl Responder {
    let query_result = inner_get_all_user_rolls_with_names(user_id.into_inner(), cosmos_db).await;
    HttpResponse::Ok().json(query_result)
}

#[get("/user_roll/{user_id}/{roll_id}")]
async fn get_user_roll_by_id(
    path: Path<(String, i32)>,
    cosmos_db: Data<CosmosDb>,
) -> impl Responder {
    let (user_id, roll_id) = path.into_inner();
    let all_user_rolls_with_names = inner_get_all_user_rolls_with_names(user_id, cosmos_db).await;
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

async fn inner_get_all_user_rolls_with_names(
    user_id: String,
    cosmos_db: Data<CosmosDb>,
) -> Vec<GetRollResult> {
    let roll_collection = cosmos_db.database.collection_client(USER_ROLLS);
    let mal_character_collection = cosmos_db.database.collection_client(MAL_CHARACTERS);
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

use crate::model::errors::ServerError;
use crate::model::user_credit::{UserCredit, UserCreditUpdateInfo, UserCreditUpdateOpt};
use actix_web::web::{Path, ServiceConfig};
use actix_web::{delete, get, patch, post, HttpResponse, Responder};
use azure_data_cosmos::prelude::{Param, Query};

use crate::shared::util::{
    add_document, adjust_credit, get_documents, initialize_clients, query_document,
    query_document_within_collection,
};

pub const USER_CREDITS: &str = "UserCredits";

pub fn config_credit_controller(cfg: &mut ServiceConfig) {
    cfg.service(get_all_user_credits)
        .service(get_single_user_credits)
        .service(add_credit)
        .service(reduce_credit)
        .service(add_user)
        .service(delete_user);
}

#[get("/credit")]
async fn get_all_user_credits() -> impl Responder {
    if let Some(credits) = get_documents::<UserCredit, _>(USER_CREDITS).await {
        HttpResponse::Ok().json(credits)
    } else {
        HttpResponse::InternalServerError().json(ServerError {
            error_message: "Failed to retrieve user credits.".to_string(),
        })
    }
}

#[get("/credit/{user_id}")]
async fn get_single_user_credits(user_id: Path<String>) -> impl Responder {
    let query = Query::with_params(
        format!(
            "SELECT * FROM {} u WHERE u.user_id = @user_id",
            USER_CREDITS
        ),
        vec![Param::new("@user_id".into(), user_id.into_inner())],
    );

    if let Some(query_result) = query_document::<UserCredit, _, _>(USER_CREDITS, query).await {
        HttpResponse::Ok().json(query_result.first().cloned().unwrap_or_default())
    } else {
        HttpResponse::NotFound().json(ServerError {
            error_message: "The specified user's credit info is not found.".into(),
        })
    }
}

#[post("/credit")]
async fn add_user(request: actix_web::web::Json<UserCredit>) -> impl Responder {
    if request.username.is_empty() || request.user_id.is_empty() {
        return HttpResponse::BadRequest().json(ServerError::with_message(
            "Either the user ID or the username is empty.",
        ));
    } else if request.credits < 0 {
        return HttpResponse::BadRequest().json(ServerError::with_message(
            "The amount of credits has to be greater than 0.",
        ));
    }

    let query = Query::with_params(
        format!(
            "SELECT * FROM {} u WHERE u.user_id = @user_id",
            USER_CREDITS
        ),
        vec![Param::new("@user_id".into(), request.user_id.clone())],
    );

    let query_result = query_document::<UserCredit, _, _>(USER_CREDITS, query).await;
    if query_result.is_some() {
        return HttpResponse::BadRequest().json(ServerError::with_message(
            "Specified user already exists. Use PATCH to update user's information.",
        ));
    }

    match add_document(USER_CREDITS, request.into_inner()).await {
        Ok(_) => HttpResponse::Created().finish(),
        Err(e) => {
            let error_message = format!("{}", e);
            log::error!("{}", &error_message);
            HttpResponse::InternalServerError().json(ServerError::with_message(error_message))
        }
    }
}

#[patch("/credit/{user_id}/plus")]
async fn add_credit(
    user_id: Path<String>,
    request: actix_web::web::Json<UserCreditUpdateInfo>,
) -> impl Responder {
    adjust_credit(
        user_id.into_inner(),
        request.into_inner(),
        UserCreditUpdateOpt::Plus,
    )
    .await
}

#[patch("/credit/{user_id}/minus")]
async fn reduce_credit(
    user_id: Path<String>,
    request: actix_web::web::Json<UserCreditUpdateInfo>,
) -> impl Responder {
    adjust_credit(
        user_id.into_inner(),
        request.into_inner(),
        UserCreditUpdateOpt::Minus,
    )
    .await
}

#[delete("/credit/{user_id}")]
async fn delete_user(user_id: Path<String>) -> impl Responder {
    let (_client, database) = initialize_clients();
    let collection = database.collection_client(USER_CREDITS);

    let query = Query::with_params(
        format!(
            "SELECT * FROM {} u WHERE u.user_id = @user_id",
            USER_CREDITS
        ),
        vec![Param::new("@user_id".into(), user_id.into_inner())],
    );
    let query_result = query_document_within_collection::<UserCredit, _>(&collection, query)
        .await
        .and_then(|result| result.first().cloned());

    if let Some(result) = query_result {
        let document = collection.document_client(result.id.clone(), &result.id);
        match document {
            Ok(doc) => match doc.delete_document().into_future().await {
                Ok(_) => HttpResponse::NoContent().finish(),
                Err(e) => {
                    let error_message = format!("Failed to delete user credit: {}", e);
                    log::error!("{}", &error_message);
                    HttpResponse::InternalServerError()
                        .json(ServerError::with_message(error_message))
                }
            },
            Err(e) => {
                let error_message = format!("{}", e);
                log::error!("{}", &error_message);
                HttpResponse::InternalServerError().json(ServerError::with_message(error_message))
            }
        }
    } else {
        HttpResponse::NotFound().json(ServerError::with_message(
            "The specified user doesn't exist.",
        ))
    }
}

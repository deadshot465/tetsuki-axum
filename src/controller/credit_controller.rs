use crate::model::errors::ServerError;
use crate::model::response::Response;
use crate::model::user::{Credit, CreditUpdateInfo, UserCredit};
use actix_web::web::{Data, Path, ServiceConfig};
use actix_web::{get, patch, post, HttpResponse, Responder};
use gremlin_client::aio::GremlinClient;
use tokio_stream::StreamExt;

pub fn config_credit_controller(cfg: &mut ServiceConfig) {
    cfg.service(get_all_user_credits)
        .service(get_single_user_credits)
        .service(add_user)
        .service(add_credit)
        .service(reduce_credit);
}

#[get("/credit")]
async fn get_all_user_credits(gremlin: Data<GremlinClient>) -> impl Responder {
    let query = gremlin.execute("g.V().hasLabel('user').project('username', 'user_id', 'credits').by('username').by('user_id').by(out('has').hasLabel('credit').values('amount'))", &[])
        .await
        .map(|res| res.filter_map(Result::ok).map(UserCredit::try_from));

    match query {
        Ok(result) => match result.collect::<Result<Vec<_>, _>>().await {
            Ok(user_credits) => HttpResponse::Ok().json(user_credits),
            Err(e) => {
                let error_message = format!("Failed to construct UserCredit struct: {}", e);
                log::error!("{}", &error_message);
                HttpResponse::InternalServerError().json(ServerError { error_message })
            }
        },
        Err(e) => {
            let error_message = format!("Failed to retrieve data from database: {:?}", e);
            log::error!("{}", &error_message);
            HttpResponse::InternalServerError().json(ServerError { error_message })
        }
    }
}

#[get("/credit/{user_id}")]
async fn get_single_user_credits(
    user_id: Path<String>,
    gremlin: Data<GremlinClient>,
) -> impl Responder {
    let user_id = user_id.into_inner();
    let query = gremlin
        .execute("g.V().hasLabel('user').has('user_id', params).project('username', 'user_id', 'credits').by('username').by('user_id').by(out('has').hasLabel('credit').values('amount'))", &[("params", &user_id.as_str())])
        .await
        .map(|res| res.filter_map(Result::ok).map(UserCredit::try_from));

    match query {
        Ok(result) => match result.collect::<Result<Vec<_>, _>>().await {
            Ok(user_credits) => {
                if user_credits.is_empty() {
                    HttpResponse::NotFound().json(ServerError {
                        error_message: "Cannot find the specified user.".to_string(),
                    })
                } else {
                    HttpResponse::Ok().json(&user_credits[0])
                }
            }
            Err(e) => {
                let error_message = format!("Failed to construct UserCredit struct: {}", e);
                log::error!("{}", &error_message);
                HttpResponse::InternalServerError().json(ServerError { error_message })
            }
        },
        Err(e) => {
            let error_message = format!("Failed to retrieve data from database: {:?}", e);
            log::error!("{}", &error_message);
            HttpResponse::InternalServerError().json(ServerError { error_message })
        }
    }
}

#[post("/credit")]
async fn add_user(
    request: actix_web::web::Json<UserCredit>,
    gremlin: Data<GremlinClient>,
) -> impl Responder {
    if request.username.is_empty() || request.user_id.is_empty() {
        return HttpResponse::BadRequest().body("Either the userId or the username is empty.");
    } else if request.credits < 0 {
        return HttpResponse::BadRequest().body("The amount of credits has to be greater than 0.");
    }

    let sanitized_username = request.username.to_lowercase().replace(' ', "_");

    let add_user_cmd = format!(
        "g.addV('user').property('id', {}).property('username', {}).property('user_id', {}).property('pk', 'pk')",
        &sanitized_username,
        &request.username,
        &request.user_id
    );

    let sanitized_user_credit_id = format!("{}_credit", &sanitized_username);

    let add_credit_cmd = format!(
        "g.addV('credit').property('id', {}).property('amount', {}).property('pk', 'pk')",
        &sanitized_user_credit_id, &request.credits
    );

    let add_edge_cmd = format!(
        "g.V('{}').addE('has').to(g.V('{}'))",
        &sanitized_username, &sanitized_user_credit_id
    );

    let query = gremlin.execute(add_user_cmd, &[]).await;

    if let Err(e) = query {
        let error_message = format!("Failed to insert user into the database: {:?}", e);
        log::error!("{}", &error_message);
        return HttpResponse::InternalServerError().json(ServerError { error_message });
    }

    let query = gremlin.execute(add_credit_cmd, &[]).await;

    if let Err(e) = query {
        let error_message = format!("Failed to insert credit into the database: {:?}", e);
        log::error!("{}", &error_message);
        return HttpResponse::InternalServerError().json(ServerError { error_message });
    }

    let query = gremlin.execute(add_edge_cmd, &[]).await;

    if let Err(e) = query {
        let error_message = format!("Failed to insert edge into the database: {:?}", e);
        log::error!("{}", &error_message);
        return HttpResponse::InternalServerError().json(ServerError { error_message });
    }

    HttpResponse::Created().json((&*request).clone())
}

#[patch("/credit/{user_id}/plus")]
async fn add_credit(
    user_id: Path<String>,
    request: actix_web::web::Json<CreditUpdateInfo>,
    gremlin: Data<GremlinClient>,
) -> impl Responder {
    let user_id = user_id.into_inner();
    let credit: Credit;

    match get_credit(&user_id, gremlin.clone()).await {
        Response::Success(c) => credit = c,
        Response::NotFound(msg) => {
            return HttpResponse::NotFound().json(ServerError { error_message: msg });
        }
        Response::InternalError(msg) => {
            return HttpResponse::InternalServerError().json(ServerError { error_message: msg });
        }
    }

    let new_amount = credit.amount + request.credit as i64;

    let cmd = format!(
        "g.V().hasLabel('user').has('user_id', '{}').out('has').hasLabel('credit').property('amount', {})",
        &user_id,
        new_amount
    );

    let query = gremlin.execute(&cmd, &[]).await;

    if let Err(e) = query {
        let error_message = format!("Failed to update user's credit: {:?}", e);
        log::error!("{}", &error_message);
        HttpResponse::InternalServerError().json(ServerError { error_message })
    } else {
        HttpResponse::Ok().finish()
    }
}

#[patch("/credit/{user_id}/minus")]
async fn reduce_credit(
    user_id: Path<String>,
    request: actix_web::web::Json<CreditUpdateInfo>,
    gremlin: Data<GremlinClient>,
) -> impl Responder {
    let user_id = user_id.into_inner();
    let credit: Credit;

    match get_credit(&user_id, gremlin.clone()).await {
        Response::Success(c) => credit = c,
        Response::NotFound(msg) => {
            return HttpResponse::NotFound().json(ServerError { error_message: msg });
        }
        Response::InternalError(msg) => {
            return HttpResponse::InternalServerError().json(ServerError { error_message: msg });
        }
    }

    let new_amount = credit.amount - request.credit as i64;

    let cmd = format!(
        "g.V().hasLabel('user').has('user_id', '{}').out('has').hasLabel('credit').property('amount', {})",
        &user_id,
        new_amount
    );

    let query = gremlin.execute(&cmd, &[]).await;

    if let Err(e) = query {
        let error_message = format!("Failed to update user's credit: {:?}", e);
        log::error!("{}", &error_message);
        HttpResponse::InternalServerError().json(ServerError { error_message })
    } else {
        HttpResponse::Ok().finish()
    }
}

async fn get_credit(user_id: &str, gremlin: Data<GremlinClient>) -> Response<Credit> {
    let cmd = format!(
        "g.V().hasLabel('user').has('user_id', '{}').out('has').hasLabel('credit').valueMap()",
        user_id
    );

    let query = gremlin
        .execute(&cmd, &[])
        .await
        .map(|r| r.filter_map(Result::ok).map(Credit::try_from));

    let credit: Credit;

    match query {
        Ok(m) => match m.collect::<Result<Vec<_>, _>>().await {
            Ok(r) => {
                if r.is_empty() {
                    return Response::NotFound("Cannot find the specified user.".to_string());
                }
                credit = r[0].clone();
            }
            Err(e) => {
                let error_message = format!("Failed to construct Credit struct: {:?}", e);
                log::error!("{}", &error_message);
                return Response::InternalError(error_message);
            }
        },
        Err(e) => {
            let error_message = format!(
                "Failed to retrieve specified user's credit from database: {:?}",
                e
            );
            log::error!("{}", &error_message);
            return Response::InternalError(error_message);
        }
    }

    Response::Success(credit)
}

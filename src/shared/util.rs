use crate::model::errors::ServerError;
use crate::model::user_credit::{UserCredit, UserCreditUpdateInfo, UserCreditUpdateOpt};
use crate::CONFIGURATION;
use actix_web::HttpResponse;
use azure_data_cosmos::prelude::*;
use futures::StreamExt;
use serde::de::DeserializeOwned;
use serde::Serialize;

pub async fn get_documents<T, S>(collection_name: S) -> Option<Vec<T>>
where
    T: DeserializeOwned + Send + Sync + Clone,
    S: Into<std::borrow::Cow<'static, str>>,
{
    let (_client, database) = initialize_clients();
    let collection = database.collection_client(collection_name);

    let documents = collection
        .list_documents()
        .into_stream::<T>()
        .collect::<Vec<_>>()
        .await
        .into_iter()
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| log::error!("Failed to retrieve user credits: {}", e))
        .ok()
        .and_then(|result| result.first().cloned())
        .map(|response| {
            response
                .documents
                .into_iter()
                .map(|document| document.document)
                .collect::<Vec<_>>()
        });
    documents
}

pub async fn query_document<T, S, Q>(collection_name: S, query: Q) -> Option<Vec<T>>
where
    T: DeserializeOwned + Send + Sync + Clone,
    S: Into<std::borrow::Cow<'static, str>>,
    Q: Into<Query>,
{
    let (_client, database) = initialize_clients();
    let collection = database.collection_client(collection_name);
    query_document_within_collection(&collection, query).await
}

pub async fn query_document_within_collection<T, Q>(
    collection: &CollectionClient,
    query: Q,
) -> Option<Vec<T>>
where
    T: DeserializeOwned + Send + Sync + Clone,
    Q: Into<Query>,
{
    let documents: Option<Vec<T>> = collection
        .query_documents(query)
        .query_cross_partition(true)
        .into_stream::<T>()
        .collect::<Vec<_>>()
        .await
        .into_iter()
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| log::error!("Failed to retrieve document: {}", e))
        .ok()
        .and_then(|result| result.first().cloned())
        .map(|response| {
            response
                .results
                .into_iter()
                .map(|data| match data {
                    QueryResult::Document(doc) => doc.result,
                    QueryResult::Raw(raw) => raw,
                })
                .collect()
        });

    if let Some(document) = documents {
        if document.is_empty() {
            None
        } else {
            Some(document)
        }
    } else {
        None
    }
}

pub async fn add_document<S, D>(
    collection_name: S,
    new_document: D,
) -> Result<CreateDocumentResponse, azure_core::error::Error>
where
    S: Into<std::borrow::Cow<'static, str>>,
    D: Serialize + CosmosEntity + Send + 'static,
{
    let (_client, database) = initialize_clients();
    let collection = database.collection_client(collection_name);

    collection
        .create_document(new_document)
        .is_upsert(true)
        .into_future()
        .await
}

pub async fn adjust_credit(
    user_id: String,
    request: UserCreditUpdateInfo,
    opt: UserCreditUpdateOpt,
) -> HttpResponse {
    let query = Query::with_params(
        "SELECT * FROM UserCredits u WHERE u.user_id = @user_id".into(),
        vec![Param::new("@user_id".into(), user_id)],
    );

    let query_result = query_document::<UserCredit, _, _>("UserCredits", query).await;
    if query_result.is_none() {
        return HttpResponse::NotFound().json(ServerError {
            error_message: "Cannot update user's credit because the specified user doesn't exist."
                .into(),
        });
    }

    let query_result = query_result
        .and_then(|v| v.first().cloned())
        .unwrap_or_default();
    let new_document = UserCredit {
        credits: match opt {
            UserCreditUpdateOpt::Plus => query_result.credits + request.credit,
            UserCreditUpdateOpt::Minus => query_result.credits - request.credit,
        },
        ..query_result
    };

    match add_document("UserCredits", new_document.clone()).await {
        Ok(_) => HttpResponse::Ok().json(new_document),
        Err(e) => {
            let error_message = format!("{}", e);
            log::error!("{}", &error_message);
            HttpResponse::InternalServerError().json(ServerError { error_message })
        }
    }
}

pub fn initialize_clients() -> (CosmosClient, DatabaseClient) {
    let authorization_token =
        AuthorizationToken::primary_from_base64(&CONFIGURATION.cosmos_db_primary_key)
            .map_err(|e| log::error!("Failed to generate authorization token for CosmosDB: {}", e))
            .expect("Failed to generate authorization token for CosmosDB.");

    let client = CosmosClient::new(
        CONFIGURATION.cosmos_db_account.clone(),
        authorization_token,
        CosmosOptions::default(),
    );

    let database = client.database_client(&CONFIGURATION.cosmos_db_database_name);
    (client, database)
}

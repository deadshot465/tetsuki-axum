use crate::controller::credit_controller::USER_CREDITS;
use crate::model::cosmos_db::CosmosDb;
use crate::model::errors::ServerError;
use crate::model::user_credit::{UserCredit, UserCreditUpdateInfo, UserCreditUpdateOpt};
use crate::CONFIGURATION;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use azure_data_cosmos::prelude::*;
use futures::StreamExt;
use serde::de::DeserializeOwned;
use serde::Serialize;

pub async fn get_documents<T, S>(database: &DatabaseClient, collection_name: S) -> Option<Vec<T>>
where
    T: DeserializeOwned + Send + Sync + Clone,
    S: Into<std::borrow::Cow<'static, str>>,
{
    let collection = database.collection_client(collection_name);

    collection
        .list_documents()
        .into_stream::<T>()
        .collect::<Vec<_>>()
        .await
        .into_iter()
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| tracing::error!("Failed to retrieve all documents: {}", e))
        .ok()
        .and_then(|result| result.first().cloned())
        .map(|response| {
            response
                .documents
                .into_iter()
                .map(|document| document.document)
                .collect::<Vec<_>>()
        })
}

pub async fn query_document<T, S, Q>(
    database: &DatabaseClient,
    collection_name: S,
    query: Q,
    cross_partition: bool,
) -> Option<Vec<T>>
where
    T: DeserializeOwned + Send + Sync + Clone,
    S: Into<std::borrow::Cow<'static, str>>,
    Q: Into<Query>,
{
    let collection = database.collection_client(collection_name);
    query_document_within_collection(&collection, query, cross_partition).await
}

pub async fn query_document_within_collection<T, Q>(
    collection: &CollectionClient,
    query: Q,
    cross_partition: bool,
) -> Option<Vec<T>>
where
    T: DeserializeOwned + Send + Sync + Clone,
    Q: Into<Query>,
{
    let documents: Option<Vec<T>> = collection
        .query_documents(query)
        .query_cross_partition(cross_partition)
        .into_stream::<T>()
        .collect::<Vec<_>>()
        .await
        .into_iter()
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| tracing::error!("Failed to retrieve document: {}", e))
        .ok()
        .and_then(|result| result.first().cloned())
        .map(|response| {
            response
                .results
                .into_iter()
                .map(|(data, _attrs)| data)
                .collect()
        });

    documents.filter(|document| !document.is_empty())
}

pub async fn add_document<S, D>(
    database: &DatabaseClient,
    collection_name: S,
    new_document: D,
) -> Result<CreateDocumentResponse, azure_core::error::Error>
where
    S: Into<std::borrow::Cow<'static, str>>,
    D: Serialize + CosmosEntity + Send + 'static,
{
    let collection = database.collection_client(collection_name);

    add_document_into_collection(&collection, new_document).await
}

pub async fn add_document_into_collection<D: Serialize + CosmosEntity + Send + 'static>(
    collection: &CollectionClient,
    new_document: D,
) -> Result<CreateDocumentResponse, azure_core::error::Error> {
    collection
        .create_document(new_document)
        .is_upsert(true)
        .into_future()
        .await
}

pub async fn adjust_credit(
    database: &DatabaseClient,
    user_id: String,
    request: UserCreditUpdateInfo,
    opt: UserCreditUpdateOpt,
) -> Response {
    let collection = database.collection_client(USER_CREDITS);
    adjust_credit_in_collection(&collection, user_id, request, opt).await
}

pub async fn adjust_credit_in_collection(
    credit_collection: &CollectionClient,
    user_id: String,
    request: UserCreditUpdateInfo,
    opt: UserCreditUpdateOpt,
) -> Response {
    let query = Query::with_params(
        "SELECT * FROM UserCredits u WHERE u.user_id = @user_id".into(),
        vec![Param::new("@user_id".into(), user_id)],
    );

    let query_result =
        query_document_within_collection::<UserCredit, _>(credit_collection, query, true).await;
    if query_result.is_none() {
        return (
            StatusCode::NOT_FOUND,
            Json(ServerError {
                error_message:
                    "Cannot update user's credit because the specified user doesn't exist.".into(),
            }),
        )
            .into_response();
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

    match add_document_into_collection(credit_collection, new_document.clone()).await {
        Ok(_) => (StatusCode::OK, Json(new_document)).into_response(),
        Err(e) => {
            let error_message = format!("{}", e);
            tracing::error!("{}", &error_message);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ServerError { error_message }),
            )
                .into_response()
        }
    }
}

pub fn initialize_clients() -> CosmosDb {
    let authorization_token = AuthorizationToken::primary_key(&CONFIGURATION.cosmos_db_primary_key)
        .map_err(|e| tracing::error!("Failed to generate authorization token for CosmosDB: {}", e))
        .expect("Failed to generate authorization token for CosmosDB.");

    let client = CosmosClient::new(CONFIGURATION.cosmos_db_account.clone(), authorization_token);

    let database = client.database_client(&CONFIGURATION.cosmos_db_database_name);
    CosmosDb { client, database }
}

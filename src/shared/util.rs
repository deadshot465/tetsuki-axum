use crate::CONFIGURATION;
use crate::controller::credit_controller::USER_CREDITS;
use crate::model::cosmos_db::CosmosDb;
use crate::model::errors::ServerError;
use crate::model::user_credit::{UserCredit, UserCreditUpdateInfo, UserCreditUpdateOpt};
use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use azure_data_cosmos::models::ItemResponse;
use azure_data_cosmos::options::{
    ContentResponseOnWrite, ItemWriteOptions, OperationOptions, Region,
};
use azure_data_cosmos::{
    AccountEndpoint, AccountReference, ContainerClient, CosmosClient, CosmosError, DatabaseClient,
    FeedScope, PartitionKey, Query, RoutingStrategy,
};
use futures::StreamExt;
use serde::Serialize;
use serde::de::DeserializeOwned;

pub async fn get_documents<T, S>(database: &DatabaseClient, container_name: S) -> Option<Vec<T>>
where
    T: DeserializeOwned + Send + Sync + Clone + 'static,
    S: Into<String>,
{
    let container_name = container_name.into();
    let res = database.container_client(&container_name, None).await;

    match res {
        Err(e) => {
            let error_message = format!("Failed to get container {}: {}", container_name, e);
            tracing::error!("{}", &error_message);
            None
        }
        Ok(container) => {
            match container
                .query_items::<T>(
                    format!("SELECT * FROM {}", container_name),
                    FeedScope::full_container(),
                    None,
                )
                .await
            {
                Err(e) => {
                    let error_message = format!("Failed to query items: {}", e);
                    tracing::error!("{}", &error_message);
                    None
                }
                Ok(iterator) => iterator
                    .collect::<Vec<_>>()
                    .await
                    .into_iter()
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(|e| tracing::error!("Failed to query items: {}", e))
                    .ok(),
            }
        }
    }
}

pub async fn query_document<T, S, Q>(
    database: &DatabaseClient,
    container_name: S,
    query: Q,
) -> Option<Vec<T>>
where
    T: DeserializeOwned + Send + Sync + Clone + 'static,
    S: Into<String>,
    Q: Into<Query>,
{
    let container_name = container_name.into();
    match database.container_client(&container_name, None).await {
        Err(e) => {
            tracing::error!("Failed to get container {}: {}", container_name, e);
            None
        }
        Ok(container) => query_document_within_container(&container, query).await,
    }
}

pub async fn query_document_within_container<T, Q>(
    container: &ContainerClient,
    query: Q,
) -> Option<Vec<T>>
where
    T: DeserializeOwned + Send + Sync + Clone + 'static,
    Q: Into<Query>,
{
    match container
        .query_items::<T>(query, FeedScope::full_container(), None)
        .await
    {
        Err(e) => {
            tracing::error!("Failed to query items: {}", e);
            None
        }
        Ok(iterator) => {
            let documents = iterator
                .collect::<Vec<_>>()
                .await
                .into_iter()
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| tracing::error!("Failed to query items: {}", e))
                .ok();

            documents.filter(|docs| !docs.is_empty())
        }
    }
}

pub async fn add_document<S, D>(
    database: &DatabaseClient,
    container_name: S,
    item_id: &str,
    new_document: D,
) -> Result<ItemResponse, CosmosError>
where
    S: Into<String>,
    D: Serialize + Send + 'static,
{
    match database.container_client(container_name.into(), None).await {
        Ok(container) => add_document_into_container(&container, item_id, new_document).await,
        Err(e) => Err(e),
    }
}

pub async fn add_document_into_container<D: Serialize + Send + 'static>(
    container: &ContainerClient,
    item_id: &str,
    new_document: D,
) -> Result<ItemResponse, CosmosError> {
    let mut options = OperationOptions::default();
    options.content_response_on_write = Some(ContentResponseOnWrite::Enabled);
    let item_write_options = ItemWriteOptions::default().with_operation_options(options);

    let partition_key = PartitionKey::from(item_id.to_string());

    container
        .upsert_item(
            partition_key,
            item_id,
            new_document,
            Some(item_write_options),
        )
        .await
}

pub async fn adjust_credit(
    database: &DatabaseClient,
    user_id: String,
    request: UserCreditUpdateInfo,
    opt: UserCreditUpdateOpt,
) -> Response {
    match database.container_client(USER_CREDITS, None).await {
        Err(e) => {
            let error_message = format!("Failed to adjust credit: {}", e);
            tracing::error!("{}", &error_message);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ServerError::with_message(error_message)),
            )
                .into_response()
        }
        Ok(container) => adjust_credit_in_collection(&container, user_id, request, opt).await,
    }
}

pub async fn adjust_credit_in_collection(
    credit_container: &ContainerClient,
    user_id: String,
    request: UserCreditUpdateInfo,
    opt: UserCreditUpdateOpt,
) -> Response {
    let query = Query::from("SELECT * FROM UserCredits u WHERE u.user_id = @user_id")
        .with_parameter("@user_id", user_id)
        .expect("Failed to build a query.");

    let query_result =
        query_document_within_container::<UserCredit, _>(credit_container, query).await;
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

    let item_id = new_document.id.as_str();

    match add_document_into_container(credit_container, item_id, new_document.clone()).await {
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

pub async fn initialize_clients() -> anyhow::Result<CosmosDb> {
    let endpoint = (CONFIGURATION
        .cosmos_db_endpoint
        .parse::<AccountEndpoint>()?)
    .clone();

    let account_reference = AccountReference::with_authentication_key(
        endpoint,
        CONFIGURATION.cosmos_db_primary_key.as_str(),
    );

    let client = CosmosClient::builder()
        .build(
            account_reference,
            RoutingStrategy::PreferredRegions(vec![
                Region::JAPAN_EAST,
                Region::JAPAN_WEST,
                Region::EAST_ASIA,
                Region::SOUTHEAST_ASIA,
                Region::EAST_EUROPE,
            ]),
        )
        .await?;

    Ok(CosmosDb { client })
}

use azure_data_cosmos::prelude::{CosmosClient, DatabaseClient};

#[derive(Clone)]
pub struct CosmosDb {
    pub client: CosmosClient,
    pub database: DatabaseClient,
}

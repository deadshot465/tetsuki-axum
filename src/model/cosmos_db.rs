use azure_data_cosmos::prelude::{CosmosClient, DatabaseClient};

pub struct CosmosDb {
    pub client: CosmosClient,
    pub database: DatabaseClient,
}

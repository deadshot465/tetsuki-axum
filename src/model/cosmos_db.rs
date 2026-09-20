use azure_data_cosmos::CosmosClient;

#[derive(Clone)]
pub struct CosmosDb {
    pub client: CosmosClient,
}

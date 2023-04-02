use crate::model::cosmos_db::CosmosDb;

#[derive(Clone)]
pub struct AppState {
    pub cosmos_db: CosmosDb,
}

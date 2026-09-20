use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub struct CodexSummaryRequest {
    pub keyword: String,
    pub word_count: i32,
    pub novel: Option<String>,
    pub additional_instructions: Option<String>,
}

#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub struct CodexSummaryResponse {
    pub output: String,
    pub image: Vec<u8>,
}

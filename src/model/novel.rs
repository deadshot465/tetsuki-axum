use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub struct CodexSummaryRequest {
    pub keyword: String,
    pub word_count: i32,
    pub novel: Option<String>,
    pub additional_instructions: Option<String>,
}

#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub struct CodexSummaryContainerResponse {
    pub container_id: String,
}

#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub struct CodexSummaryResponse {
    pub logs: String,
    pub image: Vec<u8>,
}

#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub struct CodexSummaryResponseLogs {
    pub errors: Vec<String>,
    pub outs: Vec<String>,
    pub console: Vec<String>,
    pub ins: Vec<String>,
}

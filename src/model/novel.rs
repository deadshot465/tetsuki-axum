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
pub struct CodexSummaryResponseLogs {
    pub errors: Vec<String>,
    pub outs: Vec<String>,
    pub console: Vec<String>,
    pub ins: Vec<String>,
    pub images: Vec<String>
}

#[derive(sqlx::FromRow, Deserialize, Serialize, Debug, Clone, Default, Eq, PartialEq, Hash)]
pub struct NovelEntityCardRecord {
    pub id: i32,
    pub image_path: String,
    pub language: String,
    pub entity_id: i32
}
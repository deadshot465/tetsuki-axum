use std::fmt::Display;

use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub struct CodexSummaryRequest {
    pub keyword: String,
    pub word_count: i32,
    pub novel: Option<String>,
    pub additional_instructions: Option<String>,
    pub request_language: CodexSummaryRequestedLanguage,
    pub push_to_line: bool,
    pub schedule_polling: bool,
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
    pub images: Vec<String>,
}

#[derive(sqlx::FromRow, Deserialize, Serialize, Debug, Clone, Default, Eq, PartialEq, Hash)]
pub struct NovelEntityCardRecord {
    pub id: i32,
    pub image_path: String,
    pub language: String,
    pub entity_id: i32,
}

#[derive(Deserialize, Serialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum CodexSummaryRequestedLanguage {
    #[default]
    ZhTw,
    JaJp,
    EnUs,
}

#[derive(Deserialize, Serialize, Debug, Clone, Default, Eq, PartialEq)]
pub struct CodexSummaryTrackItem {
    pub container_id: String,
    pub keyword: String,
    pub requested_language: CodexSummaryRequestedLanguage,
    pub push_to_line: bool,
}

impl Display for CodexSummaryRequestedLanguage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CodexSummaryRequestedLanguage::ZhTw => write!(f, "zh_tw"),
            CodexSummaryRequestedLanguage::JaJp => write!(f, "ja_jp"),
            CodexSummaryRequestedLanguage::EnUs => write!(f, "en_us"),
        }
    }
}

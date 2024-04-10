use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SaveFileRequest {
    pub filename: String,
    pub file_url: String,
}

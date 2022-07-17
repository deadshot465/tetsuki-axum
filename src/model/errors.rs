use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct ServerError {
    pub error_message: String,
}

impl ServerError {
    pub fn with_message<S: Into<String>>(message: S) -> Self {
        ServerError {
            error_message: message.into(),
        }
    }
}

impl From<String> for ServerError {
    fn from(str: String) -> Self {
        ServerError::with_message(str)
    }
}

impl From<&str> for ServerError {
    fn from(str: &str) -> Self {
        ServerError::with_message(str)
    }
}

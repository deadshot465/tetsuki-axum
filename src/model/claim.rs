use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct Claim {
    pub sub: String,
    pub exp: usize,
}

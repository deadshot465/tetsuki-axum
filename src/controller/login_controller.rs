use crate::model::claim::Claim;
use crate::model::login_info::{LoginCredential, LoginResponse};
use crate::shared::configuration::CONFIGURATION;
use actix_web::{post, HttpResponse, Responder};
use chrono::{Duration, SecondsFormat, Utc};
use jsonwebtoken::{encode, EncodingKey, Header};

#[post("/login")]
pub async fn login(request: actix_web::web::Json<LoginCredential>) -> impl Responder {
    let user_name = &CONFIGURATION.bot_user_name;
    let password = &CONFIGURATION.bot_user_pass;
    if user_name == &request.user_name && password == &request.password {
        let token = generate_jwt_token(user_name);
        let login_response = LoginResponse {
            token,
            expiry: (Utc::now() + Duration::hours(1)).to_rfc3339_opts(SecondsFormat::Micros, false),
        };
        HttpResponse::Ok().json(&login_response)
    } else {
        HttpResponse::Unauthorized().json("".to_string())
    }
}

fn generate_jwt_token(user_name: &str) -> String {
    let secret = &CONFIGURATION.jwt_secret;
    let claim = Claim {
        sub: user_name.into(),
        exp: (Utc::now() + Duration::hours(1)).timestamp() as usize,
    };
    encode(
        &Header::default(),
        &claim,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .expect("Failed to encode JWT token.")
}

use crate::model::claim::Claim;
use crate::model::login_info::{LoginCredential, LoginResponse};
use crate::shared::configuration::CONFIGURATION;
use actix_web::{post, HttpResponse, Responder};
use jsonwebtoken::{encode, EncodingKey, Header};
use std::ops::Add;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

#[post("/login")]
pub async fn login(request: actix_web::web::Json<LoginCredential>) -> impl Responder {
    let user_name = &CONFIGURATION.bot_user_name;
    let password = &CONFIGURATION.bot_user_pass;
    if user_name == &request.user_name && password == &request.password {
        let token = generate_jwt_token(user_name);
        let expiry = OffsetDateTime::now_utc().add(time::Duration::hours(1));
        let login_response = LoginResponse {
            token,
            expiry: expiry.format(&Rfc3339).unwrap_or_default(),
        };
        HttpResponse::Ok().json(&login_response)
    } else {
        HttpResponse::Unauthorized().json("".to_string())
    }
}

fn generate_jwt_token(user_name: &str) -> String {
    let timestamp = OffsetDateTime::now_utc()
        .add(time::Duration::hours(1))
        .unix_timestamp();
    let secret = &CONFIGURATION.jwt_secret;
    let claim = Claim {
        sub: user_name.into(),
        exp: timestamp as usize,
    };
    encode(
        &Header::default(),
        &claim,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .expect("Failed to encode JWT token.")
}

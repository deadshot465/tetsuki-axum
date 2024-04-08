use crate::model::claim::Claim;
use crate::model::login_info::{LoginCredential, LoginResponse};
use crate::shared::configuration::CONFIGURATION;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use jsonwebtoken::{encode, EncodingKey, Header};
use std::ops::Add;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

pub async fn login(Json(login_credential): Json<LoginCredential>) -> Response {
    let user_name = &CONFIGURATION.bot_user_name;
    let password = &CONFIGURATION.bot_user_pass;
    if user_name == &login_credential.user_name && password == &login_credential.password {
        let token = generate_jwt_token(user_name);
        let expiry = OffsetDateTime::now_utc().add(time::Duration::hours(1));
        let login_response = LoginResponse {
            token,
            expiry: expiry.format(&Rfc3339).unwrap_or_default(),
        };
        (StatusCode::OK, Json(login_response)).into_response()
    } else {
        StatusCode::UNAUTHORIZED.into_response()
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

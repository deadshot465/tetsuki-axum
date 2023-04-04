#![allow(clippy::type_complexity)]

use crate::model::claim::Claim;
use crate::model::errors::{ApiError, ServerError};
use crate::shared::configuration::CONFIGURATION;
use axum::extract::FromRequestParts;
use axum::headers::authorization::Bearer;
use axum::headers::Authorization;
use axum::http::request::Parts;
use axum::http::StatusCode;
use axum::{async_trait, Json, RequestPartsExt, TypedHeader};
use jsonwebtoken::{decode, DecodingKey, Validation};
use time::OffsetDateTime;

#[async_trait]
impl<S> FromRequestParts<S> for Claim {
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let TypedHeader(Authorization(bearer)) = parts
            .extract::<TypedHeader<Authorization<Bearer>>>()
            .await
            .map_err(|_e| {
                (
                    StatusCode::UNAUTHORIZED,
                    Json(ServerError {
                        error_message: "Authorization header not found".to_string(),
                    }),
                )
            })?;

        let secret = &CONFIGURATION.jwt_secret;

        if let Ok(token) = decode::<Claim>(
            bearer.token(),
            &DecodingKey::from_secret(secret.as_bytes()),
            &Validation::default(),
        ) {
            tracing::info!("{:?}", &token.claims);
            match OffsetDateTime::from_unix_timestamp(token.claims.exp as i64) {
                Ok(expiry) => {
                    if expiry > OffsetDateTime::now_utc() {
                        Ok(token.claims)
                    } else {
                        Err((
                            StatusCode::UNAUTHORIZED,
                            Json(ServerError {
                                error_message: "Token expired".to_string(),
                            }),
                        ))
                    }
                }
                Err(e) => {
                    tracing::error!(
                        "Failed to retrieve token expiration data from timestamp: {}",
                        e
                    );
                    Err((
                        StatusCode::BAD_REQUEST,
                        Json(ServerError {
                            error_message:
                                "Failed to retrieve token expiration data from timestamp."
                                    .to_string(),
                        }),
                    ))
                }
            }
        } else {
            Err((
                StatusCode::UNAUTHORIZED,
                Json(ServerError {
                    error_message: "Unauthorized".to_string(),
                }),
            ))
        }
    }
}

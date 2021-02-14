use crate::model::Claim;
use actix_service::{Service, Transform};
use actix_web::dev::{ServiceRequest, ServiceResponse};
use actix_web::{Error, HttpResponse};
use futures::prelude::future::{ok, Ready};
use futures::Future;
use jsonwebtoken::{decode, DecodingKey, Validation};
use once_cell::sync::OnceCell;
use std::pin::Pin;
use std::task::{Context, Poll};

static ANONYMOUS_ENDPOINTS: OnceCell<Vec<String>> = OnceCell::new();

pub struct Authentication;

impl<S> Transform<S, ServiceRequest> for Authentication
where
    S: Service<ServiceRequest, Error = Error, Response = ServiceResponse>,
    S::Future: 'static,
{
    type Response = ServiceResponse;
    type Error = Error;
    type Transform = AuthenticationMiddleware<S>;
    type InitError = ();
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ok(AuthenticationMiddleware { service })
    }
}

pub struct AuthenticationMiddleware<S> {
    service: S,
}

impl<S> Service<ServiceRequest> for AuthenticationMiddleware<S>
where
    S: Service<ServiceRequest, Error = Error, Response = ServiceResponse>,
    S::Future: 'static,
{
    type Response = ServiceResponse;
    type Error = Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>>>>;

    fn poll_ready(&self, ctx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.service.poll_ready(ctx)
    }

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let mut authentication_pass = false;
        let anonymous_endpoints = ANONYMOUS_ENDPOINTS.get_or_init(|| {
            let raw_data = std::fs::read("asset/anonymous_endpoints.json")
                .expect("Failed to read anonymous endpoints from assets.");
            serde_json::from_slice(&raw_data).expect("Failed to deserialize JSON file.")
        });
        let path = req.path();
        if anonymous_endpoints.iter().any(|s| path.starts_with(&*s)) {
            authentication_pass = true;
        } else if let Some(header) = req.headers().get("Authorization") {
            let header_value = header.to_str().unwrap_or("");
            if header_value.starts_with("Bearer") {
                let token = header_value[6..].trim();
                let secret = dotenv::var("JWT_SECRET").unwrap_or_default();
                if let Ok(token) = decode::<Claim>(
                    token,
                    &DecodingKey::from_secret(secret.as_bytes()),
                    &Validation::default(),
                ) {
                    println!("{:?}", &token.claims);
                    authentication_pass = true;
                }
            }
        }

        if authentication_pass {
            let future = self.service.call(req);
            Box::pin(async move {
                let response = future.await?;
                Ok(response)
            })
        } else {
            Box::pin(async move { Ok(req.into_response(HttpResponse::Unauthorized())) })
        }
    }
}

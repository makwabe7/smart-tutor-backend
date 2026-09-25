use crate::handlers::auth::Claims;
use actix_web::{Error, FromRequest, HttpRequest, dev::Payload, error::ErrorUnauthorized};
use futures_util::future::{Ready, ready};
use jsonwebtoken::{DecodingKey, Validation, decode};
use std::env;

#[derive(Clone, Debug)]
pub struct AuthUser {
    pub user_id: String,
    pub role: String,
}
impl FromRequest for AuthUser {
    type Error = Error;
    type Future = Ready<Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, _: &mut Payload) -> Self::Future {
        let authorization = match req.headers().get("Authorization") {
            Some(value) => value,
            None => {
                return ready(Err(ErrorUnauthorized("no authorization header")));
            }
        };
        let authorization = match authorization.to_str() {
            Ok(value) => value,
            Err(_) => {
                return ready(Err(ErrorUnauthorized("Missing JWT_SECRET")));
            }
        };
        let token = match authorization.strip_prefix("Bearer") {
            Some(token) if !token.trim().is_empty() => token.trim(),
            _ => {
                return ready(Err(ErrorUnauthorized("Invalid Authorization header")));
            }
        };
        let secret = match env::var("JWT_SECRET") {
            Ok(secret) => secret,

            Err(_) => {
                return ready(Err(ErrorUnauthorized("JWT configuration missing")));
            }
        };
        let token_data = match decode::<Claims>(
            token,
            &DecodingKey::from_secret(secret.as_bytes()),
            &Validation::default(),
        ) {
            Ok(data) => data,
            Err(error) => {
                println!("JWT validation error: {:?}", error);
                return ready(Err(ErrorUnauthorized("Invalid or expired Token")));
            }
        };
        ready(Ok(AuthUser {
            user_id: token_data.claims.sub,
            role: token_data.claims.role,
        }))
    }
}

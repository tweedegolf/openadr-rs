use std::sync::Arc;

use axum::{
    async_trait,
    extract::{FromRef, FromRequestParts},
    http::request::Parts,
};
use axum_extra::{
    headers::{authorization::Bearer, Authorization},
    TypedHeader,
};
use jsonwebtoken::{encode, DecodingKey, EncodingKey, Header};
use tracing::trace;

use crate::error::AppError;

pub struct JwtManager {
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[allow(clippy::upper_case_acronyms)]
pub enum AuthRole {
    BL,
    VEN,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct Claims {
    exp: usize,
    nbf: usize,
    pub sub: String,
    pub role: AuthRole,
    pub ven: Option<String>,
}

impl JwtManager {
    /// Create a new JWT manager from a base64 encoded secret
    pub fn from_base64_secret(secret: &str) -> Result<Self, jsonwebtoken::errors::Error> {
        let encoding_key = EncodingKey::from_base64_secret(secret)?;
        let decoding_key = DecodingKey::from_base64_secret(secret)?;
        Ok(Self::new(encoding_key, decoding_key))
    }

    /// Create a new JWT manager from some secret bytes
    pub fn from_secret(secret: &[u8]) -> Self {
        let encoding_key = EncodingKey::from_secret(secret);
        let decoding_key = DecodingKey::from_secret(secret);
        Self::new(encoding_key, decoding_key)
    }

    /// Create a new JWT manager with a specific encoding and decoding key
    pub fn new(encoding_key: EncodingKey, decoding_key: DecodingKey) -> Self {
        Self {
            encoding_key,
            decoding_key,
        }
    }

    /// Create a new JWT token with the given claims and expiration time
    pub fn create(
        &self,
        expires_in: std::time::Duration,
        client_id: String,
        role: AuthRole,
        ven: Option<String>,
    ) -> Result<String, jsonwebtoken::errors::Error> {
        let now = chrono::Utc::now();
        let exp = now + expires_in;

        let claims = Claims {
            exp: exp.timestamp() as usize,
            nbf: now.timestamp() as usize,
            sub: client_id,
            role,
            ven,
        };

        let token = encode(&Header::default(), &claims, &self.encoding_key)?;

        Ok(token)
    }

    /// Decode and validate a given JWT token, returning the validated claims
    pub fn decode_and_validate(&self, token: &str) -> Result<Claims, jsonwebtoken::errors::Error> {
        let validation = jsonwebtoken::Validation::default();
        let token_data = jsonwebtoken::decode::<Claims>(token, &self.decoding_key, &validation)?;
        Ok(token_data.claims)
    }
}

pub struct User(pub Claims);
pub struct BLUser(pub Claims);
// pub struct VENUser(pub Claims);

#[async_trait]
impl<S: Send + Sync> FromRequestParts<S> for User
where
    Arc<JwtManager>: FromRef<S>,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let Ok(bearer) =
            TypedHeader::<Authorization<Bearer>>::from_request_parts(parts, state).await
        else {
            return Err(AppError::Auth(
                "Authorization via Bearer token in Authorization header required".to_string(),
            ));
        };

        let jwt_manager = Arc::<JwtManager>::from_ref(state);

        let Ok(claims) = jwt_manager.decode_and_validate(bearer.0.token()) else {
            return Err(AppError::Auth(
                "Invalid authentication token provided".to_string(),
            ));
        };

        trace!(user = ?claims, "Extracted User from request");

        Ok(User(claims))
    }
}

#[async_trait]
impl<S: Send + Sync> FromRequestParts<S> for BLUser
where
    Arc<JwtManager>: FromRef<S>,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let user = User::from_request_parts(parts, state).await?;

        if user.0.role != AuthRole::BL {
            return Err(AppError::Auth(
                "User does not have the required role".to_string(),
            ));
        }

        Ok(BLUser(user.0))
    }
}

// #[async_trait]
// impl<S: Send + Sync> FromRequestParts<S> for VENUser where Arc<JwtManager>: FromRef<S> {
//     type Rejection = AppError;

//     async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
//         let user = User::from_request_parts(parts, state).await?;

//         if user.0.role != AuthRole::VEN {
//             return Err(AppError::Auth("User does not have the required role".to_string()));
//         }

//         Ok(VENUser(user.0))
//     }
// }

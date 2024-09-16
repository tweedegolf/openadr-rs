use crate::error::AppError;
use axum::extract::rejection::JsonRejection;
use axum::extract::{FromRequest, FromRequestParts, Request};
use axum::{async_trait, Json};
use axum_extra::extract::{Query, QueryRejection};
use serde::de::DeserializeOwned;
use validator::Validate;

pub mod auth;
pub mod event;
pub mod program;
pub mod report;

pub type AppResponse<T> = Result<Json<T>, AppError>;

#[derive(Debug, Clone)]
pub struct ValidatedQuery<T>(pub T);

#[derive(Debug, Clone)]
pub struct ValidatedJson<T>(pub T);

#[async_trait]
impl<T, S> FromRequest<S> for ValidatedJson<T>
where
    T: DeserializeOwned + Validate,
    S: Send + Sync,
    Json<T>: FromRequest<S, Rejection = JsonRejection>,
{
    type Rejection = AppError;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        let Json(value) = Json::<T>::from_request(req, state).await?;
        value.validate()?;
        Ok(ValidatedJson(value))
    }
}

#[async_trait]
impl<T, S> FromRequestParts<S> for ValidatedQuery<T>
where
    T: DeserializeOwned + Validate,
    S: Send + Sync,
    Query<T>: FromRequestParts<S, Rejection = QueryRejection>,
{
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        state: &S,
    ) -> Result<Self, Self::Rejection> {
        let Query(value) = Query::<T>::from_request_parts(parts, state).await?;
        value.validate()?;
        Ok(ValidatedQuery(value))
    }
}

#[cfg(test)]
mod test {
    use crate::jwt::AuthRole;
    use crate::state::AppState;

    #[allow(dead_code)]
    pub(crate) fn jwt_test_token(state: &AppState, roles: Vec<AuthRole>) -> String {
        state
            .jwt_manager
            .create(
                std::time::Duration::from_secs(60),
                "test_admin".to_string(),
                roles,
            )
            .unwrap()
    }
}

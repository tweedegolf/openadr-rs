use crate::error::AppError;
use axum::extract::rejection::{JsonRejection, QueryRejection};
use axum::extract::{FromRequest, Query, Request};
use axum::{async_trait, Json};
use serde::de::DeserializeOwned;
use validator::Validate;

mod event;
mod program;
mod report;

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
impl<T, S> FromRequest<S> for ValidatedQuery<T>
where
    T: DeserializeOwned + Validate,
    S: Send + Sync,
    Query<T>: FromRequest<S, Rejection = QueryRejection>,
{
    type Rejection = AppError;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        let Query(value) = Query::<T>::from_request(req, state).await?;
        value.validate()?;
        Ok(ValidatedQuery(value))
    }
}

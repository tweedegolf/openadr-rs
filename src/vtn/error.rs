use axum::extract::rejection::JsonRejection;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum_extra::extract::QueryRejection;
use tracing::{error, trace, warn};
use uuid::Uuid;

use openadr::wire::Problem;

use crate::data_source;
use crate::data_source::Error;

#[derive(thiserror::Error, Debug)]
pub enum AppError {
    #[error("Invalid request: {0}")]
    Validation(#[from] validator::ValidationErrors),
    #[error("Invalid request: {0}")]
    Json(#[from] JsonRejection),
    #[error("Invalid request: {0}")]
    QueryParams(#[from] QueryRejection),
    #[error("Object not found")]
    NotFound,
    #[error("Bad request: {0}")]
    BadRequest(&'static str),
    #[error("Not implemented {0}")]
    NotImplemented(&'static str),
    #[error("Conflict: {0}")]
    Conflict(String),
    #[error("Database error: {0}")]
    Sql(sqlx::Error),
    #[error("Json (de)serialization error : {0}")]
    SerdeJson(#[from] serde_json::Error),
}

impl From<sqlx::Error> for AppError {
    fn from(err: sqlx::Error) -> Self {
        match err {
            sqlx::Error::RowNotFound => Self::NotFound,
            sqlx::Error::Database(err) if err.is_unique_violation() => {
                trace!(?err);
                Self::Conflict("Conflict".to_string())
            }
            _ => Self::Sql(err),
        }
    }
}

impl From<data_source::Error> for AppError {
    fn from(err: data_source::Error) -> Self {
        match err {
            Error::Sqlx(e) => e.into(),
            Error::Json(e) => e.into(),
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let reference = Uuid::new_v4();

        match self {
            AppError::Validation(err) => {
                trace!(
                    "Error reference: {}, Received invalid request: {}",
                    reference,
                    err
                );
                Problem {
                    r#type: Default::default(),
                    title: Some(StatusCode::BAD_REQUEST.to_string()),
                    status: StatusCode::BAD_REQUEST,
                    detail: Some(err.to_string()),
                    instance: Some(reference.to_string()),
                }
            }
            AppError::Json(err) => {
                trace!(
                    "Error reference: {}, Received invalid JSON in request: {}",
                    reference,
                    err
                );
                Problem {
                    r#type: Default::default(),
                    title: Some(StatusCode::BAD_REQUEST.to_string()),
                    status: StatusCode::BAD_REQUEST,
                    detail: Some(err.to_string()),
                    instance: Some(reference.to_string()),
                }
            }
            AppError::QueryParams(err) => {
                trace!(
                    "Error reference: {}, Received invalid query parameters: {}",
                    reference,
                    err
                );
                Problem {
                    r#type: Default::default(),
                    title: Some(StatusCode::BAD_REQUEST.to_string()),
                    status: StatusCode::BAD_REQUEST,
                    detail: Some(err.to_string()),
                    instance: Some(reference.to_string()),
                }
            }
            AppError::NotFound => {
                trace!("Error reference: {}, Object not found", reference,);
                Problem {
                    r#type: Default::default(),
                    title: Some(StatusCode::NOT_FOUND.to_string()),
                    status: StatusCode::NOT_FOUND,
                    detail: None,
                    instance: Some(reference.to_string()),
                }
            }
            AppError::BadRequest(err) => {
                trace!(
                    "Error reference: {}, Received invalid request: {}",
                    reference,
                    err
                );
                Problem {
                    r#type: Default::default(),
                    title: Some(StatusCode::BAD_REQUEST.to_string()),
                    status: StatusCode::BAD_REQUEST,
                    detail: Some(err.to_string()),
                    instance: Some(reference.to_string()),
                }
            }
            AppError::NotImplemented(err) => {
                error!("Error reference: {}, Not implemented: {}", reference, err);
                Problem {
                    r#type: Default::default(),
                    title: Some(StatusCode::NOT_IMPLEMENTED.to_string()),
                    status: StatusCode::NOT_IMPLEMENTED,
                    detail: Some(err.to_string()),
                    instance: Some(reference.to_string()),
                }
            }
            AppError::Conflict(err) => {
                warn!("Error reference: {}, Conflict: {}", reference, err);
                Problem {
                    r#type: Default::default(),
                    title: Some(StatusCode::CONFLICT.to_string()),
                    status: StatusCode::CONFLICT,
                    detail: Some(err.to_string()),
                    instance: Some(reference.to_string()),
                }
            }
            AppError::Sql(err) => {
                error!("Error reference: {}, SQL error: {}", reference, err);
                Problem {
                    r#type: Default::default(),
                    title: Some(StatusCode::INTERNAL_SERVER_ERROR.to_string()),
                    status: StatusCode::INTERNAL_SERVER_ERROR,
                    detail: Some(err.to_string()),
                    instance: Some(reference.to_string()),
                }
            }
            AppError::SerdeJson(err) => {
                trace!("Error reference: {}, serde json error: {}", reference, err);
                Problem {
                    r#type: Default::default(),
                    title: Some(StatusCode::BAD_REQUEST.to_string()),
                    status: StatusCode::BAD_REQUEST,
                    detail: Some(err.to_string()),
                    instance: Some(reference.to_string()),
                }
            }
        }
        .into_response()
    }
}

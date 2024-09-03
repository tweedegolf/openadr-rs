use axum::extract::rejection::JsonRejection;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use axum_extra::extract::QueryRejection;
use openadr_wire::problem::Problem;
use openadr_wire::IdentifierError;
use serde::{Deserialize, Serialize};
#[cfg(feature = "sqlx")]
use sqlx::error::DatabaseError;
use tracing::{error, trace, warn};
use uuid::Uuid;

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
    #[cfg(feature = "sqlx")]
    #[error("Conflict: {0}")]
    Conflict(String, Option<Box<dyn DatabaseError>>),
    #[cfg(not(feature = "sqlx"))]
    #[error("Conflict: {0}")]
    Conflict(String),
    #[cfg(feature = "sqlx")]
    #[error("Unprocessable Content: {0}")]
    UnprocessableContent(String, Option<Box<dyn DatabaseError>>),
    #[error("Authentication error: {0}")]
    Auth(String),
    #[cfg(feature = "sqlx")]
    #[error("Database error: {0}")]
    Sql(sqlx::Error),
    #[cfg(feature = "sqlx")]
    #[error("Json (de)serialization error : {0}")]
    SerdeJsonInternalServerError(serde_json::Error),
    #[cfg(feature = "sqlx")]
    #[error("Json (de)serialization error : {0}")]
    SerdeJsonBadRequest(serde_json::Error),
    #[error("Malformed Identifier")]
    Identifier(#[from] IdentifierError),
}

#[cfg(feature = "sqlx")]
impl From<sqlx::Error> for AppError {
    fn from(err: sqlx::Error) -> Self {
        match err {
            sqlx::Error::RowNotFound => Self::NotFound,
            sqlx::Error::Database(err) if err.is_unique_violation() => {
                Self::Conflict("Conflict".to_string(), Some(err))
            }
            sqlx::Error::Database(err) if err.is_foreign_key_violation() => {
                Self::UnprocessableContent(
                    "A foreign key constraint is violated".to_string(),
                    Some(err),
                )
            }
            _ => Self::Sql(err),
        }
    }
}

impl AppError {
    fn into_problem(self) -> Problem {
        let reference = Uuid::new_v4();

        match self {
            AppError::Validation(err) => {
                trace!(%reference,
                    "Received invalid request: {}",
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
                trace!(%reference,
                    "Received invalid JSON in request: {}",
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
                trace!(%reference,
                    "Received invalid query parameters: {}",
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
                trace!(%reference, "Object not found");
                Problem {
                    r#type: Default::default(),
                    title: Some(StatusCode::NOT_FOUND.to_string()),
                    status: StatusCode::NOT_FOUND,
                    detail: None,
                    instance: Some(reference.to_string()),
                }
            }
            AppError::BadRequest(err) => {
                trace!(%reference,
                    "Received invalid request: {}",
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
                error!(%reference, "Not implemented: {}", err);
                Problem {
                    r#type: Default::default(),
                    title: Some(StatusCode::NOT_IMPLEMENTED.to_string()),
                    status: StatusCode::NOT_IMPLEMENTED,
                    detail: Some(err.to_string()),
                    instance: Some(reference.to_string()),
                }
            }
            #[cfg(feature = "sqlx")]
            AppError::Conflict(err, db_err) => {
                warn!(%reference, "Conflict: {}, DB err: {:?}", err, db_err);
                Problem {
                    r#type: Default::default(),
                    title: Some(StatusCode::CONFLICT.to_string()),
                    status: StatusCode::CONFLICT,
                    detail: Some(err.to_string()),
                    instance: Some(reference.to_string()),
                }
            }
            #[cfg(not(feature = "sqlx"))]
            AppError::Conflict(err) => {
                warn!(%reference, "Conflict: {}", err);
                Problem {
                    r#type: Default::default(),
                    title: Some(StatusCode::CONFLICT.to_string()),
                    status: StatusCode::CONFLICT,
                    detail: Some(err.to_string()),
                    instance: Some(reference.to_string()),
                }
            }
            AppError::Auth(err) => {
                trace!(%reference,
                    "Authentication error: {}",
                    err
                );
                Problem {
                    r#type: Default::default(),
                    title: Some(StatusCode::UNAUTHORIZED.to_string()),
                    status: StatusCode::UNAUTHORIZED,
                    detail: Some(err.to_string()),
                    instance: Some(reference.to_string()),
                }
            }
            #[cfg(feature = "sqlx")]
            AppError::Sql(err) => {
                error!(%reference, "SQL error: {}", err);
                Problem {
                    r#type: Default::default(),
                    title: Some(StatusCode::INTERNAL_SERVER_ERROR.to_string()),
                    status: StatusCode::INTERNAL_SERVER_ERROR,
                    detail: Some("A database error occurred".to_string()),
                    instance: Some(reference.to_string()),
                }
            }
            #[cfg(feature = "sqlx")]
            AppError::SerdeJsonInternalServerError(err) => {
                trace!(%reference, "serde json error: {}", err);
                Problem {
                    r#type: Default::default(),
                    title: Some(StatusCode::INTERNAL_SERVER_ERROR.to_string()),
                    status: StatusCode::INTERNAL_SERVER_ERROR,
                    detail: Some(err.to_string()),
                    instance: Some(reference.to_string()),
                }
            }
            #[cfg(feature = "sqlx")]
            AppError::SerdeJsonBadRequest(err) => {
                trace!(%reference, "serde json error: {}", err);
                Problem {
                    r#type: Default::default(),
                    title: Some(StatusCode::BAD_REQUEST.to_string()),
                    status: StatusCode::BAD_REQUEST,
                    detail: Some(err.to_string()),
                    instance: Some(reference.to_string()),
                }
            }
            AppError::Identifier(err) => {
                trace!(%reference,
                    "Malformed identifier: {}",
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
            #[cfg(feature = "sqlx")]
            AppError::UnprocessableContent(err, db_err) => {
                trace!(%reference,
                    "Unprocessable Content: {}, DB details: {:?}",
                    err,
                    db_err
                );
                Problem {
                    r#type: Default::default(),
                    title: Some(StatusCode::UNPROCESSABLE_ENTITY.to_string()),
                    status: StatusCode::UNPROCESSABLE_ENTITY,
                    detail: Some(err.to_string()),
                    instance: Some(reference.to_string()),
                }
            }
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let problem = self.into_problem();
        (problem.status, Json(problem)).into_response()
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct ProblemUri(String);

impl Default for ProblemUri {
    fn default() -> Self {
        Self("about:blank".to_string())
    }
}

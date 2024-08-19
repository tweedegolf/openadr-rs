use axum::extract::rejection::JsonRejection;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use axum_extra::extract::QueryRejection;
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;
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
    #[error("Conflict: {0}")]
    Conflict(String),
    #[error("Authentication error: {0}")]
    Auth(String),
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
            AppError::Auth(err) => {
                trace!(
                    "Error reference: {}, Authentication error: {}",
                    reference,
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
        }
        .into_response()
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct ProblemUri(String);

impl Default for ProblemUri {
    fn default() -> Self {
        Self("about:blank".to_string())
    }
}

/// Reusable error response. From <https://opensource.zalando.com/problem/schema.yaml>.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
#[skip_serializing_none]
#[serde(rename_all = "camelCase")]
pub struct Problem {
    /// An absolute URI that identifies the problem type.
    /// When dereferenced, it SHOULD provide human-readable documentation for the problem type
    /// (e.g., using HTML).
    #[serde(default)]
    pub r#type: ProblemUri,
    /// A short, summary of the problem type.
    /// Written in english and readable for engineers
    /// (usually not suited for non-technical stakeholders and not localized);
    /// example: Service Unavailable.
    pub title: Option<String>,
    /// The HTTP status code generated by the origin server for this occurrence of the problem.
    #[serde(with = "status_code_serialization")]
    pub status: StatusCode,
    /// A human-readable explanation specific to this occurrence of the problem.
    pub detail: Option<String>,
    /// An absolute URI that identifies the specific occurrence of the problem.
    /// It may or may not yield further information if dereferenced.
    pub instance: Option<String>,
}

mod status_code_serialization {
    use reqwest::StatusCode;
    use serde::de::Unexpected;
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(code: &StatusCode, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_u16(code.as_u16())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<StatusCode, D::Error>
    where
        D: Deserializer<'de>,
    {
        u16::deserialize(deserializer).and_then(|code| {
            StatusCode::from_u16(code).map_err(|_| {
                serde::de::Error::invalid_value(
                    Unexpected::Unsigned(code as u64),
                    &"Valid http status code",
                )
            })
        })
    }
}

impl IntoResponse for Problem {
    fn into_response(self) -> Response {
        (self.status, Json(self)).into_response()
    }
}

/// Errors that can occur using the [`Client`](crate::Client)
#[derive(Debug)]
pub enum Error {
    Reqwest(reqwest::Error),
    Serde(serde_json::Error),
    UrlParseError(url::ParseError),
    Problem(crate::wire::Problem),
    ObjectNotFound,
    DuplicateObject,
    InvalidParentObject,
    InvalidInterval,
}

impl From<reqwest::Error> for Error {
    fn from(err: reqwest::Error) -> Self {
        Error::Reqwest(err)
    }
}

impl From<serde_json::Error> for Error {
    fn from(err: serde_json::Error) -> Self {
        Error::Serde(err)
    }
}

impl From<url::ParseError> for Error {
    fn from(err: url::ParseError) -> Self {
        Error::UrlParseError(err)
    }
}

impl From<crate::wire::Problem> for Error {
    fn from(err: crate::wire::Problem) -> Self {
        Error::Problem(err)
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Error::Reqwest(err) => write!(f, "Reqwest error: {}", err),
            Error::Serde(err) => write!(f, "Serde error: {}", err),
            Error::UrlParseError(err) => write!(f, "URL parse error: {}", err),
            Error::Problem(err) => write!(f, "OpenADR Problem: {:?}", err),
            Error::ObjectNotFound => write!(f, "Object not found"),
            Error::DuplicateObject => write!(f, "Found more than one object matching the filter"),
            Error::InvalidParentObject => write!(f, "Invalid parent object"),
            Error::InvalidInterval => write!(f, "Invalid interval specified"),
        }
    }
}

impl std::error::Error for Error {}

pub(crate) type Result<T> = std::result::Result<T, Error>;

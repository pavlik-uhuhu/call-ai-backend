use std::fmt;
use std::sync::Arc;

use axum::{
    body::Body,
    response::{IntoResponse, Response},
    Json,
};
use http::StatusCode;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ErrorKind {
    DbQueryFailed,
    DeserializationFailed,
    EntityNotFound,
    SerializationFailed,
    TaskAlreadyProcessing,
    FileAlredyExists,
    AMQPError,
    CalcMetricsFailed,
    InvalidSettingsRequest,
    WorkerRequestFailed,
}

impl fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl From<ErrorKind> for StatusCode {
    fn from(k: ErrorKind) -> Self {
        match k {
            ErrorKind::DbQueryFailed => StatusCode::UNPROCESSABLE_ENTITY,
            ErrorKind::EntityNotFound => StatusCode::NOT_FOUND,
            ErrorKind::TaskAlreadyProcessing => StatusCode::BAD_REQUEST,
            ErrorKind::FileAlredyExists => StatusCode::BAD_REQUEST,
            ErrorKind::InvalidSettingsRequest => StatusCode::BAD_REQUEST,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Error {
    pub kind: ErrorKind,
    err: Option<Arc<anyhow::Error>>,
}

impl Error {
    pub fn new(kind: ErrorKind, err: anyhow::Error) -> Self {
        Self {
            kind,
            err: Some(Arc::new(err)),
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.err {
            Some(err) => write!(f, "{}: {err}", self.kind),
            None => write!(f, "{}", self.kind),
        }
    }
}

impl From<sqlx::Error> for Error {
    fn from(value: sqlx::Error) -> Self {
        Self {
            kind: ErrorKind::DbQueryFailed,
            err: Some(Arc::new(anyhow::anyhow!(value))),
        }
    }
}

impl From<ErrorKind> for Error {
    fn from(kind: ErrorKind) -> Self {
        Self { kind, err: None }
    }
}

impl IntoResponse for Error {
    fn into_response(self) -> Response<Body> {
        let status: StatusCode = self.kind.into();
        let payload = serde_json::json!({"error_detail": self.to_string()});

        (status, Json(&payload)).into_response()
    }
}

////////////////////////////////////////////////////////////////////////////////

pub trait ErrorExt<T> {
    fn error(self, kind: ErrorKind) -> Result<T, Error>;
}

impl<T, E> ErrorExt<T> for Result<T, E>
where
    E: Into<anyhow::Error>,
{
    fn error(self, kind: ErrorKind) -> Result<T, Error> {
        self.map_err(|err| Error {
            kind,
            err: Some(Arc::new(err.into())),
        })
    }
}

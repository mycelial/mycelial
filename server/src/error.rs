use arrow::error::ArrowError;
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use sqlx::migrate::MigrateError;

// TODO: figure out this error stuff, I just copied and pasted this for now.
#[derive(Debug)]
pub enum Error {
    // status code wrap, probably not needed
    StatusCode(StatusCode),

    // sqlx migration error
    SqlxMigration(MigrateError),

    // sqlx error
    Sqlx(sqlx::Error),

    // core didn't respond to message
    CoreRecv,

    // failed to send message to core
    CoreSend,

    //
    Io(std::io::Error),

    // axum error wrap
    Axum(axum::Error),

    // serde error wrap
    SerdeJson(serde_json::Error),

    // &'static str error
    Str(&'static str),

    // Arrow Error
    Arrow(ArrowError),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::SqlxMigration(e) => Some(e),
            Error::Sqlx(e) => Some(e),
            Error::Io(e) => Some(e),
            Error::Axum(e) => Some(e),
            _ => None,
        }
    }
}

impl From<StatusCode> for Error {
    fn from(s: StatusCode) -> Self {
        Self::StatusCode(s)
    }
}

impl From<MigrateError> for Error {
    fn from(e: MigrateError) -> Self {
        Self::SqlxMigration(e)
    }
}

impl From<sqlx::Error> for Error {
    fn from(e: sqlx::Error) -> Self {
        Self::Sqlx(e)
    }
}

impl From<tokio::sync::oneshot::error::RecvError> for Error {
    fn from(_: tokio::sync::oneshot::error::RecvError) -> Self {
        Self::CoreRecv
    }
}

impl<T> From<tokio::sync::mpsc::error::SendError<T>> for Error {
    fn from(_: tokio::sync::mpsc::error::SendError<T>) -> Self {
        Self::CoreSend
    }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

impl From<axum::Error> for Error {
    fn from(e: axum::Error) -> Self {
        Self::Axum(e)
    }
}

impl From<serde_json::Error> for Error {
    fn from(e: serde_json::Error) -> Self {
        Self::SerdeJson(e)
    }
}

impl From<&'static str> for Error {
    fn from(e: &'static str) -> Self {
        Self::Str(e)
    }
}

impl From<ArrowError> for Error {
    fn from(e: ArrowError) -> Self {
        Self::Arrow(e)
    }
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        let mut response: Response = match self {
            Self::StatusCode(s) => s,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        }
        .into_response();
        response.extensions_mut().insert(self);
        response
    }
}

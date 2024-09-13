use reqwest::StatusCode;
use sqlx::{migrate::MigrateError as SqlxMigrateError, Error as SqlxError};

pub(crate) type StdError = Box<dyn std::error::Error + Send + Sync + 'static>;

#[derive(Debug)]
pub enum RuntimeError {
    /// Failed to initalize config registry
    ConfigRegistryInitError(StdError),

    /// Failed to deserialize RawConfig into Config
    RawConfigDeserializeError {
        config_name: String,
        raw_config: Box<dyn config::Config>,
    },

    /// Storage sqlx errors
    SqlxError(SqlxError),

    /// Storage migration errors
    SqlxMigrateError(SqlxMigrateError),

    /// Error on sending data to channel
    ChannelSendError,

    ChannelRecvError,

    /// Failed to upgrade weak channel to channel
    ChannelUpgradeError,

    /// Bad control plane URL
    ControlPlaneUrlParseError(String),

    ControlPlaneUrlError(StdError),

    /// Failed to parse token
    ControlPlaneMalformedToken,

    ControlPlaneTlsUrlNotSet,

    ControlPlaneCertifiedNotSet,

    ControlPlaneWebsocketClosed,
    ControlPlaneWebsocketUnexpectedBinarydMessage,
    ControlPlaneWebsocketUnexpectedFrameMessage,
    ControlPlaneWebsocketError(tungstenite::Error),
    ControlPlaneWebsocketSendError,

    /// Control plane request error
    ControlPlaneRequestError(reqwest::Error),

    ControlPlaneJoinError {
        status: StatusCode,
        desc: String,
    },

    PkiCsrError(StdError),
    PkiParseCaCertificateError(StdError),
    PkiParseCertificateError(StdError),
    PkiParsePrivateKeyError(StdError),
    PkiVerifiedInitError(StdError),
    RustlsConfigInitError(StdError),

    SerdeJsonError(serde_json::Error),
    ResetError(StdError),
    
    // Scheduler Errors
    TaskFailedToStart(StdError),
    SectionChannelAllocationError,
    
    // Section Storage Errors
    StorageError(StdError),
}

impl std::fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for RuntimeError {}

impl From<SqlxError> for RuntimeError {
    fn from(value: SqlxError) -> Self {
        RuntimeError::SqlxError(value)
    }
}

impl From<SqlxMigrateError> for RuntimeError {
    fn from(value: SqlxMigrateError) -> Self {
        RuntimeError::SqlxMigrateError(value)
    }
}

impl<T> From<tokio::sync::mpsc::error::SendError<T>> for RuntimeError {
    fn from(_: tokio::sync::mpsc::error::SendError<T>) -> Self {
        RuntimeError::ChannelSendError
    }
}

impl From<tokio::sync::oneshot::error::RecvError> for RuntimeError {
    fn from(_: tokio::sync::oneshot::error::RecvError) -> Self {
        RuntimeError::ChannelRecvError
    }
}

impl From<reqwest::Error> for RuntimeError {
    fn from(value: reqwest::Error) -> Self {
        RuntimeError::ControlPlaneRequestError(value)
    }
}

impl From<serde_json::Error> for RuntimeError {
    fn from(value: serde_json::Error) -> Self {
        RuntimeError::SerdeJsonError(value)
    }
}

impl From<tungstenite::Error> for RuntimeError {
    fn from(value: tungstenite::Error) -> Self {
        RuntimeError::ControlPlaneWebsocketError(value)
    }
}

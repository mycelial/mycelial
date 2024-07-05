pub mod components;

pub type StdError = Box<dyn std::error::Error + Send + Sync + 'static>;
pub type Result<T, E = StdError> = std::result::Result<T, E>;

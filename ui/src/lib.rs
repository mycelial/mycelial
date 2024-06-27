pub mod components;
pub mod config_registry;

pub type StdError = Box<dyn std::error::Error + Send + Sync + 'static>;
pub type Result<T, E = StdError> = std::result::Result<T, E>;

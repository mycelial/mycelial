use components::app::AppError;

pub mod components;

pub type Result<T, E = AppError> = std::result::Result<T, E>;

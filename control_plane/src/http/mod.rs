pub mod api;
pub mod daemon_api;

use crate::app::{AppError, AppErrorKind};
use axum::{
    body::Body,
    http::{Method, Request, StatusCode, Uri},
    middleware::Next,
    response::{IntoResponse, Response},
};
use chrono::Utc;
use std::sync::Arc;

pub type Result<T, E = AppError> = core::result::Result<T, E>;

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let status_code = match self.kind {
            AppErrorKind::Unauthorized => StatusCode::UNAUTHORIZED,
            AppErrorKind::BadRequest
            | AppErrorKind::JoinRequestHashMissmatch
            | AppErrorKind::TokenUsed => StatusCode::BAD_REQUEST,
            AppErrorKind::NotFound => StatusCode::NOT_FOUND,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };
        let mut response = status_code.into_response();
        response.extensions_mut().insert(Arc::new(self));
        response
    }
}

pub async fn log_middleware(
    method: Method,
    uri: Uri,
    request: Request<Body>,
    next: Next,
) -> Response {
    let timestamp = Utc::now();
    let response = next.run(request).await;
    let request_time_ms = Utc::now()
        .signed_duration_since(timestamp)
        .num_milliseconds();

    match response.extensions().get::<Arc<AppError>>() {
        Some(error) => tracing::error!(
            request_time_ms = request_time_ms,
            method = method.as_str(),
            status_code = response.status().as_u16(),
            path = uri.path(),
            error = ?error
        ),
        None => tracing::info!(
            request_time_ms = request_time_ms,
            method = method.as_str(),
            status_code = response.status().as_u16(),
            path = uri.path(),
        ),
    };
    response
}

pub mod assets;
pub mod workspace;
pub mod workspaces;

use axum::{
    body::Body,
    http::{Method, Request, StatusCode, Uri},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{delete, get, post},
    Router,
};
use chrono::Utc;
use std::sync::Arc;

pub type Result<T, E = AppError> = core::result::Result<T, E>;

#[derive(Debug)]
pub struct AppError {
    pub err: anyhow::Error,
}

impl<E: Into<anyhow::Error>> From<E> for AppError {
    fn from(err: E) -> Self {
        Self { err: err.into() }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let mut response = StatusCode::INTERNAL_SERVER_ERROR.into_response();
        response.extensions_mut().insert(Arc::new(self));
        response
    }
}

// log response middleware
async fn log_middleware(method: Method, uri: Uri, request: Request<Body>, next: Next) -> Response {
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

// top level router
pub fn new(app: crate::app::App) -> Router {
    Router::new()
        // workspaces API
        .route(
            "/api/workspaces",
            post(workspaces::create).get(workspaces::read),
        )
        .route(
            "/api/workspaces/:name",
            delete(workspaces::delete)
        )
        // workspace API
        .route(
            "/api/workspace/:name",
            get(workspace::read)
        )
        .fallback(assets::assets)
        .layer(middleware::from_fn(log_middleware))
        .with_state(app)
}

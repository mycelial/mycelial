//! Workspaces routes

use std::sync::Arc;
use axum::{response::IntoResponse, routing::post, Router};

async fn create_workspaces() -> impl IntoResponse {
    "ok"
}

async fn read_workspaces() -> impl IntoResponse {
    "ok"
}

async fn update_workspaces() -> impl IntoResponse {
    "ok"
}

async fn delete_workspaces() -> impl IntoResponse {
    "ok"
}

pub fn new(app: Arc<crate::app::App>) -> Router {
    Router::new()
        .route(
            "/api/workspaces",
             post(create_workspaces)
                .get(read_workspaces)
                .put(update_workspaces)
                .delete(delete_workspaces)
        )
        .with_state(app)
}
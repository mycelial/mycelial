pub mod assets;
pub mod workspace;
pub mod workspaces;
pub mod daemon;

use axum::{
    middleware::{self},
    routing::{delete, get, post},
    Router,
};

pub fn new(app: crate::app::App) -> Router {
    Router::new()
        // workspaces API
        .route(
            "/api/workspaces",
            get(workspaces::read).post(workspaces::create),
        )
        .route("/api/workspaces/:name", delete(workspaces::delete))
        // workspace API
        .route("/api/workspace", post(workspace::update))
        .route("/api/workspace/:name", get(workspace::read))
        // daemon join api
        .route("/api/daemon/join", post(daemon::join))
        // assets
        .fallback(assets::assets)
        .layer(middleware::from_fn(crate::http::log_middleware))
        .with_state(app)
}

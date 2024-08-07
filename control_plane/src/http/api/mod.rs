pub mod assets;
pub mod daemon;
pub mod workspace;
pub mod workspaces;

use axum::{
    middleware::{self},
    routing::{delete, get, post},
    Router,
};

pub fn new(app: crate::app::AppState) -> Router {
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
        .route(
            "/api/daemon/tokens",
            post(daemon::create_token).get(daemon::list_tokens),
        )
        .route("/api/daemon/tokens/:id", delete(daemon::delete_token))
        // assets
        .fallback(assets::assets)
        .layer(middleware::from_fn(crate::http::log_middleware))
        .with_state(app)
}

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
        // daemon tokens api
        .route(
            "/api/daemon/tokens",
            post(daemon::create_token).get(daemon::list_tokens),
        )
        .route("/api/daemon/tokens/:id", delete(daemon::delete_token))
        // daemons api
        .route("/api/daemon", get(daemon::list_daemons))
        .route(
            "/api/daemon/set_name/:id",
            post(daemon::set_name).delete(daemon::unset_name),
        )
        .route(
            "/api/daemon/assign/:node_id/:daemon_id",
            post(daemon::assign_node_to_daemon),
        )
        .route(
            "/api/daemon/unassign/:node_id",
            delete(daemon::unassign_node_from_daemon),
        )
        // assets
        .fallback(assets::assets)
        .layer(middleware::from_fn(crate::http::log_middleware))
        .with_state(app)
}

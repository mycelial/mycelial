pub mod assets;
pub mod workspaces;

use axum::Router;
use std::sync::Arc;

// top level router
pub fn new(app: Arc<crate::app::App>) -> Router {
    Router::new()
        .merge(workspaces::new(Arc::clone(&app)))
        .merge(assets::new())
}

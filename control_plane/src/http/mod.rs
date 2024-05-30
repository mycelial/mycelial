use axum::Router;

pub mod assets;


// top level router
pub fn new() -> Router {
    Router::new()
        .merge(assets::new())
}
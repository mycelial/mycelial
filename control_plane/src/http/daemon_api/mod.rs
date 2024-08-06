use axum::{response::IntoResponse, routing::get, Router};

async fn index() -> impl IntoResponse {
    ""
}

pub fn new(app: crate::app::AppState) -> Router {
    Router::new().route("/", get(index)).with_state(app)
}

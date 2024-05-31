//! embedded assets from ui crate

use axum::{
    http::{header, Uri},
    response::IntoResponse,
    Router,
};
use rust_embed::Embed;

#[derive(Embed)]
#[folder = "../ui/dist/"]
struct Assets;

async fn assets(uri: Uri) -> impl IntoResponse {
    let path = match uri.path() {
        "/" => "index.html",
        p => p,
    }
    .trim_start_matches('/');
    match Assets::get(path) {
        Some(file) => {
            let mime = mime_guess::from_path(path).first_or_octet_stream();
            ([(header::CONTENT_TYPE, mime.as_ref())], file.data).into_response()
        }
        None => {
            // FIXME:
            // fallback to index page
            let index = Assets::get("index.html").unwrap();
            let mime = mime_guess::from_path("index.html").first_or_octet_stream();
            ([(header::CONTENT_TYPE, mime.as_ref())], index.data).into_response()
        }
    }
}

pub fn new() -> Router {
    Router::new().fallback(assets)
}

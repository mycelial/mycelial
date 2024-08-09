use std::{net::SocketAddr, sync::Arc, time::Duration};

use axum::{
    extract::{
        ws::WebSocket,
        State, WebSocketUpgrade,
    },
    middleware,
    response::IntoResponse,
    routing::get,
    Extension, Router,
};
use chrono::Utc;
use futures::StreamExt;

use crate::{app::AppState, tls_server::PeerInfo};

async fn ws_handler(
    State(app): State<AppState>,
    ws: WebSocketUpgrade,
    Extension(PeerInfo { common_name, addr }): Extension<PeerInfo>,
) -> impl IntoResponse {
    tracing::info!("`{common_name}` from {addr} connected.");
    ws.on_upgrade(move |socket| handle_socket(app, socket, addr, common_name))
}

async fn handle_socket(
    app: AppState,
    socket: WebSocket,
    addr: SocketAddr,
    common_name: Arc<str>,
) {
    let (input, mut output) = socket.split();
    loop {
        tokio::select! {
            msg = output.next() => {
                if msg.is_none() {
                    if let Err(e) = app.daemon_set_last_seen(&common_name, Utc::now()).await {
                        tracing::error!("failed to set last seen for {common_name}: {e}");
                    }
                    return;
                }
            }
        }
        tokio::time::sleep(Duration::from_secs(5)).await;
    }
}

pub fn new(app: AppState) -> Router {
    Router::new()
        .route("/", get(ws_handler))
        .layer(middleware::from_fn(crate::http::log_middleware))
        .with_state(app)
}

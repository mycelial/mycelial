use std::{net::SocketAddr, sync::Arc, time::Duration};

use axum::{
    extract::{
        ws::{Message, WebSocket},
        State, WebSocketUpgrade,
    },
    middleware,
    response::IntoResponse,
    routing::get,
    Extension, Router,
};

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
    mut socket: WebSocket,
    addr: SocketAddr,
    common_name: Arc<str>,
) {
    loop {
        if socket.send(Message::Ping(vec![])).await.is_ok() {
            tracing::info!("Pinged {common_name}...");
        } else {
            tracing::error!("Could not send ping {common_name}!");
            return;
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

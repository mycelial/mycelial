use std::{net::SocketAddr, sync::Arc};

use axum::{
    extract::{
        ws::{Message as WebsocketMessage, WebSocket},
        State, WebSocketUpgrade,
    },
    middleware,
    response::IntoResponse,
    routing::get,
    Extension, Router,
};
use chrono::Utc;
use futures::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    app::{AppState, DaemonGraph},
    tls_server::PeerInfo,
    Result,
};

async fn ws_handler(
    State(app): State<AppState>,
    ws: WebSocketUpgrade,
    Extension(PeerInfo { common_name, addr }): Extension<PeerInfo>,
) -> Result<impl IntoResponse> {
    let common_name: Uuid = common_name.parse()?;
    Ok(ws.on_upgrade(move |socket| async move {
        handle_socket(app, socket, addr, common_name).await.ok();
    }))
}

struct DaemonTrackingGuard {
    app: AppState,
    id: Uuid,
}

impl DaemonTrackingGuard {
    async fn new(app: AppState, id: Uuid) -> Result<Self> {
        app.daemon_connected(id).await?;
        Ok(Self { app, id })
    }
}

impl Drop for DaemonTrackingGuard {
    fn drop(&mut self) {
        let id = self.id;
        let app = Arc::clone(&self.app);
        tokio::spawn(async move {
            app.daemon_disconnected(id).await.ok();
        });
    }
}

// FIXME:
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "message")]
pub enum Message {
    GetGraph,
    GetGraphResponse { graph: DaemonGraph },
    RefetchGraph,
}

async fn handle_socket(
    app: AppState,
    socket: WebSocket,
    addr: SocketAddr,
    daemon_id: Uuid,
) -> Result<()> {
    let _guard = DaemonTrackingGuard::new(Arc::clone(&app), daemon_id).await?;
    let (mut input, mut output) = socket.split();
    loop {
        tokio::select! {
            msg = output.next() => {
                let msg = match msg {
                    None => {
                        if let Err(e) = app.daemon_set_last_seen(daemon_id, Utc::now()).await {
                            tracing::error!("failed to set last seen for {daemon_id}: {e}");
                        }
                        return Ok(());
                    },
                    Some(msg) => msg?,
                };
                let msg = match msg {
                    WebsocketMessage::Text(data) => serde_json::from_str::<Message>(&data)?,
                    WebsocketMessage::Ping(_) => continue,
                    WebsocketMessage::Pong(_) => continue,
                    _ => Err(anyhow::anyhow!("unexpected message: {msg:?}"))?,
                };
                match msg {
                    Message::GetGraph => {
                        let response = Message::GetGraphResponse { graph: app.get_daemon_graph(daemon_id).await?};
                        input.send(serde_json::to_string(&response)?.into()).await?;
                    },
                    _ => {
                        tracing::info!("unexpected message: {msg:?}");
                    },
                }
                tracing::info!("got msg: {:?}", msg);
            }

        }
    }
}

pub fn new(app: AppState) -> Router {
    Router::new()
        .route("/", get(ws_handler))
        .layer(middleware::from_fn(crate::http::log_middleware))
        .with_state(app)
}

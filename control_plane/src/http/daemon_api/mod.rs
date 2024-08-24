use std::{net::SocketAddr, sync::Arc};

use crate::{app::daemon_tracker::DaemonMessage, AppError};
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
use futures::{Sink, SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::UnboundedReceiver;
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

struct Daemon {
    id: Uuid,
    rx: UnboundedReceiver<DaemonMessage>,
    app: AppState,
}

impl Daemon {
    async fn new(app: AppState, id: Uuid) -> Result<Self> {
        let rx = app.daemon_connected(id).await?;
        Ok(Self { app, rx, id })
    }
}

impl Drop for Daemon {
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

struct WebsocketInput<S> {
    input: S,
}

impl<S: Sink<WebsocketMessage> + Unpin> WebsocketInput<S> {
    fn new(input: S) -> Self {
        Self { input }
    }

    async fn send_message(&mut self, message: &Message) -> Result<()> {
        self.input
            .send(WebsocketMessage::Text(serde_json::to_string(&message)?))
            .await
            .map_err(|_| anyhow::anyhow!("failed to send websocket message"))?;
        Ok(())
    }
}

async fn handle_socket(
    app: AppState,
    socket: WebSocket,
    addr: SocketAddr,
    daemon_id: Uuid,
) -> Result<()> {
    let daemon = &mut Daemon::new(Arc::clone(&app), daemon_id).await?;
    let (input, mut output) = socket.split();
    let input = &mut WebsocketInput::new(input);
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
                        input.send_message(
                            &Message::GetGraphResponse { graph: app.get_daemon_graph(daemon_id).await?}
                        ).await?;
                    },
                    _ => {
                        tracing::info!("unexpected message: {msg:?}");
                    },
                }
            },

            msg = daemon.rx.recv() => {
                let msg = match msg {
                    None => Err(AppError::internal("daemon tracker is down"))?,
                    Some(msg) => msg
                };
                match msg {
                    DaemonMessage::NotifyGraphUpdate => {
                        input.send_message(&Message::RefetchGraph).await?;
                    }
                }
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

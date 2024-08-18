use std::{sync::Arc, time::Duration};

use anyhow::Result;
use pki::ClientConfig;
use reqwest::Url;
use futures::{SinkExt as _, StreamExt as _};
use serde::{Deserialize, Serialize};
use sha2::Digest;
use tokio::{
    sync::{
        mpsc::{channel, Receiver, Sender, WeakSender},
        oneshot::{channel as oneshot_channel, Sender as OneshotSender},
    },
    task::JoinHandle,
};
use tokio_tungstenite::Connector;
use tungstenite::Message as WebsocketMessage;

use crate::{CertifiedKey, DaemonHandle};

#[derive(Debug, Serialize)]
struct JoinRequest<'a> {
    id: &'a str,
    csr: &'a str,
    hash: String,
}

impl<'a> JoinRequest<'a> {
    fn new(id: &'a str, csr: &'a str, secret: &'a str) -> Self {
        let mut hasher = sha2::Sha256::new();
        [csr, ":", secret]
            .into_iter()
            .for_each(|value| hasher.update(value));
        let hash = format!("{:x}", hasher.finalize());
        Self { id, csr, hash }
    }
}

#[derive(Deserialize)]
struct JoinErrorResponse {
    error: String,
}

#[derive(Deserialize)]
struct JoinResponse {
    certificate: String,
    ca_certificate: String,
}

#[derive(Debug)]
struct ControlPlaneClient {
    daemon_handle: DaemonHandle,
    socket: Option<JoinHandle<()>>,
    control_plane_tls_url: Option<Arc<Url>>,
    certifiedkey: Option<Arc<CertifiedKey>>,
}

impl ControlPlaneClient {
    fn new(daemon_handle: DaemonHandle) -> Self {
        Self {
            daemon_handle,
            socket: None,
            control_plane_tls_url: None,
            certifiedkey: None,
        }
    }

    fn spawn(self) -> ControlPlaneClientHandle {
        let (tx, rx) = channel(1);
        let weak_tx = tx.clone().downgrade();
        tokio::spawn(async move {
            let mut rx = rx;
            if let Err(e) = self.enter_loop(weak_tx, &mut rx).await {
                tracing::error!("control plane client stopped: {e}")
            };
        });
        ControlPlaneClientHandle { tx }
    }

    async fn enter_loop(
        mut self,
        weak_tx: WeakSender<Message>,
        rx: &mut Receiver<Message>,
    ) -> Result<()> {
        while let Some(msg) = rx.recv().await {
            match msg {
                Message::Join {
                    control_plane_url,
                    join_token,
                    reply_to,
                } => {
                    let response = self.join(&control_plane_url, &join_token).await;
                    reply_to.send(response).ok();
                }
                Message::SetTlsUrl {
                    control_plane_tls_url,
                    certifiedkey,
                    reply_to,
                } => {
                    let response = match self.set_tls_url(control_plane_tls_url, certifiedkey) {
                        Ok(()) => self.setup_websocket_client(&weak_tx).await,
                        Err(e) => Err(e),
                    };
                    reply_to.send(response).ok();
                }
                Message::Shutdown { reply_to } => {
                    reply_to.send(Ok(())).ok();
                    return Ok(());
                }
                Message::WebSocketClientDown => {
                    if self.control_plane_tls_url.is_none() || self.certifiedkey.is_none() {
                        continue;
                    };
                    tracing::error!("websocket client is down, restarting");
                    if let Err(e) = self.setup_websocket_client(&weak_tx).await {
                        tracing::error!("failed to restart websocket client: {e}");
                    };
                }
            }
        }
        Err(anyhow::anyhow!("all control plane handles are dropped"))?
    }

    fn set_tls_url(
        &mut self,
        control_plane_tls_url: String,
        certifiedkey: CertifiedKey,
    ) -> Result<()> {
        let mut url: Url = control_plane_tls_url.parse()?;
        match url.scheme() {
            "http" | "https" => Some("wss"),
            _ => None,
        }
        .map(|new_scheme| url.set_scheme(new_scheme));
        self.control_plane_tls_url = Some(Arc::from(url));
        self.certifiedkey = Some(Arc::new(certifiedkey));
        Ok(())
    }

    async fn join(&self, control_plane_url: &str, join_token: &str) -> Result<CertifiedKey> {
        let control_plane_url: Url = control_plane_url.parse()?;
        let split = join_token.splitn(2, ":").collect::<Vec<_>>();
        let (token, secret) = match split.as_slice() {
            [token, secret] => (token, secret),
            _ => Err(anyhow::anyhow!("malformed token"))?,
        };
        let (key, csr) = pki::generate_csr_request(token)
            .map(|(key, csr)| (key.serialize_pem(), csr.pem()))
            .map_err(|e| anyhow::anyhow!("failed to generate csr: {e}"))?;
        let csr = csr?;
        let request = JoinRequest::new(token, &csr, secret);
        let response = reqwest::Client::new()
            .post(control_plane_url.join("api/daemon/join")?)
            .json(&request)
            .send()
            .await?;
        let response = match response.status() {
            status if status.is_success() => response.json::<JoinResponse>().await?,
            status => {
                let error = response.json::<JoinErrorResponse>().await?;
                Err(anyhow::anyhow!(
                    "failed to join control plane: {status}, error: {}",
                    error.error
                ))?
            }
        };
        Ok(CertifiedKey {
            key,
            certificate: response.certificate,
            ca_certificate: response.ca_certificate,
        })
    }

    async fn setup_websocket_client(&mut self, weak_tx: &WeakSender<Message>) -> Result<()> {
        // get tx clone before upgrade
        let tx = weak_tx
            .clone()
            .upgrade()
            .ok_or(anyhow::anyhow!("failed to upgrade tx"))?;

        // drop previous client
        if let Some(join_handle) = self.socket.take() {
            join_handle.abort()
        }

        let control_plane_tls_url = self
            .control_plane_tls_url
            .as_ref()
            .map(Clone::clone)
            .ok_or(anyhow::anyhow!("control plane tls url is not set"))?;
        let certifiedkey = self
            .certifiedkey
            .as_ref()
            .map(Clone::clone)
            .ok_or(anyhow::anyhow!("certified key is not set"))?;

        let daemon_handle = self.daemon_handle.clone();
        self.socket = Some(tokio::spawn(async move {
            let mut tx = tx;
            if let Err(e) =
                websocket_client(daemon_handle, control_plane_tls_url, certifiedkey).await
            {
                tracing::error!("websocket error: {e}");
            }
            tokio::time::sleep(Duration::from_secs(3)).await;
            tx.send(Message::WebSocketClientDown { })
                .await
                .ok();
        }));
        Ok(())
    }
}

enum Message {
    Join {
        control_plane_url: String,
        join_token: String,
        reply_to: OneshotSender<Result<CertifiedKey>>,
    },
    SetTlsUrl {
        control_plane_tls_url: String,
        certifiedkey: CertifiedKey,
        reply_to: OneshotSender<Result<()>>,
    },
    Shutdown {
        reply_to: OneshotSender<Result<()>>,
    },
    WebSocketClientDown,
}

async fn websocket_client(
    daemon_handle: DaemonHandle,
    control_plane_url: Arc<Url>,
    certifiedkey: Arc<CertifiedKey>,
) -> Result<()> {
    tracing::info!("connected to control plane");
    let ca_cert = pki::parse_certificate(&certifiedkey.ca_certificate)
        .map_err(|e| anyhow::anyhow!("failed to parse ca certificate: {e}"))?;
    let cert = pki::parse_certificate(&certifiedkey.certificate)
        .map_err(|e| anyhow::anyhow!("failed to parse certificate: {e}"))?;
    let key = pki::parse_private_key(&certifiedkey.key)
        .map_err(|e| anyhow::anyhow!("failed to parse private key: {e}"))?;
    let connector = Connector::Rustls(Arc::new(
        ClientConfig::builder()
            .dangerous()
            .with_custom_certificate_verifier(Arc::new(
                pki::Verifier::new(ca_cert)
                    .map_err(|e| anyhow::anyhow!("failed to build client verifier: {e}"))?,
            ))
            .with_client_auth_cert(vec![cert], key)?,
    ));
    let (socket, _response) = tokio_tungstenite::connect_async_tls_with_config(
        control_plane_url.as_str(),
        None,
        false,
        Some(connector),
    )
    .await?;
    let (mut input, mut output) = socket.split();
    let mut interval = tokio::time::interval(Duration::from_secs(30));
    input.send(WebsocketMessage::Text(serde_json::to_string(&protocol::Message::get_graph())?)).await?;
    loop {
        tokio::select! {
            msg = output.next() => {
                let msg = match msg {
                    None => Err(anyhow::anyhow!("connection closed"))?,
                    Some(msg) => msg?,
                };
                let data = match msg {
                    WebsocketMessage::Ping{ .. } => { continue },
                    WebsocketMessage::Pong{ .. } => { continue },
                    WebsocketMessage::Close{ .. } => Err(anyhow::anyhow!("connection closed"))?,
                    WebsocketMessage::Binary{ .. } => Err(anyhow::anyhow!("unexpected binary message"))?,
                    WebsocketMessage::Frame{ .. } => Err(anyhow::anyhow!("unexpected raw frame"))?,
                    WebsocketMessage::Text(data) => serde_json::from_str::<protocol::Message>(&data),
                };
            },
            _ = interval.tick() => {
                if let Err(e) = input.send(WebsocketMessage::Ping(vec![])).await {
                    Err(anyhow::anyhow!("failed to ping control plane: {e}"))?;
                }
            }
        }
    }
}

#[derive(Debug)]
pub struct ControlPlaneClientHandle {
    tx: Sender<Message>,
}

impl ControlPlaneClientHandle {
    pub async fn join(&self, control_plane_url: &str, join_token: &str) -> Result<CertifiedKey> {
        let (reply_to, rx) = oneshot_channel();
        let message = Message::Join {
            control_plane_url: control_plane_url.into(),
            join_token: join_token.into(),
            reply_to,
        };
        self.tx.send(message).await?;
        rx.await?
    }

    pub async fn set_tls_url(
        &self,
        control_plane_tls_url: String,
        certifiedkey: CertifiedKey,
    ) -> Result<()> {
        let (reply_to, rx) = oneshot_channel();
        let message = Message::SetTlsUrl {
            control_plane_tls_url,
            certifiedkey,
            reply_to,
        };
        self.tx.send(message).await?;
        rx.await?
    }

    pub async fn shutdown(&self) -> Result<()> {
        let (reply_to, rx) = oneshot_channel();
        let message = Message::Shutdown { reply_to };
        self.tx.send(message).await?;
        rx.await?
    }
}

pub fn new(daemon_handle: DaemonHandle) -> ControlPlaneClientHandle {
    let client = ControlPlaneClient::new(daemon_handle);
    client.spawn()
}

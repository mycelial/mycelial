use std::{sync::Arc, time::Duration};

use pki::ClientConfig;
use reqwest::Url;
use section::{
    futures::{SinkExt as _, StreamExt as _},
    prelude::Sink,
};
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

use crate::{
    runtime::{CertifiedKey, Graph, RuntimeHandle},
    runtime_error::RuntimeError,
    Result,
};

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
    runtime_handle: RuntimeHandle,
    socket: Option<JoinHandle<()>>,
    control_plane_tls_url: Option<Arc<Url>>,
    certifiedkey: Option<Arc<CertifiedKey>>,
}

impl ControlPlaneClient {
    fn new(runtime_handle: RuntimeHandle) -> Self {
        Self {
            runtime_handle,
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
        Ok(())
    }

    fn set_tls_url(
        &mut self,
        control_plane_tls_url: String,
        certifiedkey: CertifiedKey,
    ) -> Result<()> {
        let mut url: Url = control_plane_tls_url
            .parse()
            .map_err(|_| RuntimeError::ControlPlaneUrlParseError(control_plane_tls_url))?;
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
        let control_plane_url: Url = control_plane_url
            .parse()
            .map_err(|_| RuntimeError::ControlPlaneUrlParseError(control_plane_url.into()))?;
        let split = join_token.splitn(2, ":").collect::<Vec<_>>();
        let (token, secret) = match split.as_slice() {
            [token, secret] => (token, secret),
            _ => Err(RuntimeError::ControlPlaneMalformedToken)?,
        };
        let (key, csr) = pki::generate_csr_request(token)
            .map(|(key, csr)| (key.serialize_pem(), csr.pem()))
            .map_err(RuntimeError::PkiCsrError)?;
        let csr = csr.map_err(|e| RuntimeError::PkiCsrError(e.into()))?;
        let request = JoinRequest::new(token, &csr, secret);
        let response = reqwest::Client::new()
            .post(
                control_plane_url
                    .join("api/daemon/join")
                    .map_err(|e| RuntimeError::ControlPlaneUrlError(e.into()))?,
            )
            .json(&request)
            .send()
            .await?;
        let response = match response.status() {
            status if status.is_success() => response.json::<JoinResponse>().await?,
            status => {
                let error = response.json::<JoinErrorResponse>().await?;
                Err(RuntimeError::ControlPlaneJoinError {
                    status,
                    desc: error.error,
                })?
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
            .ok_or(RuntimeError::ChannelUpgradeError)?;

        // drop previous client
        if let Some(join_handle) = self.socket.take() {
            join_handle.abort()
        }

        let control_plane_tls_url = self
            .control_plane_tls_url
            .as_ref()
            .map(Clone::clone)
            .ok_or(RuntimeError::ControlPlaneTlsUrlNotSet)?;
        let certifiedkey = self
            .certifiedkey
            .as_ref()
            .map(Clone::clone)
            .ok_or(RuntimeError::ControlPlaneCertifiedNotSet)?;

        let runtime_handle = self.runtime_handle.clone();
        self.socket = Some(tokio::spawn(async move {
            let tx = tx;
            if let Err(e) =
                websocket_client(runtime_handle, control_plane_tls_url, certifiedkey).await
            {
                tracing::error!("websocket connection closed: {e}");
            }
            tokio::time::sleep(Duration::from_secs(3)).await;
            tx.send(Message::WebSocketClientDown {}).await.ok();
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

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "message")]
pub enum ControlPlaneMessage {
    GetGraph,
    GetGraphResponse { graph: Graph },
    RefetchGraph,
}

struct WebsocketInput<S> {
    input: S,
}

impl<S: Sink<WebsocketMessage> + Unpin> WebsocketInput<S> {
    async fn get_graph(&mut self) -> Result<()> {
        self.input
            .send(WebsocketMessage::Text(serde_json::to_string(
                &ControlPlaneMessage::GetGraph {},
            )?))
            .await
            .map_err(|_| RuntimeError::ControlPlaneWebsocketSendError)?;
        Ok(())
    }

    async fn ping(&mut self) -> Result<()> {
        self.input
            .send(WebsocketMessage::Ping(vec![]))
            .await
            .map_err(|_| RuntimeError::ControlPlaneWebsocketSendError)?;
        Ok(())
    }
}

async fn websocket_client(
    runtime_handle: RuntimeHandle,
    control_plane_url: Arc<Url>,
    certifiedkey: Arc<CertifiedKey>,
) -> Result<()> {
    tracing::info!("connected to control plane");
    let ca_cert = pki::parse_certificate(&certifiedkey.ca_certificate)
        .map_err(RuntimeError::PkiParseCaCertificateError)?;
    let cert = pki::parse_certificate(&certifiedkey.certificate)
        .map_err(RuntimeError::PkiParseCertificateError)?;
    let key =
        pki::parse_private_key(&certifiedkey.key).map_err(RuntimeError::PkiParsePrivateKeyError)?;
    let connector = Connector::Rustls(Arc::new(
        ClientConfig::builder()
            .dangerous()
            .with_custom_certificate_verifier(Arc::new(
                pki::Verifier::new(ca_cert).map_err(RuntimeError::PkiVerifiedInitError)?,
            ))
            .with_client_auth_cert(vec![cert], key)
            .map_err(|e| RuntimeError::RustlsConfigInitError(e.into()))?,
    ));
    let (socket, _response) = tokio_tungstenite::connect_async_tls_with_config(
        control_plane_url.as_str(),
        None,
        false,
        Some(connector),
    )
    .await?;
    let (input, mut output) = socket.split();
    let input = &mut WebsocketInput { input };
    input.get_graph().await?;
    let mut interval = tokio::time::interval(Duration::from_secs(30));
    loop {
        tokio::select! {
            msg = output.next() => {
                let msg = match msg {
                    None => Err(RuntimeError::ControlPlaneWebsocketClosed)?,
                    Some(msg) => msg?,
                };
                let message = match msg {
                    WebsocketMessage::Ping{ .. } => { continue },
                    WebsocketMessage::Pong{ .. } => { continue },
                    WebsocketMessage::Close{ .. } => Err(RuntimeError::ControlPlaneWebsocketClosed)?,
                    WebsocketMessage::Binary{ .. } => Err(RuntimeError::ControlPlaneWebsocketUnexpectedBinarydMessage)?,
                    WebsocketMessage::Frame{ .. } => Err(RuntimeError::ControlPlaneWebsocketUnexpectedFrameMessage)?,
                    WebsocketMessage::Text(data) => serde_json::from_str::<ControlPlaneMessage>(&data)?,
                };
                match message {
                    ControlPlaneMessage::GetGraphResponse{ graph } => runtime_handle.graph(graph)?,
                    ControlPlaneMessage::RefetchGraph => input.get_graph().await?,
                    _ => (),
                }
            },
            _ = interval.tick() => {
                input.ping().await?;
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

pub fn new(runtime_handle: RuntimeHandle) -> ControlPlaneClientHandle {
    let client = ControlPlaneClient::new(runtime_handle);
    client.spawn()
}

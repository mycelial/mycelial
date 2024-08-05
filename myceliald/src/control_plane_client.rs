use std::sync::Arc;

use anyhow::Result;
use reqwest::Url;
use serde::{Deserialize, Serialize};
use sha2::Digest;
use tokio::{
    sync::{
        mpsc::{
            channel, Receiver, Sender, UnboundedSender,
            WeakSender,
        },
        oneshot::{channel as oneshot_channel, Sender as OneshotSender},
    },
    task::JoinHandle,
};

use crate::{CertifiedKey, DaemonMessage};

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
struct JoinResponse {
    certificate: String,
}

#[derive(Debug)]
struct ControlPlaneClient {
    tx: UnboundedSender<DaemonMessage>,
    socket: Option<JoinHandle<()>>,
    control_plane_tls_url: Option<Arc<Url>>,
    certifiedkey: Option<Arc<CertifiedKey>>,
    gen: u64,
}

impl ControlPlaneClient {
    fn new(tx: UnboundedSender<DaemonMessage>) -> Self {
        Self {
            tx,
            socket: None,
            control_plane_tls_url: None,
            certifiedkey: None,
            gen: 0,
        }
    }

    fn spawn(self) -> ControlPlaneClientHandle {
        let (tx, rx) = channel(1);
        let weak_tx = tx.clone().downgrade();
        tokio::spawn(async move {
            let mut rx = rx;
            if let Err(e) = self.enter_loop(weak_tx, &mut rx).await {
                tracing::error!("control plane client error: {e}")
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
                    let response = self
                        .setup_websocket_client(&weak_tx, control_plane_tls_url, certifiedkey)
                        .await;
                    reply_to.send(response).ok();
                }
            }
        }
        Err(anyhow::anyhow!("channel closed"))?
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
            status => Err(anyhow::anyhow!("failed to join control plane: {status}"))?,
        };
        Ok(CertifiedKey {
            key,
            certificate: response.certificate,
        })
    }

    async fn setup_websocket_client(
        &mut self,
        weak_tx: &WeakSender<Message>,
        control_plane_tls_url: String,
        certifiedkey: CertifiedKey,
    ) -> Result<()> {
        let url: Url = control_plane_tls_url.parse()?;
        self.control_plane_tls_url = Some(Arc::from(url));
        self.certifiedkey = Some(Arc::new(certifiedkey));

        // get tx clone before upgrade
        let tx = weak_tx
            .clone()
            .upgrade()
            .ok_or(anyhow::anyhow!("failed to upgrade tx"))?;
        // increment generation of websocket client ( all messages from previous generations will be ignored )
        self.gen += 1;
        // drop previous client
        if let Some(join_handle) = self.socket.take() { join_handle.abort() }

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

        let gen = self.gen;
        self.socket = Some(tokio::spawn(async move {
            if let Err(e) = websocket_client(gen, tx, control_plane_tls_url, certifiedkey).await {
                tracing::error!("websocket error: {e}");
            }
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
}

async fn websocket_client(
    generation: u64,
    tx: Sender<Message>,
    control_plane_url: Arc<Url>,
    certifiedkey: Arc<CertifiedKey>,
) -> Result<()> {
    Ok(())
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
}

pub fn new(tx: UnboundedSender<DaemonMessage>) -> ControlPlaneClientHandle {
    let client = ControlPlaneClient::new(tx);
    client.spawn()
}

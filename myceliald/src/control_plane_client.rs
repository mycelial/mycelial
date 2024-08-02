use reqwest::Url;
use anyhow::Result;
use pki;
use serde::{Deserialize, Serialize};
use sha2::Digest;
use tokio::sync::{
    mpsc::{unbounded_channel, Sender, UnboundedReceiver, UnboundedSender},
    oneshot::{channel as oneshot_channel, Sender as OneshotSender}
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
        [csr, ":", secret].into_iter().for_each(|value| hasher.update(value));
        let hash = format!("{:x}", hasher.finalize());
        Self { id, csr, hash }
    }
}

#[derive(Deserialize)]
struct JoinResponse {
    certificate: String,
}

pub struct CertifiedKey {
    pub key: String,
    pub certificate: String,
}

#[derive(Debug)]
struct ControlPlaneClient { }

impl ControlPlaneClient {
    fn spawn() -> ControlPlaneClientHandle {
        let (tx, rx) = unbounded_channel();
        let client = Self {};
        tokio::spawn(async move { 
            let mut rx = rx;
            if let Err(e) = client.enter_loop(&mut rx).await {
                tracing::error!("control plane client error: {e}")
            };
        });
        ControlPlaneClientHandle { tx }

    }
    
    async fn enter_loop(self, rx: &mut UnboundedReceiver<Message>) -> Result<()> {
        while let Some(msg) = rx.recv().await {
            match msg {
                Message::Join{ control_plane_url, join_token, reply_to } => {
                    let response = self.join(&control_plane_url, &control_plane_tls_url, &join_token).await;
                    reply_to.send(response).await.ok();
                },
            }
        }
        Err(anyhow::anyhow!("channel closed"))?
    }
    
    /// join control plane via provided token
    async fn join(&self, control_plane_url: &str, control_plane_tls_url: &str, join_token: &str) -> Result<CertifiedKey> {
        // FIXME: check of daemon already joined
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
            .post(control_plane_url.join("/api/daemon/join")?)
            .json(&request)
            .send()
            .await?;
        let response = match response.status() {
            status if status.is_success() => response.json::<JoinResponse>().await?,
            status => Err(anyhow::anyhow!("failed to join control plane: {status}"))?,
        };
        Ok(CertifiedKey{ key, certificate: response.certificate })
    }
}

enum Message {
    Join {
        control_plane_url: String,
        join_token: String,
        reply_to: 
    }
}

pub struct ControlPlaneClientHandle {
    tx: UnboundedSender<Message>
}

impl ControlPlaneClientHandle {
    pub async fn join(&self, control_plane_url: &str, control_plane_tls_url: &str, join_token: &str) -> Result<CertifiedKey> {

    }
}

pub fn new() -> ControlPlaneClientHandle {
    ControlPlaneClient::spawn()
}
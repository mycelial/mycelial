pub mod db;
pub mod migration;
pub mod tables;

use chrono::{DateTime, Utc};
use config::prelude::*;
use config_registry::{self, ConfigRegistry};
use pki::{CertificateDer, CertifiedKey, KeyPair};
use serde::{Deserialize, Serialize};
use sha2::Digest;
use std::sync::Arc;
use uuid::Uuid;

pub type Result<T, E = AppError> = std::result::Result<T, E>;

#[derive(Debug, PartialEq, Eq)]
pub enum AppErrorKind {
    Unauthorized,
    BadRequest,
    NotFound,
    Internal,
    TokenUsed,
    JoinRequestHashMissmatch,
}

#[derive(Debug)]
pub struct AppError {
    pub kind: AppErrorKind,
    pub err: anyhow::Error,
}

impl AppError {
    pub fn new(err: anyhow::Error) -> Self {
        Self {
            kind: AppErrorKind::Internal,
            err,
        }
    }

    pub fn not_found(err: anyhow::Error) -> Self {
        Self {
            kind: AppErrorKind::NotFound,
            err,
        }
    }

    pub fn token_used(id: Uuid) -> Self {
        Self {
            kind: AppErrorKind::TokenUsed,
            err: anyhow::anyhow!("token {id} already used"),
        }
    }

    pub fn join_hash_missmatch(id: Uuid) -> Self {
        Self {
            kind: AppErrorKind::JoinRequestHashMissmatch,
            err: anyhow::anyhow!("join request hash missmatch for token id {id}"),
        }
    }
}

impl<E: Into<anyhow::Error>> From<E> for AppError {
    fn from(err: E) -> Self {
        Self::new(err.into())
    }
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

// App Types
#[derive(Debug, Serialize, Deserialize)]
pub struct Workspace {
    pub name: String,
    #[serde(default)]
    pub created_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize)]
pub struct Graph {
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
}

#[derive(Debug, Serialize)]
pub struct Node {
    pub id: uuid::Uuid,
    pub display_name: String,
    pub config: Box<dyn config::Config>,
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Serialize)]
pub struct Edge {
    pub from_id: uuid::Uuid,
    pub to_id: uuid::Uuid,
}

#[derive(Debug, Deserialize)]
pub struct WorkspaceUpdate {
    name: String,
    operations: Vec<WorkspaceOperation>,
}

#[derive(Debug, Deserialize)]
pub enum WorkspaceOperation {
    AddNode {
        id: Uuid,
        x: f64,
        y: f64,
        config: Box<dyn config::Config>,
    },
    UpdateNodeConfig {
        id: Uuid,
        config: Box<dyn config::Config>,
    },
    UpdateNodePosition {
        uuid: Uuid,
        x: f64,
        y: f64,
    },
    RemoveNode(Uuid),
    AddEdge {
        from: Uuid,
        to: Uuid,
    },
    RemoveEdge {
        from: Uuid,
    },
}

#[derive(Debug, Serialize)]
pub struct Daemon {
    pub id: uuid::Uuid,
    pub display_name: String,
    pub last_online: String,
}

#[derive(Debug, Serialize)]
pub struct DaemonToken {
    pub id: uuid::Uuid,
    pub secret: String,
    pub issued_at: chrono::DateTime<Utc>,
    pub used_at: Option<chrono::DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
pub struct DaemonJoinRequest {
    pub id: Uuid,
    pub csr: String,
    pub hash: String,
}

#[derive(Debug, Serialize)]
pub struct DaemonJoinResponse {
    certificate: String,
    ca_certificate: String,
}

pub struct AppBuilder {
    db: Box<dyn db::DbTrait>,
}

impl AppBuilder {
    pub async fn new(database_url: &str) -> Result<Self> {
        let db = db::new(database_url).await?;
        db.migrate().await?;
        Ok(Self { db })
    }

    pub async fn build(self) -> Result<App> {
        let certificate_bundle = self.get_or_create_certificate_bundle().await?;
        Ok(App {
            db: self.db,
            config_registry: config_registry::new()
                .map_err(|e| anyhow::anyhow!("failed to build config registry: {e}"))?,
            certificate_bundle,
        })
    }

    async fn get_or_create_certificate_bundle(&self) -> Result<CertificateBundle> {
        let ca_cert_key = self.get_or_create_ca_cert_key().await?;
        let (cert, key) = self
            .get_or_create_control_plane_cert_key(&ca_cert_key)
            .await?;
        Ok(CertificateBundle {
            ca_cert_key,
            cert,
            key,
        })
    }

    async fn get_or_create_ca_cert_key(&self) -> Result<CertifiedKey> {
        if let Some((ca_cert, ca_key)) = self.db.get_ca_cert_key().await? {
            let ca_certkey = pki::rebuild_ca_certkey(&ca_key, &ca_cert)
                .map_err(|e| anyhow::anyhow!("failed to rebuild ca certkey: {e}"))?;
            return Ok(ca_certkey);
        }
        let ca_certkey =
            pki::generate_ca_certkey("control plane").map_err(|e| anyhow::anyhow!("{e}"))?;
        let cert = ca_certkey.cert.pem();
        let key = ca_certkey.key_pair.serialize_pem();
        self.db.store_ca_cert_key(&key, &cert).await?;
        Ok(ca_certkey)
    }

    async fn get_or_create_control_plane_cert_key(
        &self,
        ca_cert_key: &CertifiedKey,
    ) -> Result<(CertificateDer<'static>, KeyPair)> {
        if let Some((cert, key)) = self.db.get_control_plane_cert_key().await? {
            let cert = pki::parse_certificate(&cert).map_err(|e| anyhow::anyhow!("{e}"))?;
            let key = pki::parse_keypair(&key).map_err(|e| anyhow::anyhow!("{e}"))?;
            return Ok((cert, key));
        }
        let certkey = pki::generate_control_plane_cert(ca_cert_key, "control plane")
            .map_err(|e| anyhow::anyhow!("failed to create certificate for control plane: {e}"))?;
        let cert = certkey.cert.pem();
        let key = certkey.key_pair.serialize_pem();
        self.db
            .store_control_plane_cert_key(key.as_str(), cert.as_str())
            .await?;
        Ok((certkey.cert.der().clone(), certkey.key_pair))
    }
}

pub type AppState = Arc<App>;

pub(crate) struct App {
    db: Box<dyn db::DbTrait>,
    config_registry: ConfigRegistry,
    certificate_bundle: CertificateBundle,
}

pub(crate) struct CertificateBundle {
    pub ca_cert_key: CertifiedKey,
    pub cert: CertificateDer<'static>,
    pub key: KeyPair,
}

impl App {
    pub fn certificate_bundle(&self) -> &CertificateBundle {
        &self.certificate_bundle
    }

    // workspaces api
    pub async fn create_workspace(&self, workspace: &Workspace) -> Result<()> {
        self.db.create_workspace(workspace).await
    }

    pub async fn read_workspaces(&self) -> Result<Vec<Workspace>> {
        self.db.read_workspaces().await
    }

    pub async fn delete_workspace(&self, name: &str) -> Result<()> {
        self.db.delete_workspace(name).await
    }

    // workspace api
    pub async fn get_graph(&self, name: &str) -> Result<Graph> {
        let mut graph = self.db.get_graph(name).await?;
        let mut nodes = Vec::with_capacity(graph.nodes.len());
        for mut node in graph.nodes.into_iter() {
            match self.config_registry.build_config(node.config.name()) {
                Ok(mut config) => {
                    if let Err(e) = deserialize_into_config(&*node.config, &mut *config) {
                        // FIXME: emit warning which will be visible in UI
                        tracing::error!(
                            "failed to deserialize stored config with name: {}: {e}",
                            config.name()
                        );
                    };
                    config.strip_secrets();
                    std::mem::swap(&mut node.config, &mut config);
                    nodes.push(node)
                }
                Err(e) => {
                    tracing::error!(
                        "failed to build config from config registry for {}: {e}",
                        node.config.name()
                    );
                }
            }
        }
        graph.nodes = nodes;
        Ok(graph)
    }

    pub async fn update_workspace(&self, updates: &mut [WorkspaceUpdate]) -> Result<()> {
        // validate configs
        for update in updates.iter_mut() {
            for operation in update.operations.as_mut_slice() {
                if let WorkspaceOperation::AddNode { config, .. } = operation {
                    let config_name = config.name();
                    let mut default_config = self
                        .config_registry
                        .build_config(config_name)
                        .map_err(|e| {
                            anyhow::anyhow!("failed to build config for {config_name}: {e}")
                        })?;
                    deserialize_into_config(&**config, &mut *default_config).map_err(|e| {
                        anyhow::anyhow!("failed to deserialize config {config_name}: {e}")
                    })?;
                    std::mem::swap(config, &mut default_config);
                }
            }
        }
        self.db
            .update_workspace(&self.config_registry, updates)
            .await
    }

    // daemon API

    pub async fn create_daemon_token(&self) -> Result<DaemonToken> {
        let secret = rand::random::<[u8; 16]>()
            .into_iter()
            .map(|byte| format!("{byte:x}"))
            .collect::<Vec<_>>()
            .join("");
        let token = DaemonToken {
            id: uuid::Uuid::now_v7(),
            secret,
            issued_at: Utc::now(),
            used_at: None,
        };
        self.db.store_daemon_token(&token).await?;
        Ok(token)
    }

    pub async fn list_daemon_tokens(&self) -> Result<Vec<DaemonToken>> {
        self.db.list_daemon_tokens().await
    }

    pub async fn delete_daemon_token(&self, id: uuid::Uuid) -> Result<()> {
        self.db.delete_daemon_token(id).await?;
        Ok(())
    }

    pub async fn daemon_join(&self, join_request: DaemonJoinRequest) -> Result<DaemonJoinResponse> {
        let token = match self.db.consume_token(join_request.id).await? {
            None => return Err(AppError::not_found(anyhow::anyhow!("token not found"))),
            Some(token) => token,
        };
        let mut hasher = sha2::Sha256::new();
        [&join_request.csr, ":", &token.secret]
            .into_iter()
            .for_each(|value| hasher.update(value));
        let hash = format!("{:x}", hasher.finalize());
        if hash != join_request.hash {
            tracing::error!("join request hash doesn't match");
            Err(AppError::join_hash_missmatch(join_request.id))?
        };
        let certificate = pki::sign_csr(
            &self.certificate_bundle().ca_cert_key,
            join_request.csr.as_str(),
            &join_request.id.to_string(),
        )
        .map_err(|e| anyhow::anyhow!("failed to sign certificate request: {e}"))?;
        self.db.add_daemon(join_request.id).await?;
        Ok(DaemonJoinResponse {
            certificate: certificate.pem(),
            ca_certificate: self.certificate_bundle.ca_cert_key.cert.pem(),
        })
    }

    pub async fn list_daemons(&self) -> Result<Vec<Daemon>> {
        unimplemented!()
    }

    pub async fn daemon_set_last_seen(&self, id: &str, timestamp: DateTime<Utc>) -> Result<()> {
        let id: Uuid = id.parse()?;
        self.db.daemon_set_last_seen(id, timestamp).await?;
        Ok(())
    }
}

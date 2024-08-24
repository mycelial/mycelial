pub mod daemon_tracker;
pub mod db;
pub mod migration;
pub mod tables;

use chrono::{DateTime, Utc};
use config::prelude::*;
use config_registry::{self, ConfigRegistry};
use daemon_tracker::DaemonMessage;
use pki::{CertificateDer, CertifiedKey, KeyPair};
use serde::{Deserialize, Serialize};
use sha2::Digest;
use std::{collections::BTreeMap, sync::Arc};
use tokio::sync::mpsc::UnboundedReceiver;
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
    WorkspaceNotFound,
    ConfigNotFound,
    ConfigIsInvalid,
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

    pub fn workspace_not_found(workspace: &str) -> Self {
        Self {
            kind: AppErrorKind::WorkspaceNotFound,
            err: anyhow::anyhow!("{workspace} not found"),
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

    pub fn internal(desc: &'static str) -> Self {
        Self {
            kind: AppErrorKind::Internal,
            err: anyhow::anyhow!(desc),
        }
    }

    pub fn is_internal(&self) -> bool {
        self.kind == AppErrorKind::Internal
    }

    pub fn config_not_found(name: &str) -> Self {
        Self {
            kind: AppErrorKind::ConfigNotFound,
            err: anyhow::anyhow!("config with {name} not found in config registry"),
        }
    }

    pub fn invalid_config(name: &str) -> Self {
        Self {
            kind: AppErrorKind::ConfigIsInvalid,
            err: anyhow::anyhow!("configuration for {name} is invalid"),
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

/// Workspace State
///
/// Contains graph and list of daemons
/// Workspace graph nodes are stripped from sensitive data
#[derive(Debug, Serialize)]
pub struct WorkspaceState {
    pub nodes: Vec<WorkspaceNode>,
    pub edges: Vec<Edge>,
    pub daemons: Vec<Daemon>,
}

#[derive(Debug)]
pub struct WorkspaceGraph {
    pub nodes: Vec<WorkspaceNode>,
    pub edges: Vec<Edge>,
}

#[derive(Debug, Serialize)]
pub struct WorkspaceNode {
    pub id: uuid::Uuid,
    pub display_name: String,
    pub config: Box<dyn config::Config>,
    pub daemon_id: Option<Uuid>,
    pub x: f64,
    pub y: f64,
}

impl WorkspaceNode {
    pub fn new(
        id: uuid::Uuid,
        display_name: String,
        config: Box<dyn config::Config>,
        daemon_id: Option<Uuid>,
        x: f64,
        y: f64,
    ) -> Self {
        Self {
            id,
            display_name,
            config,
            daemon_id,
            x,
            y,
        }
    }

    pub fn strip_secrets(&mut self, config_registry: &ConfigRegistry) -> Result<()> {
        match config_registry.build_config(self.config.name()) {
            Ok(mut config) => {
                if let Err(e) = deserialize_into_config(&mut *self.config, &mut *config) {
                    // FIXME: emit warning which will be visible in UI?
                    tracing::error!(
                        "failed to deserialize stored config with name: {}: {e}",
                        config.name()
                    );
                };
                config.strip_secrets();
                std::mem::swap(&mut self.config, &mut config);
                Ok(())
            }
            Err(e) => Err(anyhow::anyhow!(
                "failed to build config from config registry for {}: {e}",
                self.config.name()
            ))?,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DaemonGraph {
    pub nodes: Vec<DaemonNode>,
    pub edges: Vec<Edge>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DaemonNode {
    id: uuid::Uuid,
    config: Box<dyn config::Config>,
}

impl DaemonNode {
    fn build_config(&mut self, config_registry: &ConfigRegistry) -> Result<()> {
        let mut new_config = config_registry
            .build_config(self.config.name())
            .map_err(|e| anyhow::anyhow!("failed to build config {}: {e}", self.config.name()))?;
        deserialize_into_config(&mut *self.config, &mut *new_config)
            .map_err(|e| anyhow::anyhow!("failed to deserialize config: {e}"))?;
        std::mem::swap(&mut new_config, &mut self.config);
        Ok(())
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Edge {
    pub from_id: uuid::Uuid,
    pub to_id: uuid::Uuid,
}

#[derive(Debug, Deserialize)]
pub struct WorkspaceUpdate {
    name: String,
    operations: Vec<WorkspaceOperation>,
}

#[derive(Debug, Serialize)]
#[serde(tag = "result")]
pub enum WorkspaceUpdateResult {
    Success,
    Error { kind: String, description: String },
}

impl WorkspaceUpdateResult {
    fn success() -> Self {
        Self::Success
    }

    fn from_app_error(err: AppError) -> Self {
        Self::Error {
            kind: format!("{:?}", err.kind),
            description: err.err.to_string(),
        }
    }
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
    AssignNodeToDaemon {
        node_id: Uuid,
        daemon_id: Uuid,
    },
    UnassignNodeFromDaemon {
        node_id: Uuid,
    },
}

impl WorkspaceOperation {
    fn needs_daemon_notification(&self) -> bool {
        match self {
            WorkspaceOperation::AddNode { .. } => true,
            WorkspaceOperation::RemoveNode(_) => true,
            WorkspaceOperation::AddEdge { .. } => true,
            WorkspaceOperation::RemoveEdge { .. } => true,
            WorkspaceOperation::UpdateNodeConfig { .. } => true,
            WorkspaceOperation::AssignNodeToDaemon { .. } => true,
            WorkspaceOperation::UnassignNodeFromDaemon { .. } => true,
            _ => false,
        }
    }
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

#[derive(Debug, Serialize)]
pub struct Daemon {
    pub id: Uuid,
    pub name: String,
    pub address: Option<String>,
    pub last_seen: Option<DateTime<Utc>>,
    pub joined_at: Option<DateTime<Utc>>,
    pub status: DaemonStatus,
}

#[derive(Debug, Serialize)]
pub enum DaemonStatus {
    Offline,
    Online,
}

impl Default for DaemonStatus {
    fn default() -> Self {
        Self::Offline
    }
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
        let db = Arc::from(self.db);
        Ok(App {
            db: Arc::clone(&db),
            config_registry: config_registry::new()
                .map_err(|e| anyhow::anyhow!("failed to build config registry: {e}"))?,
            certificate_bundle,
            daemon_tracker: daemon_tracker::DaemonTracker::spawn(),
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
    db: Arc<dyn db::DbTrait>,
    config_registry: ConfigRegistry,
    certificate_bundle: CertificateBundle,
    daemon_tracker: daemon_tracker::DaemonTrackerHandle,
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
    pub async fn get_workspace_graph(&self, name: &str) -> Result<WorkspaceState> {
        let daemons = self.db.list_daemons().await?;
        let mut graph = self.db.get_workspace_graph(name).await?;
        graph.nodes.iter_mut().for_each(|node| {
            if let Err(e) = node.strip_secrets(&self.config_registry) {
                tracing::error!("{e}");
            }
        });
        Ok(WorkspaceState {
            nodes: graph.nodes,
            edges: graph.edges,
            daemons,
        })
    }

    pub async fn update_workspace(
        &self,
        updates: &mut [WorkspaceUpdate],
    ) -> Result<Vec<WorkspaceUpdateResult>> {
        let mut result = Vec::with_capacity(updates.len());
        // validate operation
        let validate_operation = |operation: &mut WorkspaceOperation| -> Result<()> {
            if let WorkspaceOperation::AddNode { config, .. } = operation {
                let config_name = config.name();
                let mut default_config = self
                    .config_registry
                    .build_config(config_name)
                    .map_err(|_| AppError::config_not_found(config_name))?;
                deserialize_into_config(&**config, &mut *default_config)
                    .map_err(|_| AppError::invalid_config(config_name))?;
                std::mem::swap(config, &mut default_config);
            }
            Ok(())
        };
        let mut notify_daemons = false;
        for update in updates.iter_mut() {
            let update_result =
                match update
                    .operations
                    .iter_mut()
                    .try_fold(notify_daemons, |mut acc, op| {
                        acc |= op.needs_daemon_notification();
                        validate_operation(op)?;
                        Ok(acc)
                    }) {
                    Err(e) => Err(e),
                    Ok(nd) => {
                        notify_daemons = nd;
                        self.db
                            .update_workspace(&self.config_registry, update)
                            .await
                    }
                };
            let update_result = match update_result {
                Ok(()) => WorkspaceUpdateResult::success(),
                Err(e) if e.is_internal() => Err(e)?,
                Err(e) => WorkspaceUpdateResult::from_app_error(e),
            };
            result.push(update_result);
        }
        // notify all daemons and ask to re-fetch graph.
        if notify_daemons {
            self.daemon_tracker.notify_graph_update().await?;
        }
        Ok(result)
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
        let mut stored_daemons = BTreeMap::from_iter(
            self.db
                .list_daemons()
                .await?
                .into_iter()
                .map(|daemon| (daemon.id, daemon)),
        );
        self.get_online_daemons().await?.into_iter().for_each(|id| {
            if let Some(daemon) = stored_daemons.get_mut(&id) {
                daemon.status = DaemonStatus::Online;
            }
        });
        Ok(stored_daemons.into_values().collect())
    }

    pub async fn daemon_set_last_seen(&self, id: Uuid, timestamp: DateTime<Utc>) -> Result<()> {
        self.db.daemon_set_last_seen(id, timestamp).await?;
        Ok(())
    }

    pub async fn get_online_daemons(&self) -> Result<Vec<Uuid>> {
        self.daemon_tracker.list_daemons().await
    }

    pub async fn daemon_connected(&self, id: Uuid) -> Result<UnboundedReceiver<DaemonMessage>> {
        self.daemon_tracker.daemon_connected(id).await
    }

    pub async fn daemon_disconnected(&self, id: Uuid) -> Result<()> {
        self.daemon_tracker.daemon_disconnected(id).await
    }

    pub async fn get_daemon_graph(&self, id: Uuid) -> Result<DaemonGraph> {
        let mut daemon_graph = self.db.get_daemon_graph(id).await?;
        daemon_graph
            .nodes
            .iter_mut()
            .map(|node| node.build_config(&self.config_registry))
            .collect::<Result<()>>()?;
        Ok(daemon_graph)
    }

    pub async fn set_daemon_name(&self, id: Uuid, name: &str) -> Result<()> {
        self.db.set_daemon_name(id, name).await
    }

    pub async fn unset_daemon_name(&self, id: Uuid) -> Result<()> {
        self.db.unset_daemon_name(id).await
    }
}

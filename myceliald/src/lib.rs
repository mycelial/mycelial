mod control_plane_client;
mod daemon_storage;
mod storage;

use anyhow::Result;
use config_registry::{Config as _Config, ConfigRegistry as _ConfigRegistry};
use control_plane_client::ControlPlaneClientHandle;
use daemon_storage::DaemonStorage;
use serde::{Deserialize, Serialize};
use std::{path::Path, time::Duration};
use storage::{SqliteState, SqliteStorageHandle};
use tokio::sync::mpsc::{
    unbounded_channel, UnboundedReceiver, UnboundedSender, WeakUnboundedSender,
};

pub type SectionChannel = runtime::command_channel::SectionChannel<SqliteState>;
pub type ConfigRegistry = _ConfigRegistry<SectionChannel>;
pub type Config = Box<dyn _Config<SectionChannel>>;

#[derive(Debug)]
pub struct Daemon {
    daemon_storage: DaemonStorage,
    section_storage_handle: SqliteStorageHandle,
    control_plane_client_handle: ControlPlaneClientHandle,
    config_registry: ConfigRegistry,
    rx: UnboundedReceiver<DaemonMessage>,
    weak_tx: WeakUnboundedSender<DaemonMessage>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Graph {
    nodes: Vec<Node>,
    edges: Vec<Edge>,
}

impl Graph {
    pub fn deserialize_node_configs(&mut self, registry: &ConfigRegistry) -> Result<()> {
        self.nodes
            .iter_mut()
            .map(|node| node.deserialize_config(registry))
            .collect()
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Node {
    pub id: uuid::Uuid,
    pub config: Config,
}

impl Node {
    pub fn deserialize_config(&mut self, registry: &ConfigRegistry) -> Result<()> {
        let mut config = registry.deserialize_config(&*self.config).map_err(|e| {
            anyhow::anyhow!(
                "failed to deserialize raw config into config: {}: {e:?}",
                self.config.name()
            )
        })?;
        std::mem::swap(&mut self.config, &mut config);
        Ok(())
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Edge {
    pub from_id: uuid::Uuid,
    pub to_id: uuid::Uuid,
}

#[derive(Debug)]
pub enum DaemonMessage {
    RetryControlPlaneClientInit,
    Graph(Graph),
}

#[derive(Debug)]
pub struct CertifiedKey {
    pub key: String,
    pub certificate: String,
    pub ca_certificate: String,
}

#[derive(Debug, Clone)]
pub struct DaemonHandle {
    tx: UnboundedSender<DaemonMessage>,
}

impl DaemonHandle {
    fn new(tx: &UnboundedSender<DaemonMessage>) -> Self {
        Self { tx: tx.clone() }
    }

    pub fn graph(&self, graph: Graph) -> Result<()> {
        self.tx.send(DaemonMessage::Graph(graph))?;
        Ok(())
    }
}

impl Daemon {
    pub async fn new(database_path: &str) -> Result<Self> {
        let (tx, rx) = unbounded_channel();
        let database_path = Path::new(database_path);
        let daemon_storage = daemon_storage::new(database_path).await?;
        let section_storage_handle = storage::new(database_path).await?;
        let control_plane_client_handle = control_plane_client::new(DaemonHandle::new(&tx));
        Ok(Self {
            daemon_storage,
            section_storage_handle,
            control_plane_client_handle,
            config_registry: config_registry::new()
                .map_err(|e| anyhow::anyhow!("failed to intialize config registry: {e:?}"))?,
            rx,
            weak_tx: tx.clone().downgrade(),
        })
    }

    pub async fn run(&mut self) -> Result<()> {
        self.init_control_plane_client().await?;
        while let Some(message) = self.rx.recv().await {
            match message {
                DaemonMessage::RetryControlPlaneClientInit => {
                    self.init_control_plane_client().await?;
                }
                DaemonMessage::Graph(mut graph) => {
                    match graph.deserialize_node_configs(&self.config_registry) {
                        Err(e) => {
                            tracing::error!("failed to deserialize node configs: {e}");
                            continue;
                        }
                        Ok(()) => (),
                    }
                    tracing::info!("got graph: {graph:#?}");
                }
            }
        }
        Ok(())
    }

    pub async fn join(
        &mut self,
        control_plane_url: &str,
        control_plane_tls_url: &str,
        join_token: &str,
    ) -> Result<()> {
        if self.daemon_storage.get_certified_key().await?.is_some()
            || self.daemon_storage.get_tls_url().await?.is_some()
        {
            tracing::warn!("resetting state");
            self.daemon_storage.reset_state().await?;
        }
        let certifiedkey = self
            .control_plane_client_handle
            .join(control_plane_url, join_token)
            .await?;
        self.daemon_storage
            .store_certified_key(certifiedkey)
            .await?;
        self.daemon_storage
            .store_tls_url(control_plane_tls_url)
            .await?;
        Ok(())
    }

    pub async fn reset(&mut self) -> Result<()> {
        self.daemon_storage.reset_state().await?;
        self.section_storage_handle
            .reset_state()
            .await
            .map_err(|e| anyhow::anyhow!("failed to reset storage state: {e}"))?;
        Ok(())
    }

    pub async fn shutdown(&mut self) -> anyhow::Result<()> {
        //self.scheduler_handle.shutdown().await.ok();
        self.section_storage_handle.shutdown().await.ok();
        self.control_plane_client_handle.shutdown().await.ok();
        Ok(())
    }

    async fn init_control_plane_client(&mut self) -> Result<()> {
        let tls_url = self.daemon_storage.get_tls_url().await?;
        let certifiedkey = self.daemon_storage.get_certified_key().await?;
        let success = match (tls_url, certifiedkey) {
            (Some(tls_url), Some(certifiedkey)) => self
                .control_plane_client_handle
                .set_tls_url(tls_url, certifiedkey)
                .await
                .map_err(|e| tracing::error!("failed to set tls url: {e}"))
                .is_ok(),
            _ => false,
        };
        if !success {
            tracing::info!("connection details are not set, scheduling config check in 10 seconds");
            let tx = self.weak_tx.clone().upgrade();
            tokio::spawn(async move {
                tokio::time::sleep(Duration::from_secs(10)).await;
                if let Some(tx) = tx {
                    tx.send(DaemonMessage::RetryControlPlaneClientInit).ok();
                }
            });
        }
        Ok(())
    }
}

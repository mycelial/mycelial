use crate::{
    control_plane_client::{self, ControlPlaneClientHandle},
    runtime_error::RuntimeError,
    runtime_storage::{self, RuntimeStorage},
    sqlite_storage::{self, SqliteStorageHandle},
    Config, ConfigRegistry, Result,
};
use serde::{Deserialize, Serialize};
use std::{path::Path, time::Duration};
use tokio::sync::mpsc::{
    unbounded_channel, UnboundedReceiver, UnboundedSender, WeakUnboundedSender,
};

#[derive(Debug)]
pub struct Runtime {
    runtime_storage: RuntimeStorage,
    section_storage_handle: SqliteStorageHandle,
    control_plane_client_handle: ControlPlaneClientHandle,
    config_registry: ConfigRegistry,
    rx: UnboundedReceiver<RuntimeMessage>,
    weak_tx: WeakUnboundedSender<RuntimeMessage>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Graph {
    nodes: Vec<Node>,
    edges: Vec<Edge>,
}

impl Graph {
    pub fn new() -> Self {
        Self {
            nodes: vec![],
            edges: vec![],
        }
    }

    pub fn deserialize_node_configs(&mut self, registry: &ConfigRegistry) -> Result<()> {
        self.nodes
            .iter_mut().try_for_each(|node| node.deserialize_config(registry))
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
            RuntimeError::RawConfigDeserializeError {
                config_name: self.config.name().into(),
                raw_config: self.config.as_dyn_config_ref().clone_config(),
            }
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
pub enum RuntimeMessage {
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
pub struct RuntimeHandle {
    tx: UnboundedSender<RuntimeMessage>,
}

impl RuntimeHandle {
    fn new(tx: &UnboundedSender<RuntimeMessage>) -> Self {
        Self { tx: tx.clone() }
    }

    pub fn graph(&self, graph: Graph) -> Result<()> {
        self.tx.send(RuntimeMessage::Graph(graph))?;
        Ok(())
    }
}

impl Runtime {
    pub async fn new(database_path: &str) -> Result<Self> {
        let (tx, rx) = unbounded_channel();
        let database_path = Path::new(database_path);
        let runtime_storage = runtime_storage::new(database_path).await?;
        let section_storage_handle = sqlite_storage::new(database_path).await?;
        let control_plane_client_handle = control_plane_client::new(RuntimeHandle::new(&tx));
        Ok(Self {
            runtime_storage,
            section_storage_handle,
            control_plane_client_handle,
            config_registry: config_registry::new()
                .map_err(RuntimeError::ConfigRegistryInitError)?,
            rx,
            weak_tx: tx.clone().downgrade(),
        })
    }

    pub async fn run(&mut self) -> Result<()> {
        self.init_control_plane_client().await?;
        while let Some(message) = self.rx.recv().await {
            match message {
                RuntimeMessage::RetryControlPlaneClientInit => {
                    self.init_control_plane_client().await?;
                }
                RuntimeMessage::Graph(mut graph) => {
                    if let Err(e) = graph.deserialize_node_configs(&self.config_registry) {
                        tracing::error!("failed to deserialize node configs: {e}");
                        continue;
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
        if self.runtime_storage.get_certified_key().await?.is_some()
            || self.runtime_storage.get_tls_url().await?.is_some()
        {
            tracing::warn!("resetting state");
            self.runtime_storage.reset_state().await?;
        }
        let certifiedkey = self
            .control_plane_client_handle
            .join(control_plane_url, join_token)
            .await?;
        self.runtime_storage
            .store_certified_key(certifiedkey)
            .await?;
        self.runtime_storage
            .store_tls_url(control_plane_tls_url)
            .await?;
        Ok(())
    }

    pub async fn reset(&mut self) -> Result<()> {
        self.runtime_storage.reset_state().await?;
        self.section_storage_handle
            .reset_state()
            .await
            .map_err(RuntimeError::ResetError)?;
        Ok(())
    }

    pub async fn shutdown(&mut self) -> anyhow::Result<()> {
        //self.scheduler_handle.shutdown().await.ok();
        self.section_storage_handle.shutdown().await.ok();
        self.control_plane_client_handle.shutdown().await.ok();
        Ok(())
    }

    async fn init_control_plane_client(&mut self) -> Result<()> {
        let tls_url = self.runtime_storage.get_tls_url().await?;
        let certifiedkey = self.runtime_storage.get_certified_key().await?;
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
                    tx.send(RuntimeMessage::RetryControlPlaneClientInit).ok();
                }
            });
        }
        Ok(())
    }
}

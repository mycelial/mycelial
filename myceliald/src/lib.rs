mod constructors;
mod control_plane_client;
mod daemon_storage;
mod runtime;
mod storage;

use anyhow::Result;
use chrono::{DateTime, Utc};
use control_plane_client::ControlPlaneClientHandle;
use daemon_storage::DaemonStorage;
use pipe::scheduler::SchedulerHandle;
use std::{path::Path, time::Duration};
use storage::SqliteStorageHandle;
use tokio::sync::{
    mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender, WeakUnboundedSender},
    oneshot::Sender as OneshotSender,
};
use uuid::Uuid;

#[derive(Debug)]
pub struct Daemon {
    daemon_storage: DaemonStorage,
    section_storage_handle: SqliteStorageHandle,
    scheduler_handle: SchedulerHandle,
    control_plane_client_handle: ControlPlaneClientHandle,
    rx: UnboundedReceiver<DaemonMessage>,
    weak_tx: WeakUnboundedSender<DaemonMessage>,
}

#[derive(Debug)]
pub enum DaemonMessage {
    RetryControlPlaneClientInit,
    GetOffset {
        reply_to: OneshotSender<Option<Offset>>,
    },
}

#[derive(Debug)]
pub struct CertifiedKey {
    pub key: String,
    pub certificate: String,
    pub ca_certificate: String,
}

#[derive(Debug)]
pub struct Offset {
    id: Uuid,
    timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct DaemonHandle {
    tx: UnboundedSender<DaemonMessage>,
}

impl DaemonHandle {
    fn new(tx: &UnboundedSender<DaemonMessage>) -> Self {
        Self { tx: tx.clone() }
    }

    /// get last known node id and timestamp for control plane client
    pub async fn get_offset(&self) -> Option<(Uuid, DateTime<Utc>)> {
        None
    }
}

impl Daemon {
    pub async fn new(database_path: &str) -> Result<Self> {
        let (tx, rx) = unbounded_channel();
        let database_path = Path::new(database_path);
        let daemon_storage = daemon_storage::new(database_path).await?;
        let section_storage_handle = storage::new(database_path).await?;
        let scheduler_handle = runtime::new(section_storage_handle.clone());
        let control_plane_client_handle = control_plane_client::new(DaemonHandle::new(&tx));
        Ok(Self {
            daemon_storage,
            section_storage_handle,
            scheduler_handle,
            control_plane_client_handle,
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
                DaemonMessage::GetOffset { reply_to } => {
                    reply_to.send(None).ok();
                    //
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
        self.scheduler_handle.shutdown().await.ok();
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

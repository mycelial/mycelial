mod constructors;
mod control_plane_client;
mod daemon_storage;
mod runtime;
mod storage;

use anyhow::Result;
use control_plane_client::ControlPlaneClientHandle;
use daemon_storage::DaemonStorage;
use pipe::scheduler::SchedulerHandle;
use std::path::Path;
use storage::SqliteStorageHandle;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver};

#[derive(Debug)]
pub struct Daemon {
    daemon_storage: DaemonStorage,
    section_storage_handle: SqliteStorageHandle,
    scheduler_handle: SchedulerHandle,
    control_plane_client_handle: ControlPlaneClientHandle,
    rx: UnboundedReceiver<DaemonMessage>,
}

#[derive(Debug)]
pub enum DaemonMessage {}

#[derive(Debug)]
pub struct CertifiedKey {
    pub key: String,
    pub certificate: String,
}

#[derive(Debug)]
enum Reset {
    None,
    // state reset - daemon storage and section storage are wiped
    State,
    // storage path was changed, restart all subsystems with updated path
    Restart,
}

impl Daemon {
    pub async fn new(database_path: &str) -> Result<Self> {
        let (tx, rx) = unbounded_channel();
        let database_path = Path::new(database_path);
        let daemon_storage = daemon_storage::new(database_path).await?;
        let section_storage_handle = storage::new(database_path).await?;
        let scheduler_handle = runtime::new(section_storage_handle.clone());
        let control_plane_client_handle = control_plane_client::new(tx.clone());
        Ok(Self {
            daemon_storage,
            section_storage_handle,
            scheduler_handle,
            control_plane_client_handle,
            rx,
        })
    }

    pub async fn run(&mut self) -> Result<()> {
        self.init_control_plane_client().await?;
        while let Some(message) = self.rx.recv().await {
            unimplemented!()
        }
        Ok(())
    }

    pub async fn join(
        &mut self,
        control_plane_url: &str,
        control_plane_tls_url: &str,
        join_token: &str,
    ) -> Result<()> {
        // FIXME: check if already joined
        let certifiedkey = self
            .control_plane_client_handle
            .join(control_plane_url, join_token)
            .await?;
        Ok(())
    }

    pub async fn shutdown(&mut self) -> anyhow::Result<()> {
        self.scheduler_handle.shutdown().await.ok();
        self.section_storage_handle.shutdown().await.ok();
        Ok(())
    }

    async fn init_control_plane_client(&mut self) -> Result<()> {
        let tls_url = self.daemon_storage.get_tls_url().await?;
        let certifiedkey = self.daemon_storage.get_certified_key().await?;
        match (tls_url, certifiedkey) {
            (Some(tls_url), Some(certifiedkey)) => {
                self.control_plane_client_handle
                    .set_tls_url(tls_url, certifiedkey)
                    .await?;
            }
            (None, _) => tracing::warn!("tls url is not set"),
            (_, None) => tracing::warn!("certificate and key is absent"),
        };
        Ok(())
    }
}

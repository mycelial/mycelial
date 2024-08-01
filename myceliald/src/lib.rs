mod constructors;
mod daemon_storage;
mod runtime;
mod storage;

use anyhow::{Context, Result};
use common::ClientConfig;
use common::PipeConfig;
use daemon_storage::DaemonStorage;
use pipe::scheduler::SchedulerHandle;
use sha2::Digest;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use storage::SqliteStorageHandle;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver};

async fn read_config(path: &Path) -> Result<ClientConfig> {
    let config = tokio::fs::read_to_string(path)
        .await
        .context(format!("failed to open config file at '{path:?}'"))?;
    Ok(toml::from_str(&config)?)
}

#[derive(Debug)]
pub struct Daemon {
    daemon_storage: DaemonStorage,
    section_storage_handle: SqliteStorageHandle,
    scheduler_handle: SchedulerHandle,
    rx: UnboundedReceiver<DaemonMessage>,
}

#[derive(Debug)]
pub enum DaemonMessage {
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
        let (_tx, rx) = unbounded_channel();
        let database_path = Path::new(database_path);
        let daemon_storage = daemon_storage::new(database_path).await?;
        let section_storage_handle = storage::new(database_path).await?;
        let scheduler_handle = runtime::new(section_storage_handle.clone());
        Ok(Self {
            daemon_storage,
            section_storage_handle,
            scheduler_handle,
            rx,
        })
    }

    pub async fn start(database_path: &str) -> Result<()> {
        loop {
            let mut daemon = Self::new(database_path)
                .await
                .context("failed to initialize application")?;
            match daemon.run().await {
                Ok(Reset::State) => {
                    tracing::info!("daemon state was reset, restarting subsystems");
                    continue;
                }
                Ok(Reset::Restart) => {
                    tracing::info!("daemon restarting subsystems");
                    continue;
                }
                Err(e) => return Err(e),
                _ => return Ok(()),
            }
        }
    }

    async fn run(&mut self) -> Result<Reset> {
        while let Some(message) = self.rx.recv().await {
            match message {
                _ => unimplemented!()
            }
        }
        Ok(Reset::None)
    }

    async fn shutdown(&mut self) -> anyhow::Result<()> {
        self.scheduler_handle.shutdown().await.ok();
        self.section_storage_handle.shutdown().await.ok();
        Ok(())
    }
}

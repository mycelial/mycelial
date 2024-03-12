#![allow(unused)]
mod config_watcher;
mod constructors;
mod daemon_storage;
mod http_client;
mod runtime;
mod storage;

use anyhow::{anyhow, Context, Result};
use clap::Parser;
use common::ClientConfig;
use common::PipeConfig;
use config_watcher::ConfigWatcherHandle;
use daemon_storage::{DaemonInfo, DaemonStorage, ServerInfo};
use http_client::{HttpClientEvent, HttpClientHandle};
use pipe::{config::Config, scheduler::SchedulerHandle};
use sha2::Digest;
use std::collections::HashSet;
use std::env::current_dir;
use std::path::Path;
use std::path::PathBuf;
use storage::SqliteStorageHandle;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};

use crate::config_watcher::ConfigWatcherEvent;

#[derive(Parser)]
struct Cli {
    /// Path to the TOML config file
    #[clap(short, long, env = "CONFIG_PATH")]
    config: String,
}

async fn read_config(path: &Path) -> Result<ClientConfig> {
    let config = tokio::fs::read_to_string(path)
        .await
        .context(format!("failed to open config file at '{path:?}'"))?;
    Ok(toml::from_str(&config)?)
}

struct Daemon {
    config: ClientConfig,
    config_path: PathBuf,
    storage_path: PathBuf,
    // FIXME: common crate
    config_watcher_handle: ConfigWatcherHandle,
    daemon_storage: DaemonStorage,
    section_storage_handle: SqliteStorageHandle,
    scheduler_handle: SchedulerHandle,
    http_client_handle: HttpClientHandle,
    rx: UnboundedReceiver<DaemonMessage>,
}

#[derive(Debug)]
pub enum DaemonMessage {
    ConfigWatcher(ConfigWatcherEvent),
    HttpClient(HttpClientEvent),
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
    async fn new(cli: Cli) -> Result<Self> {
        let mut config_path = PathBuf::from(cli.config);
        if !config_path.is_absolute() {
            config_path = current_dir()?.join(config_path)
        }
        let (tx, rx) = unbounded_channel();
        let config = read_config(config_path.as_path()).await?;
        let mut storage_path = PathBuf::from(&config.node.storage_path);
        if !storage_path.is_absolute() {
            storage_path = current_dir()?.join(storage_path);
        }
        let daemon_storage = daemon_storage::new(storage_path.as_path()).await?;
        let section_storage_handle = storage::new(storage_path.as_path()).await?;
        let scheduler_handle = runtime::new(section_storage_handle.clone());
        let config_watcher_handle = config_watcher::new(config_path.as_path(), tx.clone());
        let http_client_handle = http_client::new(tx);
        Ok(Self {
            config,
            config_path,
            storage_path,
            config_watcher_handle,
            daemon_storage,
            section_storage_handle,
            scheduler_handle,
            http_client_handle,
            rx,
        })
    }

    async fn run(&mut self) -> Result<Reset> {
        if let reset @ (Reset::State | Reset::Restart) = self.maybe_reset_state().await? {
            self.shutdown().await.ok();
            return Ok(reset);
        }

        self.setup_http_client().await?;
        self.maybe_resubmit_sections().await?;
        self.reschedule_pipes().await?;

        while let Some(message) = self.rx.recv().await {
            match message {
                DaemonMessage::ConfigWatcher(ConfigWatcherEvent::Modified) => {
                    match read_config(&self.config_path).await {
                        Ok(config) => self.config = config,
                        Err(e) => {
                            tracing::error!("failed to read config: {e:?}");
                            continue;
                        }
                    };
                    match self.maybe_reset_state().await? {
                        Reset::None => {
                            self.maybe_resubmit_sections().await?;
                        }
                        reset @ (Reset::State | Reset::Restart) => {
                            self.shutdown().await.ok();
                            return Ok(reset);
                        }
                    }
                }
                DaemonMessage::HttpClient(HttpClientEvent::Configs { configs }) => {
                    tracing::debug!("got new configs: {configs:?}");
                    self.daemon_storage.store_pipes(configs.as_slice()).await?;
                    self.reschedule_pipes().await?;
                }
                DaemonMessage::HttpClient(HttpClientEvent::Credentials {
                    client_id,
                    client_secret,
                }) => {
                    // client was able to 'exchange' auth token for credentials
                    self.daemon_storage
                        .store_http_credentials(client_id, client_secret)
                        .await?;
                }
                DaemonMessage::HttpClient(HttpClientEvent::SectionsSubmitted { config_hash }) => {
                    self.daemon_storage
                        .store_config_hash(config_hash.as_str())
                        .await?;
                }
            }
        }
        Ok(Reset::None)
    }

    // set credentials to http client
    async fn setup_http_client(&mut self) -> anyhow::Result<()> {
        let server_info = self.daemon_storage.retrieve_server_info().await?;
        let daemon_info = self.daemon_storage.retrieve_daemon_info().await?;
        let http_credentials = self.daemon_storage.retrieve_http_credentials().await?;
        match (&server_info, &daemon_info, &http_credentials) {
            (Some(ServerInfo{ endpoint, token }), Some(DaemonInfo{ unique_id, display_name }), creds) => {
                let (client_id, client_secret) = creds
                    .as_ref()
                    .map(|creds| (Some(creds.client_id.as_str()), Some(creds.client_secret.as_str())))
                    .unwrap_or((None, None));
                self.http_client_handle.set_connection(
                    endpoint, token, unique_id, display_name, client_id, client_secret
                ).await?;
            },
            (None, _, _) =>
                tracing::error!("server info is missing, http client can't poll configs without endpoint/token"),
            (_, None, _) =>
                tracing::error!("daemon info is missing, http client can't submit sections without display_name/unique_id"),
        };
        Ok(())
    }

    // check if sections need to be submitted to control plane
    async fn maybe_resubmit_sections(&mut self) -> anyhow::Result<()> {
        let stored_config_hash = self.daemon_storage.retrieve_config_hash().await?;

        // serialize current configuration
        let config_hash = format!("{:x}", sha2::Sha256::digest(toml::to_string(&self.config)?));
        match stored_config_hash {
            Some(hash) if config_hash == hash => (),
            _ => {
                let sources = self.config.sources.clone();
                let destinations = self.config.destinations.clone();
                self.http_client_handle
                    .submit_sections(config_hash, sources, destinations)
                    .await?;
            }
        };
        Ok(())
    }

    //
    async fn reschedule_pipes(&mut self) -> anyhow::Result<()> {
        let mut started = HashSet::new();
        for pipe in self.daemon_storage.retrieve_pipes().await? {
            let id = pipe.id;
            match pipe.try_into() {
                Ok(conf) => {
                    self.scheduler_handle
                        .add_pipe(id, conf)
                        .await
                        .map_err(|e| anyhow!(e))?;
                    started.insert(id);
                }
                Err(e) => {
                    tracing::error!("failed to convert pipe config into scheduler config: {e}");
                }
            }
        }
        let mut scheduled = HashSet::from_iter(
            self.scheduler_handle
                .list_ids()
                .await
                .map_err(|e| anyhow!(e))?
                .into_iter(),
        );
        for id in scheduled.difference(&started) {
            self.scheduler_handle
                .remove_pipe(*id)
                .await
                .map_err(|e| anyhow!(e))?;
        }
        Ok(())
    }

    // check if daemon needs reset/restart
    // - change in endpoint/token force state reset and restart
    // - change in unique_id/display_name force state reset and restart
    // - change to storage path force restart
    async fn maybe_reset_state(&mut self) -> Result<Reset> {
        let conf_storage_path = PathBuf::from(&self.config.node.storage_path);
        let conf_storage_path = current_dir()?.join(conf_storage_path);
        let conf_display_name = &self.config.node.display_name;
        let conf_unique_id = &self.config.node.unique_id;
        let conf_endpoint = &self.config.server.endpoint;
        let conf_token = &self.config.node.auth_token;

        let daemon_info = self.daemon_storage.retrieve_daemon_info().await?;
        let server_info = self.daemon_storage.retrieve_server_info().await?;
        let reset_type = match (self.storage_path.as_path(), daemon_info, server_info) {
            (path, _, _) if path != conf_storage_path => Reset::Restart,
            (
                _,
                Some(DaemonInfo {
                    ref display_name,
                    ref unique_id,
                }),
                _,
            ) if display_name != conf_display_name || unique_id != conf_unique_id => Reset::State,
            (
                _,
                _,
                Some(ServerInfo {
                    ref endpoint,
                    ref token,
                }),
            ) if endpoint != conf_endpoint || token != conf_token => Reset::State,
            (_, server_info, daemon_info) if server_info.is_none() || daemon_info.is_none() => {
                self.daemon_storage
                    .store_daemon_info(conf_display_name, conf_unique_id)
                    .await?;
                self.daemon_storage
                    .store_server_info(conf_endpoint, conf_token)
                    .await?;
                Reset::None
            }
            _ => Reset::None,
        };
        if let Reset::State = reset_type {
            tracing::info!("resetting state");
            self.daemon_storage.reset_state().await?;
            self.section_storage_handle
                .reset_state()
                .await
                .map_err(|e| anyhow!(e))?;
            for id in self
                .scheduler_handle
                .list_ids()
                .await
                .map_err(|e| anyhow!(e))?
            {
                self.scheduler_handle
                    .remove_pipe(id)
                    .await
                    .map_err(|e| anyhow!(e))?;
            }
        };
        Ok(reset_type)
    }

    async fn shutdown(&mut self) -> anyhow::Result<()> {
        self.http_client_handle.shutdown().await.ok();
        self.scheduler_handle.shutdown().await.ok();
        self.section_storage_handle.shutdown().await.ok();
        self.config_watcher_handle.shutdown().await.ok();
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    loop {
        let mut daemon = Daemon::new(Cli::try_parse()?)
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

use crate::DaemonMessage;

use anyhow::Result;
use notify::event::ModifyKind;
use notify::{EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use notify_debouncer_full::{new_debouncer, DebounceEventResult, Debouncer, FileIdMap};
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::sync::{
    mpsc::{channel, Receiver, Sender, UnboundedSender},
    oneshot::{channel as oneshot_channel, Sender as OneshotSender},
};

#[derive(Debug)]
pub struct ConfigWatcher {
    config_path: PathBuf,
    tx: UnboundedSender<DaemonMessage>,
}

#[derive(Debug)]
pub enum ConfigWatcherEvent {
    Modified,
}

enum FsEvent {
    Modified,
    Removed,
    Error,
}

#[derive(Debug)]
enum ConfigWatcherMessage {
    Shutdown { reply_to: OneshotSender<()> },
}

// Track state of config watcher
#[derive(Debug, PartialEq)]
enum LastState {
    Ok,
    Failed,
    ConfigRemoved,
}

impl ConfigWatcher {
    fn new(config_path: &Path, tx: UnboundedSender<DaemonMessage>) -> Self {
        Self {
            config_path: config_path.into(),
            tx,
        }
    }

    fn spawn(self) -> ConfigWatcherHandle {
        let (tx, mut rx) = channel(1);
        tokio::spawn(async move { self.enter_loop(&mut rx).await });
        ConfigWatcherHandle { tx }
    }

    async fn enter_loop(self, rx: &mut Receiver<ConfigWatcherMessage>) {
        let mut maybe_debouncer = None;
        let (watcher_tx, mut watcher_rx) = channel(1);
        let mut last_state = LastState::Ok;
        loop {
            tokio::select! {
                msg = rx.recv() => {
                    match msg {
                        None => {
                            tracing::info!("handle was dropped, shutting down");
                            return
                        },
                        Some(ConfigWatcherMessage::Shutdown{ reply_to }) => {
                            tracing::info!("shutting down");
                            if let Some(d) = maybe_debouncer.take() { d.stop_nonblocking() }
                            reply_to.send(()).ok();
                            return
                        }
                    }
                },
                _ = self.fs_events(&watcher_tx, &mut watcher_rx, &mut last_state, &mut maybe_debouncer) => {}
            }
        }
    }

    async fn fs_events(
        &self,
        tx: &Sender<FsEvent>,
        rx: &mut Receiver<FsEvent>,
        last_state: &mut LastState,
        maybe_debouncer: &mut Option<Debouncer<RecommendedWatcher, FileIdMap>>,
    ) {
        *last_state = match maybe_debouncer.is_some() {
            false => match self.init_debouncer(tx.clone()) {
                Ok(d) => {
                    tracing::info!("watching {:?}", self.config_path.as_path());
                    *maybe_debouncer = Some(d);
                    if last_state != &LastState::Ok {
                        self.tx
                            .send(DaemonMessage::ConfigWatcher(ConfigWatcherEvent::Modified))
                            .ok();
                    }
                    LastState::Ok
                }
                Err(e) => {
                    tracing::error!("failed to start fs watcher, error: {e:?}, retrying in 3 sec");
                    tokio::time::sleep(Duration::from_secs(3)).await;
                    LastState::Failed
                }
            },
            true => match rx.recv().await {
                None => {
                    tracing::error!("fs watcher is down");
                    maybe_debouncer.take();
                    LastState::Failed
                }
                Some(event) => match event {
                    FsEvent::Modified => {
                        tracing::info!("config modified");
                        self.tx
                            .send(DaemonMessage::ConfigWatcher(ConfigWatcherEvent::Modified))
                            .ok();
                        LastState::Ok
                    }
                    FsEvent::Removed => {
                        tracing::info!("config was removed");
                        if let Some(d) = maybe_debouncer.take() {
                            d.stop_nonblocking()
                        }
                        LastState::ConfigRemoved
                    }
                    FsEvent::Error => {
                        tracing::info!("fs watcher encountered errors");
                        if let Some(d) = maybe_debouncer.take() {
                            d.stop_nonblocking()
                        }
                        LastState::Failed
                    }
                },
            },
        }
    }

    fn init_debouncer(
        &self,
        tx: Sender<FsEvent>,
    ) -> Result<Debouncer<RecommendedWatcher, FileIdMap>> {
        let event_handler = move |events: DebounceEventResult| {
            match events {
                Ok(events) => {
                    let config_removed = events.into_iter().any(|event| {
                        matches!(event.event.kind, EventKind::Modify(ModifyKind::Name(_)))
                            || matches!(event.event.kind, EventKind::Remove(_))
                    });
                    // config file was removed, drop sender to re-initialize file watcher
                    let event = match config_removed {
                        true => FsEvent::Removed,
                        false => FsEvent::Modified,
                    };
                    tx.blocking_send(event).ok();
                }
                Err(errors) => {
                    tracing::error!("fs watcher errors: {:?}", errors);
                    tx.blocking_send(FsEvent::Error).ok();
                }
            }
        };
        let mut debouncer = new_debouncer(Duration::from_secs(1), None, event_handler)?;
        debouncer
            .watcher()
            .watch(self.config_path.as_path(), RecursiveMode::NonRecursive)?;
        Ok(debouncer)
    }
}

#[derive(Debug)]
pub struct ConfigWatcherHandle {
    tx: Sender<ConfigWatcherMessage>,
}

impl ConfigWatcherHandle {
    pub async fn shutdown(&self) -> anyhow::Result<()> {
        let (reply_to, rx) = oneshot_channel();
        let message = ConfigWatcherMessage::Shutdown { reply_to };
        self.tx.send(message).await?;
        Ok(rx.await?)
    }
}

pub fn new(path: &Path, tx: UnboundedSender<DaemonMessage>) -> ConfigWatcherHandle {
    ConfigWatcher::new(path, tx).spawn()
}

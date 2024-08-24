use std::collections::BTreeMap;

use crate::app::Result;
use tokio::sync::{
    mpsc::{channel, unbounded_channel, Receiver, Sender, UnboundedReceiver, UnboundedSender},
    oneshot::{channel as oneshot_channel, Sender as OneshotSender},
};
use uuid::Uuid;
pub struct DaemonTracker {}

impl DaemonTracker {
    pub fn spawn() -> DaemonTrackerHandle {
        let (tx, rx) = channel(8);
        tokio::spawn(async move {
            let app = DaemonTracker {};
            if let Err(e) = app.enter_loop(rx).await {
                tracing::error!("app daemon_tracker is down: {e}");
            }
        });
        DaemonTrackerHandle { tx }
    }

    async fn enter_loop(&self, mut rx: Receiver<DaemonTrackerMessage>) -> Result<()> {
        let mut daemons = BTreeMap::new();
        while let Some(message) = rx.recv().await {
            match message {
                DaemonTrackerMessage::DaemonConnected { id, reply_to } => {
                    tracing::info!("daemon connected: {}", id);
                    let (tx, rx) = unbounded_channel();
                    let daemon_handle = DaemonHandle { tx };
                    daemons.insert(id, daemon_handle);
                    reply_to.send(rx).ok();
                }
                DaemonTrackerMessage::DaemonDisconnected { id, reply_to } => {
                    tracing::info!("daemon disconnected: {id}");
                    daemons.remove(&id);
                    reply_to.send(()).ok();
                }
                DaemonTrackerMessage::ListDaemons { reply_to } => {
                    reply_to.send(daemons.keys().copied().collect()).ok();
                },
                DaemonTrackerMessage::NotifyGraphUpdate => {
                    for daemon in daemons.values() {
                        daemon.notify_graph_update();
                    }
                }
            }
        }
        Err(anyhow::anyhow!("channel closed"))?
    }
}

pub enum DaemonMessage {
    NotifyGraphUpdate,
}

pub struct DaemonHandle {
    tx: UnboundedSender<DaemonMessage>
}

impl DaemonHandle {
    pub fn notify_graph_update(&self) {
        self.tx.send(DaemonMessage::NotifyGraphUpdate).ok();
    }
}

enum DaemonTrackerMessage {
    DaemonConnected {
        id: Uuid,
        reply_to: OneshotSender<UnboundedReceiver<DaemonMessage>>,
    },
    DaemonDisconnected {
        id: Uuid,
        reply_to: OneshotSender<()>,
    },
    ListDaemons {
        reply_to: OneshotSender<Vec<Uuid>>,
    },
    NotifyGraphUpdate,
}

pub struct DaemonTrackerHandle {
    tx: Sender<DaemonTrackerMessage>,
}

impl DaemonTrackerHandle {
    pub async fn daemon_connected(&self, id: Uuid) -> Result<UnboundedReceiver<DaemonMessage>> {
        let (reply_to, rx) = oneshot_channel();
        let message = DaemonTrackerMessage::DaemonConnected { id, reply_to };
        self.tx.send(message).await?;
        Ok(rx.await?)
    }

    pub async fn daemon_disconnected(&self, id: Uuid) -> Result<()> {
        let (reply_to, rx) = oneshot_channel();
        let message = DaemonTrackerMessage::DaemonDisconnected { id, reply_to };
        self.tx.send(message).await?;
        Ok(rx.await?)
    }

    pub async fn list_daemons(&self) -> Result<Vec<Uuid>> {
        let (reply_to, rx) = oneshot_channel();
        let message = DaemonTrackerMessage::ListDaemons { reply_to };
        self.tx.send(message).await?;
        Ok(rx.await?)
    }
    
    pub async fn notify_graph_update(&self) -> Result<()> {
        self.tx.send(DaemonTrackerMessage::NotifyGraphUpdate).await?;
        Ok(())
    }
}

use crate::app::Result;
use tokio::sync::{
    mpsc::{channel, Receiver, Sender},
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
        let mut daemons = std::collections::HashSet::new();
        while let Some(message) = rx.recv().await {
            match message {
                DaemonTrackerMessage::DaemonConnected { id, reply_to } => {
                    tracing::info!("daemon connected: {id}");
                    daemons.insert(id);
                    reply_to.send(()).ok();
                }
                DaemonTrackerMessage::DaemonDisconnected { id, reply_to } => {
                    tracing::info!("daemon disconnected: {id}");
                    daemons.remove(&id);
                    reply_to.send(()).ok();
                }
                DaemonTrackerMessage::ListDaemons { reply_to } => {
                    reply_to.send(daemons.iter().copied().collect()).ok();
                }
            }
        }
        Err(anyhow::anyhow!("channel closed"))?
    }
}

enum DaemonTrackerMessage {
    DaemonConnected {
        id: Uuid,
        reply_to: OneshotSender<()>,
    },
    DaemonDisconnected {
        id: Uuid,
        reply_to: OneshotSender<()>,
    },
    ListDaemons {
        reply_to: OneshotSender<Vec<Uuid>>,
    },
}

pub struct DaemonTrackerHandle {
    tx: Sender<DaemonTrackerMessage>,
}

impl DaemonTrackerHandle {
    pub async fn daemon_connected(&self, id: Uuid) -> Result<()> {
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
}

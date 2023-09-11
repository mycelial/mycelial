//! Command channel

use std::any::Any;

use futures::{Sink, Stream};
use tokio::sync::mpsc::{
    channel, unbounded_channel, Receiver, Sender, UnboundedReceiver, UnboundedSender,
    WeakUnboundedSender,
};
use tokio::sync::oneshot::{channel as oneshot_channel, Sender as OneshotSender};
use tokio_stream::wrappers::{ReceiverStream, UnboundedReceiverStream};
use tokio_util::sync::PollSender;

use super::state::State;
use super::types::SectionError;

#[derive(Debug)]
pub enum Command {
    // Message Acknowledgement
    Ack(Box<dyn Any + Send + Sync + 'static>),

    // Store state
    StoreState {
        id: u64,
        state: State,
    },

    // Retrieve state
    RetrieveState {
        id: u64,
        reply_to: OneshotSender<Option<State>>,
    },

    // Signal for section to stop
    Stop,

    // Signal from section to pipe
    Stopped {
        id: u64,
    },

    // Logging
    Log {
        id: u64,
    },
}

/// Pipe root channel
pub struct RootChannel {
    pub tx: Sender<Command>,
    pub rx: Receiver<Command>,
}

impl RootChannel {
    pub fn new() -> Self {
        let (tx, rx) = channel(1);
        Self { tx, rx }
    }

    // create new channel for section
    pub fn section_channel(&self, section_id: u64) -> (UnboundedSender<Command>, SectionChannel) {
        let pipe_tx = self.tx.clone();
        let (section_tx, section_rx) = unbounded_channel::<Command>();
        let weak_section_tx = section_tx.clone().downgrade();
        (
            section_tx,
            SectionChannel::new(section_id, pipe_tx, section_rx, weak_section_tx),
        )
    }

    #[allow(dead_code)] // fixme
    pub fn split(Self { tx, rx }: Self) -> (impl Sink<Command>, impl Stream<Item = Command>) {
        (PollSender::new(tx), ReceiverStream::new(rx))
    }
}

pub struct SectionChannel {
    pub section_id: u64,
    // Sender to Pipe
    pub tx: Sender<Command>,
    // Receiver from Pipe
    pub rx: Option<UnboundedReceiverStream<Command>>,
    // Weak ref to own Sender
    pub self_weak_tx: WeakSenderWrapper,
}

impl SectionChannel {
    pub fn new(
        section_id: u64,
        tx: Sender<Command>,
        rx: UnboundedReceiver<Command>,
        self_weak_tx: WeakUnboundedSender<Command>,
    ) -> Self {
        Self {
            section_id,
            tx,
            rx: Some(UnboundedReceiverStream::new(rx)),
            self_weak_tx: WeakSenderWrapper::new(self_weak_tx),
        }
    }

    pub async fn store_state(&self, state: State) -> Result<(), SectionError> {
        Ok(self
            .tx
            .send(Command::StoreState {
                id: self.section_id,
                state,
            })
            .await?)
    }

    pub async fn retrieve_state(&self) -> Result<Option<State>, SectionError> {
        let (tx, rx) = oneshot_channel::<Option<State>>();
        self.tx
            .send(Command::RetrieveState {
                id: self.section_id,
                reply_to: tx,
            })
            .await?;
        Ok(rx.await?)
    }

    pub fn split(mut self) -> (Self, impl Stream<Item = Command>) {
        let stream = self.rx.take().unwrap();
        (self, stream)
    }
}

impl Drop for SectionChannel {
    fn drop(&mut self) {
        let _ = self.tx.try_send(Command::Stopped {
            id: self.section_id,
        });
    }
}

#[derive(Clone)]
pub struct WeakSenderWrapper {
    tx: WeakUnboundedSender<Command>,
}

impl WeakSenderWrapper {
    pub fn new(tx: WeakUnboundedSender<Command>) -> Self {
        Self { tx }
    }

    pub fn upgrade(&self) -> Option<UnboundedSender<Command>> {
        self.tx.clone().upgrade()
    }
}

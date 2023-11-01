//! Root channel implementation for tokio

use tokio::sync::mpsc::{
    unbounded_channel, UnboundedReceiver, UnboundedSender, WeakUnboundedSender,
};
use tokio::sync::oneshot::{
    channel as oneshot_channel, Receiver as OneshotReceiver, Sender as OneshotSender,
};

use section::{
    async_trait, Command, ReplyTo, RootChannel as RootChannelTrait,
    SectionChannel as SectionChannelTrait, SectionRequest as _SectionRequest, State as StateTrait,
    WeakSectionChannel as WeakSectionChannelTrait,
};
use std::any::Any;
use std::collections::BTreeMap;
use std::marker::PhantomData;

pub type SectionRequest<S> = _SectionRequest<S, OneshotReply<Option<S>>, OneshotReply<()>>;

pub struct OneshotReply<T> {
    tx: OneshotSender<T>,
}

impl<T> OneshotReply<T> {
    fn new() -> (OneshotReply<T>, OneshotReceiver<T>) {
        let (tx, rx) = oneshot_channel();
        (OneshotReply { tx }, rx)
    }
}

#[async_trait]
impl<T: Send> ReplyTo for OneshotReply<T> {
    type With = T;
    type Error = ChanError;

    async fn reply(self, with: Self::With) -> Result<(), Self::Error> {
        self.tx.send(with).map_err(|_| ChanError::Closed)
    }
}

#[derive(Debug)]
pub enum ChanError {
    // channel closed
    Closed,
    // attempt to add section with id which already exists in root chan section handles
    SectionExists,
    // attempt to send command to section by id which doesn't exist in section handles
    NoSuchSection,
}

impl std::fmt::Display for ChanError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl std::error::Error for ChanError {}

// Root channel
pub struct RootChannel<S: StateTrait> {
    rx: UnboundedReceiver<SectionRequest<S>>,
    tx: UnboundedSender<SectionRequest<S>>,
    section_handles: BTreeMap<u64, UnboundedSender<Command>>,
}

#[async_trait]
impl<S: StateTrait> RootChannelTrait for RootChannel<S> {
    type Error = ChanError;
    type SectionChannel = SectionChannel<S>;

    fn new() -> Self {
        let (tx, rx) = unbounded_channel::<SectionRequest<S>>();
        Self {
            tx,
            rx,
            section_handles: BTreeMap::new(),
        }
    }

    fn add_section(&mut self, section_id: u64) -> Result<Self::SectionChannel, Self::Error> {
        if self.section_handles.contains_key(&section_id) {
            return Err(ChanError::SectionExists);
        }
        let (section_tx, section_rx) = unbounded_channel();
        let weak_section_tx = section_tx.clone().downgrade();
        self.section_handles.insert(section_id, section_tx);
        Ok(SectionChannel::new(
            section_id,
            self.tx.clone(),
            section_rx,
            weak_section_tx,
        ))
    }

    fn remove_section(&mut self, section_id: u64) -> Result<(), Self::Error> {
        match self.section_handles.remove(&section_id) {
            Some(_) => Ok(()),
            None => Err(ChanError::NoSuchSection),
        }
    }

    async fn recv(&mut self) -> Result<SectionRequest<S>, Self::Error> {
        match self.rx.recv().await {
            Some(msg) => Ok(msg),
            None => Err(ChanError::Closed),
        }
    }

    async fn send(&mut self, section_id: u64, command: Command) -> Result<(), Self::Error> {
        let section_handle = match self.section_handles.get(&section_id) {
            Some(section) => section,
            None => return Err(ChanError::NoSuchSection),
        };
        section_handle.send(command).map_err(|_| ChanError::Closed)
    }
}

// section channels
pub struct SectionChannel<S>
where
    S: StateTrait,
{
    id: u64,
    root_tx: UnboundedSender<SectionRequest<S>>,
    rx: UnboundedReceiver<Command>,
    weak_tx: WeakUnboundedSender<Command>,
    _marker: PhantomData<S>,
}

impl<S: StateTrait> SectionChannel<S> {
    fn new(
        id: u64,
        root_tx: UnboundedSender<SectionRequest<S>>,
        rx: UnboundedReceiver<Command>,
        weak_tx: WeakUnboundedSender<Command>,
    ) -> Self {
        Self {
            id,
            root_tx,
            rx,
            weak_tx,
            _marker: PhantomData,
        }
    }
}

#[async_trait]
impl<S: StateTrait> SectionChannelTrait for SectionChannel<S> {
    type State = S;
    type Error = ChanError;
    type WeakChannel = WeakSectionChannel;
    type ReplyStoreState = OneshotReply<()>;
    type ReplyRetrieveState = OneshotReply<Option<Self::State>>;

    // request to runtime
    async fn retrieve_state(&mut self) -> Result<Option<Self::State>, Self::Error> {
        let (reply_to, rx) = OneshotReply::new();
        self.root_tx
            .send(SectionRequest::RetrieveState {
                id: self.id,
                reply_to,
            })
            .map_err(|_| ChanError::Closed)?;
        rx.await.map_err(|_| ChanError::Closed)
    }

    // request to runtime
    async fn store_state(&mut self, state: Self::State) -> Result<(), Self::Error> {
        let (reply_to, rx) = OneshotReply::new();
        self.root_tx
            .send(SectionRequest::StoreState {
                id: self.id,
                state,
                reply_to,
            })
            .map_err(|_| ChanError::Closed)?;
        rx.await.map_err(|_| ChanError::Closed)
    }

    // request to runtime
    async fn log<T: Into<String> + Send>(&mut self, log: T) -> Result<(), Self::Error> {
        self.root_tx
            .send(SectionRequest::Log {
                id: self.id,
                message: log.into(),
            })
            .map_err(|_| ChanError::Closed)
    }

    // request from runtime or from own weak ref?
    async fn recv(&mut self) -> Result<Command, Self::Error> {
        match self.rx.recv().await {
            Some(cmd) => Ok(cmd),
            None => Err(ChanError::Closed),
        }
    }

    // weak reference to self which can send Command messages
    // used for message acks
    fn weak_chan(&self) -> Self::WeakChannel {
        let weak_tx = self.weak_tx.clone();
        WeakSectionChannel { weak_tx }
    }
}

impl<S: StateTrait> Drop for SectionChannel<S> {
    fn drop(&mut self) {
        self.root_tx
            .send(SectionRequest::Stopped { id: self.id })
            .ok();
    }
}

pub struct WeakSectionChannel {
    weak_tx: WeakUnboundedSender<Command>,
}

#[async_trait]
impl WeakSectionChannelTrait for WeakSectionChannel {
    async fn ack(self, payload: Box<dyn Any + Send + Sync + 'static>) {
        if let Some(tx) = self.weak_tx.upgrade() {
            tx.send(Command::Ack(payload)).ok();
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use section::dummy::DummyState;

    #[test]
    fn send() {
        fn test_send<T: Send>(_: T) {}
        let mut root_chan = RootChannel::<DummyState>::new();
        let section_chan = root_chan.add_section(0).unwrap();
        let weak_chan = section_chan.weak_chan();
        test_send(root_chan);
        test_send(section_chan);
        test_send(weak_chan);
    }
}

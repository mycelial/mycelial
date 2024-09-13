//! Section channel implementation on top of Tokio

use tokio::sync::mpsc::{
    unbounded_channel, UnboundedReceiver, UnboundedSender, WeakUnboundedSender,
};
use tokio::sync::oneshot::{
    channel as oneshot_channel, Receiver as OneshotReceiver, Sender as OneshotSender,
};

use section::{
    command_channel::{
        Command, ReplyTo, RootChannel as RootChannelTrait, SectionChannel as SectionChannelTrait,
        SectionRequest as _SectionRequest, WeakSectionChannel as WeakSectionChannelTrait,
    },
    state::State as StateTrait,
};
use uuid::Uuid;
use std::any::Any;
use std::collections::BTreeMap;
use std::marker::PhantomData;

pub type SectionRequest<Id, S> = _SectionRequest<Id, S, OneshotReply<Option<S>>, OneshotReply<()>>;

pub struct OneshotReply<T> {
    tx: OneshotSender<T>,
}

impl<T> OneshotReply<T> {
    fn new() -> (OneshotReply<T>, OneshotReceiver<T>) {
        let (tx, rx) = oneshot_channel();
        (OneshotReply { tx }, rx)
    }
}

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
#[derive(Debug)]
pub struct RootChannel<S: StateTrait> {
    rx: UnboundedReceiver<SectionRequest<Uuid, S>>,
    tx: UnboundedSender<SectionRequest<Uuid, S>>,
    section_handles: BTreeMap<Uuid, UnboundedSender<Command>>,
}

impl<S: StateTrait> RootChannel<S> {
    // FIXME: should be trait method
    pub fn shutdown(&mut self) {
        let mut section_handles = BTreeMap::new();
        std::mem::swap(&mut section_handles, &mut self.section_handles);
        for (_id, handle) in section_handles {
            handle.send(Command::Stop).ok();
        }
    }
}

impl<S: StateTrait> RootChannelTrait for RootChannel<S> {
    type Id = Uuid;
    type Error = ChanError;
    type SectionChannel = SectionChannel<Self::Id, S>;

    fn new() -> Self {
        let (tx, rx) = unbounded_channel::<SectionRequest<Self::Id, S>>();
        Self {
            tx,
            rx,
            section_handles: BTreeMap::new(),
        }
    }

    fn add_section(&mut self, section_id: Self::Id) -> Result<Self::SectionChannel, Self::Error> {
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

    fn remove_section(&mut self, section_id: Self::Id) -> Result<(), Self::Error> {
        match self.section_handles.remove(&section_id) {
            Some(_) => Ok(()),
            None => Err(ChanError::NoSuchSection),
        }
    }

    async fn recv(&mut self) -> Result<SectionRequest<Self::Id, S>, Self::Error> {
        match self.rx.recv().await {
            Some(msg) => Ok(msg),
            None => Err(ChanError::Closed),
        }
    }

    async fn send(&mut self, section_id: Self::Id, command: Command) -> Result<(), Self::Error> {
        let section_handle = match self.section_handles.get(&section_id) {
            Some(section) => section,
            None => return Err(ChanError::NoSuchSection),
        };
        section_handle.send(command).map_err(|_| ChanError::Closed)
    }
}

// section channels
#[derive(Debug)]
pub struct SectionChannel<Id, S>
where
    Id: std::fmt::Debug + Copy + Send + Sync + 'static,
    S: StateTrait,
{
    id: Id,
    root_tx: UnboundedSender<SectionRequest<Id, S>>,
    rx: UnboundedReceiver<Command>,
    weak_tx: WeakUnboundedSender<Command>,
    _marker: PhantomData<S>,
}

impl<Id: std::fmt::Debug + Copy + Send + Sync + 'static, S: StateTrait> SectionChannel<Id, S> {
    fn new(
        id: Id,
        root_tx: UnboundedSender<SectionRequest<Id, S>>,
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

impl<Id, S> SectionChannelTrait for SectionChannel<Id, S>
    where S: StateTrait,
          Id: std::fmt::Debug + Copy + Send + Sync + 'static,
{
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

impl<Id, S> Drop for SectionChannel<Id, S>
    where 
        Id: std::fmt::Debug + Copy + Send + Sync + 'static,
        S: StateTrait,
{
    fn drop(&mut self) {
        self.root_tx
            .send(SectionRequest::Stopped { id: self.id })
            .ok();
    }
}

pub struct WeakSectionChannel {
    weak_tx: WeakUnboundedSender<Command>,
}

impl WeakSectionChannelTrait for WeakSectionChannel {
    async fn ack(self, payload: Box<dyn Any + Send + 'static>) {
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
        let section_chan = root_chan.add_section(Uuid::from_u128(0)).unwrap();
        let weak_chan = section_chan.weak_chan();
        test_send(root_chan);
        test_send(section_chan);
        test_send(weak_chan);
    }
}

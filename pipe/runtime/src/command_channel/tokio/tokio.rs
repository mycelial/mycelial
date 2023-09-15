//! Root channel implementation for tokio

use tokio::sync::mpsc::{
    channel,
    unbounded_channel,
    Receiver,
    Sender,
    UnboundedSender,
    UnboundedReceiver, 
    WeakUnboundedSender,
};
use tokio::sync::oneshot::{
    channel as oneshot_channel,
    Sender as OneshotSender,
    Receiver as OneshotReceiver,
};

use std::any::Any;
use std::collections::BTreeMap;
use std::marker::PhantomData;
use section::{
    State as StateTrait,
    RootChannel as RootChannelTrait,
    SectionChannel as SectionChannelTrait,
    Command,
    WeakSectionChannel as WeakSectionChannelTrait,
    SectionRequest as _SectionRequest,
    async_trait,
    ReplyTo
};

pub type SectionRequest<S=SectionState> = _SectionRequest<S, OneshotReply<Option<S>>, OneshotReply<()>>;

pub struct OneshotReply<T> {
    tx: OneshotSender<T>
}

impl<T> OneshotReply<T> {
    fn new() -> (OneshotReply<T>, OneshotReceiver<T>) {
        let (tx, rx) = oneshot_channel();
        (OneshotReply{ tx }, rx)
    }
}

#[async_trait]
impl<T: Send> ReplyTo for OneshotReply<T> {
    type With = T;
    type Error = ChanError;

    async fn reply(self, with: Self::With) -> Result<(), Self::Error> {
        self.tx
            .send(with)
            .map_err(|_| ChanError::Closed)
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

impl std::error::Error for ChanError{}

// Root channel
pub struct RootChannel {
    rx: Receiver<SectionRequest>,
    tx: Sender<SectionRequest>,
    section_handles: BTreeMap<u64, UnboundedSender<Command>>
}

impl RootChannel {
    pub fn new() -> Self {
        let (tx, rx) = channel::<SectionRequest>(1);
        Self {tx, rx, section_handles: BTreeMap::new()}
    }
}

#[async_trait]
impl RootChannelTrait for RootChannel {
    type Error = ChanError;
    type SectionChannel = SectionChannel;

    fn section_channel(
        &mut self,
        section_id: u64
    ) -> Result<Self::SectionChannel, Self::Error> {
        if self.section_handles.contains_key(&section_id) {
            return Err(ChanError::SectionExists);
        }
        let (section_tx, section_rx) = unbounded_channel();
        let weak_section_tx = section_tx.clone().downgrade();
        self.section_handles.insert(section_id, section_tx);
        Ok(SectionChannel::new(section_id, self.tx.clone(), section_rx, weak_section_tx))
    }

    async fn recv(&mut self) -> Result<SectionRequest, Self::Error> {
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
        section_handle
            .send(command)
            .map_err(|_| ChanError::Closed)
    }
}


// section state
#[derive(Debug, Clone)]
pub struct SectionState {

}

impl SectionState {
    pub fn new() -> Self {
        Self {}
    }
}

impl StateTrait for SectionState {
    fn get<T>(&self, _key: &str) -> Option<T> {
        None
    }

    fn set<T>(&mut self, _key: &str, _value: T) {
    }

    fn new() -> Self {
        SectionState::new()
    }
}

// section channels
pub struct SectionChannel<S=SectionState> where S: StateTrait { 
    id: u64,
    root_tx: Sender<SectionRequest<S>>,
    rx: UnboundedReceiver<Command>,
    weak_tx: WeakUnboundedSender<Command>,
    _marker: PhantomData<S>,
}

impl<S: StateTrait> SectionChannel<S>{
    fn new(
        id: u64,
        root_tx: Sender<SectionRequest<S>>,
        rx: UnboundedReceiver<Command>,
        weak_tx: WeakUnboundedSender<Command>,
    ) -> Self {
        Self { id, root_tx, rx, weak_tx, _marker: PhantomData }
    }
}


#[async_trait]
impl<S: StateTrait> SectionChannelTrait for SectionChannel<S> {
    type State = S;
    type Error = ChanError;
    type WeakRef = WeakSectionChannel;
    type Request = SectionRequest<S>;

    // request to runtime
    async fn retrieve_state(&mut self) -> Result<Option<Self::State>, Self::Error> {
        let (reply_to, rx) = OneshotReply::new();
        self.root_tx.send(Self::Request::RetrieveState{
            id: self.id,
            reply_to,
        })
            .await
            .map_err(|_| ChanError::Closed)?;
        rx
            .await
            .map_err(|_| ChanError::Closed)
    }

    // request to runtime
    async fn store_state(&mut self, state: Self::State) -> Result<(), Self::Error> {
        let (reply_to, rx) = OneshotReply::new();
        self.root_tx.send(SectionRequest::StoreState{id: self.id, state, reply_to})
            .await
            .map_err(|_| ChanError::Closed)?;
        rx
            .await
            .map_err(|_| ChanError::Closed)
    }

    // request to runtime
    async fn log<T: Into<String> + Send>(&mut self, log: T) -> Result<(), Self::Error> {
        self.root_tx.send(SectionRequest::Log{id: self.id, message: log.into()})
            .await
            .map_err(|_| ChanError::Closed)
    }

    // request from runtime or from own weak ref?
    async fn recv(&mut self) -> Result<Command, Self::Error> {
        match self.rx.recv().await {
            Some(cmd) => Ok(cmd),
            None => Err(ChanError::Closed)
        }
    }

    // weak reference to self which can send Command messages
    // used for message acks
    fn weak_chan(&self) -> Self::WeakRef {
        let weak_tx = self.weak_tx.clone();
        WeakSectionChannel{ weak_tx }
    }
}


pub struct WeakSectionChannel {
    weak_tx: WeakUnboundedSender<Command>
}

#[async_trait]
impl WeakSectionChannelTrait for WeakSectionChannel{
    async fn ack(self, payload: Box<dyn Any + Send + Sync + 'static>) {
        if let Some(tx) = self.weak_tx.upgrade() {
            tx.send(Command::Ack(payload)).ok();
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn send() {
        fn test_send<T: Send>(_: T) {}
        let mut root_chan = RootChannel::new();
        let section_chan = root_chan.section_channel(0).unwrap();
        let weak_chan = section_chan.weak_chan();
        test_send(root_chan);
        test_send(section_chan);
        test_send(weak_chan);
    }
}

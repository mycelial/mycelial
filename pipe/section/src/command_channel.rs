//! Command channel

use async_trait::async_trait;
use std::any::Any;

use crate::state::State;

#[async_trait]
pub trait RootChannel: Send + Sync + 'static {
    type SectionChannel: SectionChannel + Send;
    type Error: std::error::Error + Send + Sync + 'static;

    fn new() -> Self;

    // create and return new section channel
    fn add_section(&mut self, section_id: u64) -> Result<Self::SectionChannel, Self::Error>;

    // remove section
    fn remove_section(&mut self, section_id: u64) -> Result<(), Self::Error>;

    // receive request from section
    async fn recv(
        &mut self,
    ) -> Result<
        SectionRequest<
            <Self::SectionChannel as SectionChannel>::State,
            <Self::SectionChannel as SectionChannel>::ReplyRetrieveState,
            <Self::SectionChannel as SectionChannel>::ReplyStoreState,
        >,
        Self::Error,
    >;

    // send command to section by id
    async fn send(&mut self, section_id: u64, command: Command) -> Result<(), Self::Error>;
}

#[async_trait]
pub trait SectionChannel: Send + Sync + 'static {
    type Error: std::error::Error + Send + Sync + 'static;
    type State: State;
    type WeakChannel: WeakSectionChannel;
    type ReplyRetrieveState: ReplyTo<With = Option<Self::State>>;
    type ReplyStoreState: ReplyTo<With = ()>;

    // ask runtime to retrieve previosly stored state
    async fn retrieve_state(&mut self) -> Result<Option<Self::State>, Self::Error>;

    // ask runtime to store state
    async fn store_state(&mut self, state: Self::State) -> Result<(), Self::Error>;

    // ask runtime to log message
    async fn log<T: Into<String> + Send>(&mut self, log: T) -> Result<(), Self::Error>;

    // receive command from runtime
    async fn recv(&mut self) -> Result<Command, Self::Error>;

    // instantiate weak channel
    fn weak_chan(&self) -> Self::WeakChannel;
}

#[async_trait]
pub trait WeakSectionChannel: Send + Sync + 'static {
    async fn ack(self, ack: Box<dyn Any + Send + Sync + 'static>);
}

#[non_exhaustive]
#[derive(Debug)]
pub enum Command {
    // Message Acknowledgement
    Ack(Box<dyn Any + Send + Sync + 'static>),

    // Signal for section to stop
    Stop,
}

#[non_exhaustive]
pub enum SectionRequest<S: State, Rs: ReplyTo<With = Option<S>>, Ss: ReplyTo<With = ()>> {
    RetrieveState { id: u64, reply_to: Rs },
    StoreState { id: u64, state: S, reply_to: Ss },
    Log { id: u64, message: String },
    Stopped { id: u64 },
}

impl<S: State, Rs: ReplyTo<With = Option<S>>, Ss: ReplyTo<With = ()>> std::fmt::Debug
    for SectionRequest<S, Rs, Ss>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::RetrieveState { id, .. } => f
                .debug_struct("SectionRequest::RetrieveState")
                .field("id", id)
                .finish(),
            Self::StoreState { id, state, .. } => f
                .debug_struct("SectionRequest::StoreState")
                .field("id", id)
                .field("state", state)
                .finish(),
            Self::Log { id, message } => f
                .debug_struct("SectionRequest::Log")
                .field("id", id)
                .field("message", message)
                .finish(),
            Self::Stopped { id } => f
                .debug_struct("SectionRequest::Stopped")
                .field("id", id)
                .finish(),
            #[allow(unreachable_patterns)]
            _ => Err(std::fmt::Error),
        }
    }
}

#[async_trait]
pub trait ReplyTo: Send {
    type Error: std::error::Error + Send + Sync + 'static;
    type With;

    async fn reply(self, with: Self::With) -> Result<(), Self::Error>;
}

pub enum SectionRequestReplyError<E> {
    ReplyError(E),
    BadResponse(&'static str),
}

impl<S: State, Rs: ReplyTo<With = Option<S>>, Ss: ReplyTo<With = ()>> SectionRequest<S, Rs, Ss> {
    pub async fn reply_retrieve_state(
        self,
        state: Option<S>,
    ) -> Result<(), SectionRequestReplyError<<Rs as ReplyTo>::Error>> {
        match self {
            Self::RetrieveState { reply_to, .. } => reply_to
                .reply(state)
                .await
                .map_err(SectionRequestReplyError::ReplyError),
            _ => Err(SectionRequestReplyError::BadResponse(
                "expected to reply to state request",
            )),
        }
    }

    pub async fn reply_store_state(
        self,
    ) -> Result<(), SectionRequestReplyError<<Ss as ReplyTo>::Error>> {
        match self {
            Self::StoreState { reply_to, .. } => reply_to
                .reply(())
                .await
                .map_err(SectionRequestReplyError::ReplyError),
            _ => Err(SectionRequestReplyError::BadResponse(
                "expected to reply to state request",
            )),
        }
    }
}

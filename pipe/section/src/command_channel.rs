//! Command channel

use std::{any::Any, future::Future};

use crate::state::State;

pub trait RootChannel: Send + 'static {
    type Id: Copy + std::fmt::Debug + Send + Sync + 'static;
    type SectionChannel: SectionChannel + Send;
    type Error: std::error::Error + Send + Sync + 'static;

    fn new() -> Self;

    // create and return new section channel
    fn add_section(&mut self, section_id: Self::Id) -> Result<Self::SectionChannel, Self::Error>;

    // remove section
    fn remove_section(&mut self, section_id: Self::Id) -> Result<(), Self::Error>;

    // receive request from section
    fn recv(
        &mut self,
    ) -> impl Future<Output=Result<
            SectionRequest<
                Self::Id,
                <Self::SectionChannel as SectionChannel>::State,
                <Self::SectionChannel as SectionChannel>::ReplyRetrieveState,
                <Self::SectionChannel as SectionChannel>::ReplyStoreState,
            >,
            Self::Error,
        >> + Send;

    // send command to section by id
    fn send(&mut self, section_id: Self::Id, command: Command) -> impl Future<Output=Result<(), Self::Error>> + Send;
}

pub trait SectionChannel: Send + Sync + 'static {
    type Error: std::error::Error + Send + Sync + 'static;
    type State: State;
    type WeakChannel: WeakSectionChannel;
    type ReplyRetrieveState: ReplyTo<With = Option<Self::State>>;
    type ReplyStoreState: ReplyTo<With = ()>;

    // ask runtime to retrieve previosly stored state
    fn retrieve_state(&mut self) -> impl Future<Output=Result<Option<Self::State>, Self::Error>> + Send;

    // ask runtime to store state
    fn store_state(&mut self, state: Self::State) -> impl Future<Output=Result<(), Self::Error>> + Send;

    // receive command from runtime
    fn recv(&mut self) -> impl Future<Output=Result<Command, Self::Error>> + Send;

    // instantiate weak channel
    fn weak_chan(&self) -> Self::WeakChannel;
}

pub trait WeakSectionChannel: Send + Sync + 'static {
    fn ack(self, ack: Box<dyn Any + Send + 'static>) -> impl Future<Output=()> + Send;
}

#[non_exhaustive]
#[derive(Debug)]
pub enum Command {
    // Message Acknowledgement
    Ack(Box<dyn Any + Send + 'static>),

    // Signal for section to stop
    Stop,
}

#[non_exhaustive]
pub enum SectionRequest<Id: Copy + std::fmt::Debug + Send, S: State, Rs: ReplyTo<With = Option<S>>, Ss: ReplyTo<With = ()>> {
    RetrieveState { id: Id, reply_to: Rs },
    StoreState { id: Id, state: S, reply_to: Ss },
    Stopped { id: Id },
}

impl<Id: Copy + std::fmt::Debug + Send, S: State, Rs: ReplyTo<With = Option<S>>, Ss: ReplyTo<With = ()>> std::fmt::Debug
    for SectionRequest<Id, S, Rs, Ss>
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
            Self::Stopped { id } => f
                .debug_struct("SectionRequest::Stopped")
                .field("id", id)
                .finish(),
            #[allow(unreachable_patterns)]
            _ => Err(std::fmt::Error),
        }
    }
}

pub trait ReplyTo: Send {
    type Error: std::error::Error + Send + Sync + 'static;
    type With;

    fn reply(self, with: Self::With) -> impl Future<Output=Result<(), Self::Error>> + Send;
}

pub enum SectionRequestReplyError<E> {
    ReplyError(E),
    BadResponse(&'static str),
}

impl<T: Copy + std::fmt::Debug + Send, S: State, Rs: ReplyTo<With = Option<S>>, Ss: ReplyTo<With = ()>> SectionRequest<T, S, Rs, Ss> {
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

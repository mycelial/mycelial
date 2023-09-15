//! Command channel

use std::any::Any;
use async_trait::async_trait;

use crate::State;

#[async_trait]
pub trait RootChannel {
    type SectionChannel: SectionChannel;
    type Error;

    // create new channel for section
    fn section_channel(&mut self, section_id: u64) -> Result<Self::SectionChannel, Self::Error>;

    // receive request from section
    async fn recv(&mut self) -> Result<<Self::SectionChannel as SectionChannel>::Request, Self::Error>;

    // send command to section by id
    async fn send(&mut self, section_id: u64, command: Command) -> Result<(), Self::Error>;
}

#[async_trait]
pub trait SectionChannel {
    type Error;
    type State: State;
    type WeakRef: WeakSectionChannel;
    type Request;

    // ask runtime to retrieve previosly stored state
    async fn retrieve_state(&mut self) -> Result<Option<Self::State>, Self::Error>;

    // ask runtime to store state
    async fn store_state(&mut self, state: Self::State) -> Result<(), Self::Error>;

    // ask runtime to log message
    async fn log<T: Into<String> + Send>(&mut self, log: T) -> Result<(), Self::Error>;

    // receive command from runtime
    async fn recv(&mut self) -> Result<Command, Self::Error>;

    // instantiate weak channel
    fn weak_chan(&self) -> Self::WeakRef;
}

#[async_trait]
pub trait WeakSectionChannel: Send + 'static {
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
pub enum SectionRequest<S: State, Rs: ReplyTo<With=Option<S>>, Ss: ReplyTo<With=()>> {
    RetrieveState{id: u64, reply_to: Rs},
    StoreState{id: u64, state: S, reply_to: Ss},
    Log{id: u64, message: String},
}

impl<S: State, Rs: ReplyTo<With=Option<S>>, Ss: ReplyTo<With=()>> std::fmt::Debug for SectionRequest<S, Rs, Ss> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::RetrieveState { id, .. } => {
                f.debug_struct("SectionRequest::RetrieveState")
                    .field("id", id)
                    .finish()
            },
            Self::StoreState { id, state, .. } => {
                f.debug_struct("SectionRequest::StoreState")
                    .field("id", id)
                    .field("state", state)
                    .finish()
            },
            Self::Log { id, message } => {
                f.debug_struct("SectionRequest::Log")
                    .field("id", id)
                    .field("message", message)
                    .finish()
            },
            #[allow(unreachable_patterns)]
            _ => {
                Err(std::fmt::Error)
            }
        }
    }
}

#[async_trait]
pub trait ReplyTo {
    type Error;
    type With;

    async fn reply(self, with: Self::With) -> Result<(), Self::Error>;
}

pub enum SectionRequestReplyError<E> {
    ReplyError(E),
    BadResponse(&'static str)
}

impl<S: State + Send, Rs: ReplyTo<With=Option<S>>, Ss: ReplyTo<With=()>> SectionRequest<S, Rs, Ss> { 
    pub async fn reply_retrieve_state(
        self,
        state: Option<S>
    ) -> Result<(), SectionRequestReplyError<<Rs as ReplyTo>::Error>> {
        match self {
            Self::RetrieveState { reply_to, .. } => {
                reply_to
                    .reply(state)
                    .await
                    .map_err(|e| SectionRequestReplyError::ReplyError(e))
            },
            _ => Err(SectionRequestReplyError::BadResponse("expected to reply to state request")),
        }
    }

    pub async fn reply_store_state(self) -> Result<(), SectionRequestReplyError<<Ss as ReplyTo>::Error>> {
        match self {
            Self::StoreState { reply_to, .. } => {
                reply_to
                    .reply(())
                    .await
                    .map_err(|e| SectionRequestReplyError::ReplyError(e))
            },
            _ => Err(SectionRequestReplyError::BadResponse("expected to reply to state request")),
        }
    }
}

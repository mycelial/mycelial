use crate::{
    command_channel::{
        Command, ReplyTo, RootChannel, SectionChannel, SectionRequest as _SectionRequest,
        WeakSectionChannel,
    },
};
use std::{
    any::Any,
    future::{pending, ready},
    marker::PhantomData,
};

use super::DummyState;

#[derive(Debug)]
pub struct DummyError {}

impl std::fmt::Display for DummyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl std::error::Error for DummyError {}

#[derive(Debug)]
pub struct DummyRootChannel {}

impl Default for DummyRootChannel {
    fn default() -> Self {
        Self::new()
    }
}

impl DummyRootChannel {
    pub fn new() -> Self {
        Self {}
    }
}

pub type SectionRequest<Id> = _SectionRequest<Id, DummyState, RepTo<Option<DummyState>>, RepTo<()>>;

impl RootChannel for DummyRootChannel {
    type Id = ();
    type Error = DummyError;
    type SectionChannel = DummySectionChannel;

    fn new() -> Self {
        Self {}
    }

    async fn recv(&mut self) -> Result<SectionRequest<Self::Id>, Self::Error> {
        pending::<Result<SectionRequest<Self::Id>, Self::Error>>().await
    }

    async fn send(&mut self, _: Self::Id, _command: Command) -> Result<(), Self::Error> {
        ready(Ok(())).await
    }

    fn add_section(&mut self, _section_id: Self::Id) -> Result<Self::SectionChannel, Self::Error> {
        Ok(DummySectionChannel {})
    }

    fn remove_section(&mut self, _section_id: Self::Id) -> Result<(), Self::Error> {
        Ok(())
    }
}

#[derive(Debug)]
pub struct DummySectionChannel {}

impl Default for DummySectionChannel {
    fn default() -> Self {
        Self::new()
    }
}

impl DummySectionChannel {
    pub fn new() -> Self {
        Self {}
    }
}

pub struct RepTo<T> {
    _marker: PhantomData<T>,
}

impl<T: Send> ReplyTo for RepTo<T> {
    type Error = DummyError;
    type With = T;

    async fn reply(self, _with: Self::With) -> Result<(), Self::Error> {
        Ok(())
    }
}

impl SectionChannel for DummySectionChannel {
    type Error = DummyError;
    type State = DummyState;
    type WeakChannel = DummyWeakChannel;
    type ReplyRetrieveState = RepTo<Option<Self::State>>;
    type ReplyStoreState = RepTo<()>;

    async fn retrieve_state(&mut self) -> Result<Option<Self::State>, Self::Error> {
        ready(Ok(None)).await
    }

    async fn store_state(&mut self, _state: Self::State) -> Result<(), Self::Error> {
        ready(Ok(())).await
    }

    async fn recv(&mut self) -> Result<Command, Self::Error> {
        pending::<Result<Command, Self::Error>>().await
    }

    fn weak_chan(&self) -> Self::WeakChannel {
        Self::WeakChannel {}
    }
}

#[derive(Debug)]
pub struct DummyWeakChannel {}

impl WeakSectionChannel for DummyWeakChannel {
    async fn ack(self, _ack: Box<dyn Any + Send + 'static>) {
        ready(()).await
    }
}

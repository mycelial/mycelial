use crate::{async_trait, Command, RootChannel, SectionChannel, State, WeakSectionChannel};
use std::{
    any::Any,
    convert::Infallible,
    future::{pending, ready},
};

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

impl DummyRootChannel {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl RootChannel for DummyRootChannel {
    type Error = DummyError;
    type SectionChannel = DummySectionChannel;

    async fn recv(&mut self) -> Result<(), Self::Error> {
        pending::<Result<(), Self::Error>>().await
    }

    async fn send(&mut self, _section_id: u64, _command: Command) -> Result<(), Self::Error> {
        ready(Ok(())).await
    }

    fn section_channel(&mut self, _section_id: u64) -> Result<Self::SectionChannel, Self::Error> {
        Ok(DummySectionChannel {})
    }
}

#[derive(Debug)]
pub struct DummySectionChannel {}

impl DummySectionChannel {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl SectionChannel for DummySectionChannel {
    type Error = DummyError;
    type State = DummyState;
    type WeakChannel = DummyWeakChannel;
    type Request = ();

    async fn retrieve_state(&mut self) -> Result<Option<Self::State>, Self::Error> {
        ready(Ok(None)).await
    }

    async fn store_state(&mut self, _state: Self::State) -> Result<(), Self::Error> {
        ready(Ok(())).await
    }

    async fn log<T: Into<String> + Send>(&mut self, _log: T) -> Result<(), Self::Error> {
        ready(Ok(())).await
    }

    async fn recv(&mut self) -> Result<Command, Self::Error> {
        pending::<Result<Command, Self::Error>>().await
    }

    fn weak_chan(&self) -> Self::WeakChannel {
        Self::WeakChannel {}
    }
}

#[derive(Debug, Clone)]
pub struct DummyState {}

impl State for DummyState {
    type Error = Infallible;

    fn new() -> Self {
        Self {}
    }

    fn get<T>(&self, _key: &str) -> Result<Option<T>, Self::Error> {
        Ok(None)
    }

    fn set<T>(&mut self, _key: &str, _value: T) -> Result<(), Self::Error> {
        Ok(())
    }
}

#[derive(Debug)]
pub struct DummyWeakChannel {}

#[async_trait]
impl WeakSectionChannel for DummyWeakChannel {
    async fn ack(self, _ack: Box<dyn Any + Send + Sync + 'static>) {
        ready(()).await
    }
}

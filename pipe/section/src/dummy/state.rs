use std::convert::Infallible;

use crate::state::State;

#[derive(Debug, Clone)]
pub struct DummyState {}

impl Default for DummyState {
    fn default() -> Self {
        Self::new()
    }
}

impl DummyState {
    pub fn new() -> Self {
        Self {}
    }
}

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

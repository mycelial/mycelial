use std::future::Future;
use std::pin::Pin;

use section::State;

use crate::types::SectionError;

pub trait Storage<S: State + Send + 'static>: Send + Sync + 'static {
    fn store_state(
        &self,
        id: u64,
        state: S,
    ) -> Pin<Box<dyn Future<Output = Result<(), SectionError>> + Send + 'static>>;

    fn retrieve_state(
        &self,
        id: u64,
    ) -> Pin<Box<dyn Future<Output = Result<Option<S>, SectionError>> + Send + 'static>>;
}

use section::State;
use std::future::Future;
use std::pin::Pin;

use crate::types::SectionError;

pub trait Storage<S: State> {
    fn store_state(
        &self,
        id: u64,
        section_id: u64,
        section_name: String,
        state: S,
    ) -> Pin<Box<dyn Future<Output = Result<(), SectionError>> + Send + 'static>>;

    fn retrieve_state(
        &self,
        id: u64,
        section_id: u64,
        section_name: String,
    ) -> Pin<Box<dyn Future<Output = Result<Option<S>, SectionError>> + Send + 'static>>;
}

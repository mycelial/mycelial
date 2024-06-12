pub mod command_channel;
pub mod dummy;
pub mod message;
pub mod pretty_print;
pub mod section;
pub mod state;

use std::pin::Pin;

// re-export
pub use async_trait::async_trait;
pub use futures;
pub use rust_decimal as decimal;
pub use uuid;

pub type SectionError = Box<dyn std::error::Error + Send + Sync + 'static>;
pub type SectionFuture =
    Pin<Box<dyn std::future::Future<Output = Result<(), SectionError>> + Send + 'static>>;
pub type SectionMessage = Box<dyn crate::message::Message>;


pub mod prelude {
    pub use crate::{
        SectionError,
        SectionFuture,
        SectionMessage,
        section::Section,
        message::{Message, Chunk, Column, Value, ValueView, DataFrame, DataType},
        command_channel::{SectionChannel, WeakSectionChannel, RootChannel, Command},
        futures::{self, Future, Sink, Stream, SinkExt, StreamExt, FutureExt},
        uuid,
        async_trait,
        decimal,
    };
}
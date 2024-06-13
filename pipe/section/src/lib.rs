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
        async_trait,
        command_channel::{Command, RootChannel, SectionChannel, WeakSectionChannel},
        decimal,
        futures::{self, Future, FutureExt, Sink, SinkExt, Stream, StreamExt},
        message::{Chunk, Column, DataFrame, DataType, Message, Value, ValueView, Ack, Next},
        section::Section,
        state::State,
        uuid, SectionError, SectionFuture, SectionMessage,
    };
}

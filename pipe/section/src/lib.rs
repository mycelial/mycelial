pub mod command_channel;
pub mod dummy;
pub mod message;
pub mod pretty_print;
pub mod section;
pub mod state;

use std::pin::Pin;

// re-export
pub use futures;
pub use rust_decimal as decimal;
pub use uuid;

pub type SectionError = Box<dyn std::error::Error + Send + Sync + 'static>;
pub type SectionFuture =
    Pin<Box<dyn std::future::Future<Output = Result<(), SectionError>> + Send + 'static>>;
pub type SectionMessage = Box<dyn crate::message::Message>;
pub trait SectionStream: futures::Stream<Item = SectionMessage> + Send + 'static {}
impl<T> SectionStream for T where T: futures::Stream<Item = SectionMessage> + Send + 'static {}
pub trait SectionSink:
    futures::Sink<SectionMessage, Error = SectionError> + Send + 'static
{
}
impl<T> SectionSink for T where
    T: futures::Sink<SectionMessage, Error = SectionError> + Send + 'static
{
}

pub type DynStream = Pin<Box<dyn SectionStream>>;
pub type DynSink = Pin<Box<dyn SectionSink>>;
pub trait DynSection<SectionChan: command_channel::SectionChannel>:
    section::Section<DynStream, DynSink, SectionChan, Future = SectionFuture, Error = SectionError>
    + Send
{
    fn dyn_start(
        self: Box<Self>,
        input: DynStream,
        output: DynSink,
        section_chan: SectionChan,
    ) -> <Self as section::Section<DynStream, DynSink, SectionChan>>::Future;
}

impl<T, SectionChan> DynSection<SectionChan> for T
where
    T: section::Section<
            DynStream,
            DynSink,
            SectionChan,
            Future = SectionFuture,
            Error = SectionError,
        > + Send
        + 'static,
    SectionChan: command_channel::SectionChannel,
{
    fn dyn_start(
        self: Box<Self>,
        input: DynStream,
        output: DynSink,
        section_chan: SectionChan,
    ) -> <Self as section::Section<DynStream, DynSink, SectionChan>>::Future {
        self.start(input, output, section_chan)
    }
}

pub mod prelude {
    pub use crate::{
        command_channel::{Command, RootChannel, SectionChannel, WeakSectionChannel},
        decimal,
        futures::{self, Future, FutureExt, Sink, SinkExt, Stream, StreamExt},
        message::{Ack, Chunk, Column, DataFrame, DataType, Message, Next, Value, ValueView},
        section::Section,
        state::State,
        uuid, DynSection, DynSink, DynStream, SectionError, SectionFuture, SectionMessage,
        SectionSink, SectionStream,
    };
}

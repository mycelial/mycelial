use crate::message::Message;
use futures::{Future, Sink, Stream};
use section::Section;
use section::SectionChannel;
use std::pin::Pin;

pub type SectionError = Box<dyn std::error::Error + Send + Sync + 'static>;

pub type DynStream = Pin<Box<dyn Stream<Item = Message> + Send + 'static>>;
pub type DynSink = Pin<Box<dyn Sink<Message, Error = SectionError> + Send + 'static>>;
pub type SectionFuture = Pin<Box<dyn Future<Output = Result<(), SectionError>> + Send + 'static>>;

pub trait DynSection<SectionChan: SectionChannel + Send + 'static>:
    Section<DynStream, DynSink, SectionChan, Future = SectionFuture, Error = SectionError>
{
    fn dyn_start(
        self: Box<Self>,
        input: DynStream,
        output: DynSink,
        section_chan: SectionChan,
    ) -> <Self as Section<DynStream, DynSink, SectionChan>>::Future;
}

impl<T, SectionChan> DynSection<SectionChan> for T
where
    T: Section<DynStream, DynSink, SectionChan, Future = SectionFuture, Error = SectionError>
        + Send
        + 'static,
    SectionChan: SectionChannel + Send + 'static,
{
    fn dyn_start(
        self: Box<Self>,
        input: DynStream,
        output: DynSink,
        section_chan: SectionChan,
    ) -> <Self as Section<DynStream, DynSink, SectionChan>>::Future {
        self.start(input, output, section_chan)
    }
}

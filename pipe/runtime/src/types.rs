use section::{
    section::Section,
    command_channel::SectionChannel,
    SectionMessage,
    SectionError,
    futures::{Future, Sink, Stream},
};
use std::pin::Pin;

pub type DynStream = Pin<Box<dyn Stream<Item = SectionMessage> + Send + 'static>>;
pub type DynSink = Pin<Box<dyn Sink<SectionMessage, Error = SectionError> + Send + 'static>>;
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

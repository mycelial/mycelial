use futures::{Stream, Sink, Future};
use std::pin::Pin;
use crate::{message::Message, command_channel::SectionChannel};
use section::{Section, State};

pub type SectionError = Box<dyn std::error::Error + Send + Sync + 'static>;

pub type DynStream = Pin<Box<dyn Stream<Item=Message> + Send + 'static>>;
pub type DynSink = Pin<Box<dyn Sink<Message, Error=SectionError> + Send + 'static>>;
pub type SectionFuture = Pin<Box<dyn Future<Output=Result<(), SectionError>> + Send + 'static>>;

pub trait DynSection<S: State>: Section<DynStream, DynSink, SectionChannel<S>, Future=SectionFuture, Error=SectionError> {
    fn dyn_start(
        self: Box<Self>,
        input: DynStream,
        output: DynSink,
        command: SectionChannel<S>,
    ) -> <Self as Section<DynStream, DynSink, SectionChannel<S>>>::Future;
}

impl<T, S: State> DynSection<S> for T 
    where T: Section<
                DynStream,
                DynSink,
                SectionChannel<S>,
                Future=SectionFuture, Error=SectionError
          > + Send + 'static,
{
    fn dyn_start(
        self: Box<Self>,
        input: DynStream,
        output: DynSink,
        command: SectionChannel<S>,
    ) -> <Self as Section<DynStream, DynSink, SectionChannel<S>>>::Future {
        self.start(input, output, command)
    }
}

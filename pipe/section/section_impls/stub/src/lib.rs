use section::{
    command_channel::{Command, SectionChannel},
    futures::{self, Sink, Stream, StreamExt},
    section::Section,
    SectionError, SectionMessage,
};
use std::future::Future;
use std::marker::PhantomData;
use std::task::{Context, Poll};
use std::{
    convert::Infallible,
    pin::{pin, Pin},
};

#[derive(Debug)]
pub struct Stub<T, E = Infallible> {
    _marker: PhantomData<(T, E)>,
}
impl<T, E> Default for Stub<T, E> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T, E> Stub<T, E> {
    pub fn new() -> Self {
        Self {
            _marker: PhantomData,
        }
    }
}

async fn consume_input<Input>(input: Input) -> Result<(), SectionError>
where
    Input: Stream<Item = SectionMessage> + Send + 'static,
{
    let mut input = pin!(input);
    while let Some(mut msg) = input.next().await {
        while msg.next().await?.is_some() {}
    }
    Ok(())
}

async fn wait_stop_command<SectionChan>(mut section_chan: SectionChan) -> Result<(), SectionError>
where
    SectionChan: SectionChannel + Send + 'static,
{
    while let Ok(cmd) = section_chan.recv().await {
        if let Command::Stop = cmd {
            break;
        }
    }
    Ok(())
}

impl<T, Input, Output, SectionChan> Section<Input, Output, SectionChan> for Stub<T, SectionError>
where
    Input: Stream<Item = SectionMessage> + Send + 'static,
    Output: Send + 'static,
    SectionChan: SectionChannel + Send + 'static,
{
    type Error = SectionError;
    type Future = Pin<Box<dyn Future<Output = Result<(), Self::Error>> + Send + 'static>>;

    fn start(self, input: Input, output: Output, section_chan: SectionChan) -> Self::Future {
        Box::pin(async move {
            let _output = output;
            let _res = futures::join!(consume_input(input), wait_stop_command(section_chan),);
            Ok(())
        })
    }
}

impl<T, E> Stream for Stub<T, E> {
    type Item = T;

    fn poll_next(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        Poll::Pending
    }
}

impl<T, E> Sink<T> for Stub<T, E> {
    type Error = E;

    fn poll_ready(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn start_send(self: Pin<&mut Self>, _item: T) -> Result<(), Self::Error> {
        Ok(())
    }

    fn poll_close(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }
}

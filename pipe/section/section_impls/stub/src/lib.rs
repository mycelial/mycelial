use section::{
    command_channel::{Command, SectionChannel},
    futures::{self, FutureExt, Sink, Stream, StreamExt},
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

impl<T, Input, Output, SectionChan> Section<Input, Output, SectionChan> for Stub<T, SectionError>
where
    Input: Stream<Item = SectionMessage> + Send + 'static,
    Output: Send + 'static,
    SectionChan: SectionChannel + Send + 'static,
{
    type Error = SectionError;
    type Future = Pin<Box<dyn Future<Output = Result<(), Self::Error>> + Send + 'static>>;

    fn start(self, input: Input, output: Output, mut section_channel: SectionChan) -> Self::Future {
        Box::pin(async move {
            let mut input = pin!(input);
            let _output = output;
            loop {
                futures::select! {
                    cmd = section_channel.recv().fuse() => {
                        if let Command::Stop = cmd? {
                            return Ok(())
                        }
                    },
                    msg = input.next().fuse() => {
                        let mut msg = match msg {
                            None => return Ok(()),
                            Some(msg) => msg,
                        };
                        while msg.next().await?.is_some() {}
                    }
                }
            }
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

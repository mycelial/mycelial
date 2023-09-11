use std::future::Future;
use std::marker::PhantomData;
use std::task::{Context, Poll};
use std::{convert::Infallible, future::pending, pin::Pin};

use futures::{Sink, Stream};

use section::Section;

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

impl<T, E, Input, Output, CommandChannel> Section<Input, Output, CommandChannel> for Stub<T, E>
    where Input: Send + 'static,
          Output: Send + 'static,
          CommandChannel: Send + 'static,
{
    type Error = E;
    type Future = Pin<Box<dyn Future<Output = Result<(), Self::Error>> + Send + 'static>>;

    fn start(self, input: Input, output: Output, command: CommandChannel) -> Self::Future {
        Box::pin(async move {
            let _input = input;
            let _output = output;
            let _command = command;
            Ok(pending::<()>().await)
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

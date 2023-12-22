//! Wrappings around tokio channel which allow Sender to behave as Sink and Receiver as a Stream
//!
//! Channels allow to glue pipe sections together (both static and dynamic)
use section::futures::Sink;
use std::{
    pin::Pin,
    task::{Context, Poll},
};
use tokio::sync::mpsc::{
    channel as _channel, error::SendError, unbounded_channel as _unbounded_channel, Receiver,
    Sender, UnboundedSender,
};
use tokio_stream::wrappers::{ReceiverStream, UnboundedReceiverStream};
use tokio_util::sync::PollSender;

#[allow(dead_code)] // fixme
pub fn channel<T>(buf_size: usize) -> (PollSender<T>, ReceiverStream<T>)
where
    T: Send + 'static,
{
    let (tx, rx): (Sender<T>, Receiver<T>) = _channel(buf_size);
    (PollSender::new(tx), ReceiverStream::new(rx))
}

#[allow(dead_code)] // fixme
pub fn unbounded_channel<T>() -> (PollUnboundedSender<T>, UnboundedReceiverStream<T>)
where
    T: Send + 'static,
{
    let (tx, rx) = _unbounded_channel();
    (
        PollUnboundedSender::new(tx),
        UnboundedReceiverStream::new(rx),
    )
}

pub struct PollUnboundedSender<T> {
    inner: UnboundedSender<T>,
}

impl<T> PollUnboundedSender<T> {
    pub fn new(inner: UnboundedSender<T>) -> Self {
        Self { inner }
    }
}

impl<T> Sink<T> for PollUnboundedSender<T> {
    type Error = SendError<T>;

    fn start_send(self: Pin<&mut Self>, item: T) -> Result<(), Self::Error> {
        self.get_mut().inner.send(item)
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn poll_ready(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn poll_close(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }
}

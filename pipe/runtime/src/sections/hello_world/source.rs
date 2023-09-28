//! HelloWorld Source example section implementation
//!
//! Since it is a source, this section ignores the input stream, and generates a "Hello World"
//! message every 5 seconds to the output stream.
use super::HelloWorldPayload;
use crate::{
    config::Map,
    message::{Message, RecordBatch},
    types::{DynSection, SectionError, SectionFuture},
};
use futures::stream::FusedStream;
use futures::{Sink, SinkExt, Stream, StreamExt};
use section::{Section, SectionChannel, State};
use tokio::time::{Instant, Interval};

use std::pin::{pin, Pin};
use std::time::Duration;

#[derive(Debug)]
pub struct HelloWorld {}

struct IntervalStream {
    interval: Interval,
}

impl IntervalStream {
    /// Create a new `IntervalStream`.
    pub fn new(delay: Duration) -> Self {
        Self {
            interval: tokio::time::interval(delay),
        }
    }
}

impl FusedStream for IntervalStream {
    fn is_terminated(&self) -> bool {
        false
    }
}

impl Stream for IntervalStream {
    type Item = Instant;

    fn poll_next(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        self.interval.poll_tick(cx).map(Some)
    }
}

impl HelloWorld {
    pub fn new() -> Self {
        Self {}
    }

    pub async fn enter_loop<Input, Output, SectionChan>(
        self,
        _input: Input,
        output: Output,
        mut section_chan: SectionChan,
    ) -> Result<(), SectionError>
    where
        Input: Stream<Item = Message> + Send + 'static,
        Output: Sink<Message, Error = SectionError> + Send + 'static,
        SectionChan: SectionChannel + Send + 'static,
    {
        let mut output = pin!(output);
        let mut interval = pin!(IntervalStream::new(Duration::from_secs(5)));
        loop {
            futures::select! {
                _ = interval.next() => {
                    match self.get_message().await {
                        Ok(msg) => {
                            output.send(msg).await.ok();
                        },
                        Err(e) => section_chan.log(format!("failed to retrieve next batch: {:?}", e)).await?,
                    }
                },
            }
        }
    }

    async fn get_message(&self) -> Result<Message, SectionError> {
        let hello_world_payload: HelloWorldPayload = HelloWorldPayload {
            message: "Hello, World!".to_string(),
        };
        let batch: RecordBatch = hello_world_payload.try_into()?;
        let message = Message::new("hello world", batch, None);
        Ok(message)
    }
}

impl<Input, Output, SectionChan> Section<Input, Output, SectionChan> for HelloWorld
where
    Input: Stream<Item = Message> + Send + 'static,
    Output: Sink<Message, Error = SectionError> + Send + 'static,
    SectionChan: SectionChannel + Send + 'static,
{
    // FIXME: define proper error
    type Error = SectionError;
    type Future = SectionFuture;

    fn start(self, input: Input, output: Output, section_chan: SectionChan) -> Self::Future {
        Box::pin(async move { self.enter_loop(input, output, section_chan).await })
    }
}

pub fn constructor<S: State>(_config: &Map) -> Result<Box<dyn DynSection<S>>, SectionError> {
    Ok(Box::new(HelloWorld::new()))
}

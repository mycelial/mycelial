//! HelloWorld Source example section implementation
//!
//! Generates a "Hello World" message every 5 seconds.
//! Since it is a source, this section ignores the input stream, and writes its message to the output stream.
use super::HelloWorldPayload;
use crate::{
    config::Map,
    message::{Message, RecordBatch},
    types::{DynSection, SectionError, SectionFuture},
};
use futures::{FutureExt, Sink, SinkExt, Stream};
use section::{Command, Section, SectionChannel};
use tokio::time;

use std::pin::pin;
use std::time::Duration;

#[derive(Debug)]
pub struct HelloWorld {
    message: String,
    interval_milis: i64,
}

impl Default for HelloWorld {
    fn default() -> Self {
        Self::new("", 5000)
    }
}

impl HelloWorld {
    pub fn new(message: impl Into<String>, interval_milis: i64) -> Self {
        Self {
            message: message.into(),
            interval_milis,
        }
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
        let mut interval = pin!(time::interval(Duration::from_millis(
            self.interval_milis as u64
        )));
        let mut counter = 0;
        loop {
            futures::select_biased! {
                cmd = section_chan.recv().fuse() => {
                    if let Command::Stop = cmd? {
                        return Ok(())
                    }
                }
                _ = interval.tick().fuse() => {
                    counter += 1;

                    let msg_string = format!("{} {}", self.message, counter);
                    let hello_world_payload: HelloWorldPayload = HelloWorldPayload {
                        message: msg_string,
                    };
                    let batch: RecordBatch = hello_world_payload.try_into()?;
                    let message = Message::new("hello world", batch, None);
                    section_chan.log(&format!("sending message: '{:?}'", message)).await?;
                    output.send(message).await.ok();
                }
            }
        }
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

pub fn constructor<S: SectionChannel>(
    config: &Map,
) -> Result<Box<dyn DynSection<S>>, SectionError> {
    let message = config
        .get("message")
        .ok_or("hello world section requires 'message'")?
        .as_str()
        .ok_or("'message' should be a string")?;
    let interval_milis = config
        .get("interval_milis")
        .ok_or("hello world section requires 'interval_milis'")?
        .as_int()
        .ok_or("'interval_milis' should be an int")?;
    Ok(Box::new(HelloWorld::new(message, interval_milis)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::StreamExt;
    use section::dummy::DummySectionChannel;
    use stub::Stub;
    use tokio::sync::mpsc::{Receiver, Sender};
    use tokio_stream::wrappers::ReceiverStream;
    use tokio_util::sync::PollSender;

    type StdError = Box<dyn std::error::Error + Send + Sync + 'static>;

    pub fn channel<T>(buf_size: usize) -> (PollSender<T>, ReceiverStream<T>)
    where
        T: Send + 'static,
    {
        let (tx, rx): (Sender<T>, Receiver<T>) = tokio::sync::mpsc::channel(buf_size);
        (PollSender::new(tx), ReceiverStream::new(rx))
    }

    #[tokio::test]
    async fn test_source() -> Result<(), StdError> {
        let hello_world_source = HelloWorld::new("Hello, World!", 5000);

        let input = Stub::<Message, StdError>::new();

        let (output, mut rx) = channel(1);
        let output = output.sink_map_err(|_| "chan closed".into());

        let section_chan = DummySectionChannel::new();

        let hello_world_source_section = hello_world_source.start(input, output, section_chan);
        let handle = tokio::spawn(hello_world_source_section);

        let out = rx.next().await.unwrap();
        assert_eq!(out.origin, "hello world");

        let expected_record_batch: RecordBatch = HelloWorldPayload {
            message: "Hello, World!".to_string(),
        }
        .try_into()?;

        assert_eq!(out.payload, expected_record_batch);

        handle.abort();
        Ok(())
    }
}

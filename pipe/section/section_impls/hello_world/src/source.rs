//! HelloWorld Source example section implementation
//!
//! Generates a "Hello World" message every 5 seconds.
//! Since it is a source, this section ignores the input stream, and writes its message to the output stream.
use super::HelloWorldPayload;
use section::{
    command_channel::{Command, SectionChannel},
    futures,
    futures::{FutureExt, Sink, SinkExt, Stream},
    section::Section,
    SectionError, SectionFuture, SectionMessage,
};
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
}

impl<Input, Output, SectionChan> Section<Input, Output, SectionChan> for HelloWorld
where
    Input: Stream<Item = SectionMessage> + Send + 'static,
    Output: Sink<SectionMessage, Error = SectionError> + Send + 'static,
    SectionChan: SectionChannel + Send + 'static,
{
    // FIXME: define proper error
    type Error = SectionError;
    type Future = SectionFuture;

    fn start(
        mut self,
        _input: Input,
        output: Output,
        mut section_chan: SectionChan,
    ) -> Self::Future {
        Box::pin(async move {
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
                        let message = format!("{} {}", self.message, counter);
                        let hello_world_payload = HelloWorldPayload { message };
                        section_chan.log(&format!("sending message: '{:?}'", hello_world_payload)).await?;
                        output.send(hello_world_payload.into()).await.ok();
                    }
                }
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use section::dummy::DummySectionChannel;
    use section::message::ValueView;
    use section::{futures::StreamExt, message::Chunk};
    use stub::Stub;
    use tokio::sync::mpsc::{Receiver, Sender};
    use tokio_stream::wrappers::ReceiverStream;
    use tokio_util::sync::PollSender;

    type SectionError = Box<dyn std::error::Error + Send + Sync + 'static>;

    pub fn channel<T>(buf_size: usize) -> (PollSender<T>, ReceiverStream<T>)
    where
        T: Send + 'static,
    {
        let (tx, rx): (Sender<T>, Receiver<T>) = tokio::sync::mpsc::channel(buf_size);
        (PollSender::new(tx), ReceiverStream::new(rx))
    }

    #[tokio::test]
    async fn test_source() -> Result<(), SectionError> {
        let hello_world_source = HelloWorld::new("Hello, World!", 5000);

        let input = Stub::<SectionMessage, SectionError>::new();

        let (output, mut rx) = channel(1);
        let output = output.sink_map_err(|_| "chan closed".into());

        let section_chan = DummySectionChannel::new();

        let hello_world_source_section = hello_world_source.start(input, output, section_chan);
        let handle = tokio::spawn(hello_world_source_section);

        let mut out = rx.next().await.unwrap();
        assert_eq!(out.origin(), "hello world");

        let msg = out.next().await;
        assert!(msg.is_ok());

        let msg = msg.unwrap();
        assert!(msg.is_some());

        let msg = msg.unwrap();
        let df = match msg {
            Chunk::DataFrame(df) => df,
            _ => panic!("expected dataframe"),
        };

        assert_eq!(
            vec!["message"],
            df.columns().iter().map(|c| c.name()).collect::<Vec<&str>>()
        );

        assert_eq!(
            vec![vec![ValueView::Str("Hello, World! 1")]],
            df.columns()
                .into_iter()
                .map(|c| c.collect::<Vec<_>>())
                .collect::<Vec<_>>()
        );

        handle.abort();
        Ok(())
    }
}

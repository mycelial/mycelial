//! Bacalhau Source Section
//!
//! Monitors the configured folder on disk for file events, and then
//! after reading the entire JSON blob into a messages, sends it on
//! to a receiving destination section.
use super::{BacalhauPayload, Message, StdError};

use std::pin::Pin;
use std::time::Duration;
use std::{path::PathBuf, pin::pin};

use async_watcher::{notify::RecursiveMode, AsyncDebouncer};
use futures::{Future, FutureExt, Sink, SinkExt, Stream};
use section::{Command, Section, SectionChannel};

#[derive(Debug)]
pub struct Bacalhau {
    job: String,
    endpoint: String,
    outputs: String,
}

impl Default for Bacalhau {
    fn default() -> Self {
        Self::new("", "", "")
    }
}

impl Bacalhau {
    pub fn new(
        job: impl Into<String>,
        endpoint: impl Into<String>,
        outputs: impl Into<String>,
    ) -> Self {
        Self {
            job: job.into(),
            endpoint: endpoint.into(),
            outputs: outputs.into(),
        }
    }

    pub async fn enter_loop<Input, Output, SectionChan>(
        self,
        _input: Input,
        output: Output,
        mut section_chan: SectionChan,
    ) -> Result<(), StdError>
    where
        Input: Stream<Item = Message> + Send + 'static,
        Output: Sink<Message, Error = StdError> + Send + 'static,
        SectionChan: SectionChannel + Send + 'static,
    {
        let mut output = pin!(output);
        let (mut debouncer, mut file_events) =
            AsyncDebouncer::new_with_channel(Duration::from_secs(1), Some(Duration::from_secs(1)))
                .await?;

        let path: PathBuf = self.outputs.into();
        debouncer
            .watcher()
            .watch(&path, RecursiveMode::Recursive)
            .unwrap();

        loop {
            futures::select_biased! {
                cmd = section_chan.recv().fuse() => {
                    if let Command::Stop = cmd? {
                        return Ok(())
                    }
                }
                event = file_events.recv().fuse() => {
                    // TODO: It's like xmas..
                    let payloads: Vec<_> = event.unwrap().ok().unwrap().iter().map(
                        |e| {
                            let bytes = std::fs::read(&e.path).unwrap();
                            let bacalhau_payload: BacalhauPayload = serde_json::from_slice(&bytes).unwrap();
                            Message::new("bacalhau", bacalhau_payload, None)
                        }
                    ).collect();

                    for payload in payloads {
                        section_chan.log(&format!("sending message: '{:?}'", &payload)).await?;
                        output.send(payload).await.ok();

                        // TODO: delete the file
                    }
                }
            }
        }
    }
}

impl<Input, Output, SectionChan> Section<Input, Output, SectionChan> for Bacalhau
where
    Input: Stream<Item = Message> + Send + 'static,
    Output: Sink<Message, Error = StdError> + Send + 'static,
    SectionChan: SectionChannel + Send + 'static,
{
    type Error = StdError;
    type Future = Pin<Box<dyn Future<Output = Result<(), Self::Error>> + Send + 'static>>;

    fn start(self, input: Input, output: Output, command: SectionChan) -> Self::Future {
        Box::pin(async move { self.enter_loop(input, output, command).await })
    }
}

//! Bacalhau Destination Section
//!
//! Receives messages from source sections, and after extracting
//! information from the RecordBatch, posts it to the configured
//! endpoint, to be processed by an external process.

use super::{Message, StdError};
use std::pin::{pin, Pin};

use section::Section;

use futures::{Future, FutureExt, Sink, SinkExt, Stream, StreamExt};
use reqwest;
use section::{Command, SectionChannel};

impl Default for Bacalhau {
    fn default() -> Self {
        Self::new("Sample", "", "")
    }
}

#[derive(Debug)]
pub struct Bacalhau {
    pub job: String,
    pub endpoint: String,
    pub outputs: String,
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

    pub async fn submit_job(
        &self,
        id: impl Into<String>,
        msg: impl Into<String>,
    ) -> Result<(), StdError> {
        let _json_response: serde_json::Value = reqwest::Client::new()
            .post(self.endpoint.clone())
            .json(&serde_json::json!({
                "id": id.into(),
                "message": msg.into(),
            }))
            .send()
            .await?
            .json()
            .await?;

        Ok(())
    }

    pub async fn enter_loop<Input, Output, SectionChan>(
        self,
        input: Input,
        output: Output,
        mut section_chan: SectionChan,
    ) -> Result<(), StdError>
    where
        Input: Stream<Item = Message> + Send + 'static,
        Output: Sink<Message, Error = StdError> + Send + 'static,
        SectionChan: SectionChannel + Send + Sync + 'static,
    {
        let mut input = pin!(input.fuse());
        let mut output = pin!(output);
        loop {
            futures::select_biased! {
                cmd = section_chan.recv().fuse() => {
                    if let Command::Stop = cmd? {
                        return Ok(())
                    }
                }
                msg = input.next() => {
                    let msg = match msg {
                        Some(msg) => msg,
                        None => Err("input stream closed")?
                    };

                    let payload = &msg.payload;
                    let origin = &msg.origin;
                    self.submit_job(&payload.id, &payload.message).await?;

                    section_chan.log(&format!("Message from '{:?}' received! {:?}", origin, payload)).await?;
                    output.send(msg).await?;
                },
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

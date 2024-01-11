//! Mycelial Net
//!
//! network section, dumps incoming messages to provided http endpoint
use arrow::ipc::writer::StreamWriter;
use arrow_msg::df_to_recordbatch;
use base64::engine::{general_purpose::STANDARD as BASE64, Engine};
use reqwest::Body;
use section::{
    command_channel::{Command, SectionChannel},
    futures::{self, FutureExt, Sink, Stream, StreamExt, TryStreamExt},
    message::{Chunk, MessageStream},
    section::Section,
    SectionError, SectionFuture, SectionMessage,
};
use std::pin::pin;

#[derive(Debug)]
pub struct Mycelial {
    endpoint: String,
    token: String,
    topic: String,
}

impl Mycelial {
    pub fn new(
        endpoint: impl Into<String>,
        token: impl Into<String>,
        topic: impl Into<String>,
    ) -> Self {
        Self {
            endpoint: endpoint.into(),
            token: token.into(),
            topic: topic.into(),
        }
    }

    pub async fn enter_loop<Input, Output, SectionChan>(
        self,
        input: Input,
        _output: Output,
        mut section_chan: SectionChan,
    ) -> Result<(), SectionError>
    where
        Input: Stream<Item = SectionMessage> + Send,
        Output: Sink<SectionMessage, Error = SectionError> + Send,
        SectionChan: SectionChannel,
    {
        let mut input = pin!(input.fuse());
        let client = &mut reqwest::Client::new();
        loop {
            futures::select! {
                cmd = section_chan.recv().fuse() => {
                    if let Command::Stop = cmd? {
                        return Ok(())
                    }
                },

                msg = input.next() => {
                    let mut msg = match msg {
                        Some(msg) => msg,
                        None => Err("input stream closed")?
                    };
                    let origin = msg.origin().to_string();
                    let ack = msg.ack();
                    let msg_stream: MessageStream = msg.into();
                    let msg_stream = msg_stream
                        .map_ok(|chunk| {
                            match chunk {
                                Chunk::DataFrame(df) => {
                                    // FIXME: unwrap unwrap unwrap
                                    let rb = df_to_recordbatch(df.as_ref()).unwrap();
                                    let mut stream_writer: StreamWriter<_> = StreamWriter::try_new(vec![], rb.schema().as_ref()).unwrap();
                                    stream_writer.write(&rb).unwrap();
                                    stream_writer.finish().unwrap();

                                    stream_writer.into_inner().unwrap()
                                },
                                Chunk::Byte(bin) => bin,
                            }
                        });
                    let body = Body::wrap_stream(msg_stream);
                    let _ = client
                        .post(format!(
                            "{}/{}",
                            self.endpoint.as_str().trim_end_matches('/'),
                            self.topic
                        ))
                        .header("Authorization", self.basic_auth())
                        .header("x-message-origin", origin)
                        .body(body)
                        .send()
                        .await?;
                    ack.await;
                },
            }
        }
    }

    fn basic_auth(&self) -> String {
        format!("Basic {}", BASE64.encode(format!("{}:", self.token)))
    }
}

impl<Input, Output, SectionChan> Section<Input, Output, SectionChan> for Mycelial
where
    Input: Stream<Item = SectionMessage> + Send + 'static,
    Output: Sink<SectionMessage, Error = SectionError> + Send + 'static,
    SectionChan: SectionChannel,
{
    type Error = SectionError;
    type Future = SectionFuture;

    fn start(self, input: Input, output: Output, section_chan: SectionChan) -> Self::Future {
        Box::pin(async move { self.enter_loop(input, output, section_chan).await })
    }
}

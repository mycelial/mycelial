//! Mycelial Net

use arrow::ipc::reader::StreamReader;
use arrow_msg::ArrowMsg;
use reqwest::Client;
use section::{
    command_channel::{Command, SectionChannel, WeakSectionChannel},
    futures::{self, stream::FusedStream, FutureExt, Sink, SinkExt, Stream, StreamExt},
    section::Section,
    state::State,
    SectionError, SectionFuture, SectionMessage,
};
use tokio::time::{Instant, Interval};

use base64::engine::{general_purpose::STANDARD as BASE64, Engine};
use std::time::Duration;
use std::{
    io::Cursor,
    pin::{pin, Pin},
};

#[derive(Debug)]
pub struct Mycelial {
    /// endpoint URL
    endpoint: String,

    /// basic auth token
    token: String,

    /// topic
    topic: String,
}

struct IntervalStream {
    delay: Duration,
    interval: Interval,
}

impl IntervalStream {
    /// Create a new `IntervalStream`.
    pub fn new(delay: Duration) -> Self {
        Self {
            delay,
            interval: tokio::time::interval(delay),
        }
    }

    pub fn reset(&mut self) {
        self.interval = tokio::time::interval(self.delay)
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
        _input: Input,
        output: Output,
        mut section_chan: SectionChan,
    ) -> Result<(), SectionError>
    where
        Input: Stream<Item = SectionMessage> + Send + 'static,
        Output: Sink<SectionMessage, Error = SectionError> + Send + 'static,
        SectionChan: SectionChannel + Send + 'static,
    {
        let mut output = pin!(output);
        let mut client = reqwest::Client::new();
        let mut interval = pin!(IntervalStream::new(Duration::from_secs(3)));
        let mut state = section_chan.retrieve_state().await?.unwrap_or(State::new());
        let mut offset = state.get::<u64>(&self.topic)?.unwrap_or(0);
        loop {
            futures::select! {
                _ = interval.next() => {
                    match self.get_next_batch(&mut client, &section_chan, &mut offset).await {
                        Ok(Some(msg)) => {
                            output.send(msg).await.ok();
                            interval.reset();
                        },
                        Ok(None) => (),
                        Err(e) => section_chan.log(format!("failed to retrieve next batch: {:?}", e)).await?,
                    }
                },
                cmd = section_chan.recv().fuse() => {
                    match cmd? {
                        Command::Ack(ack) => {
                            match ack.downcast::<u64>() {
                                Ok(offset) => {
                                    state.set(&self.topic, *offset)?;
                                    section_chan.store_state(state.clone()).await?;
                                },
                                Err(_) =>
                                    break Err("Failed to downcast incoming Ack message".into()),
                            };
                        },
                        Command::Stop => {
                            return Ok(())
                        },
                        _ => (),
                    }
                }
            }
        }
    }

    async fn get_next_batch<SectionChan: SectionChannel>(
        &self,
        client: &mut Client,
        section_chan: &SectionChan,
        offset: &mut u64,
    ) -> Result<Option<SectionMessage>, SectionError> {
        let res = client
            .get(format!(
                "{}/{}/{}",
                self.endpoint.as_str().trim_end_matches('/'),
                self.topic,
                offset
            ))
            .header("Authorization", self.basic_auth())
            .send()
            .await?;

        let origin = match res.headers().get("x-message-origin") {
            None => Err("response needs to have x-message-origin header")?,
            Some(v) => v.to_str().unwrap().to_string(),
        };

        let maybe_new_offset = match res.headers().get("x-message-id") {
            None => Err("response needs to have x-message-id header")?,
            // FIXME: unwrap
            Some(v) => v.to_str().unwrap().parse().unwrap(),
        };

        if maybe_new_offset == *offset {
            return Ok(None);
        }
        *offset = maybe_new_offset;

        let body = res.bytes().await?.to_vec();
        let len = body.len() as u64;
        let mut body = Cursor::new(body);
        let mut batches = vec![];
        while body.position() < len {
            let reader = StreamReader::try_new_unbuffered(&mut body, None).unwrap();
            for batch in reader {
                let batch = batch?;
                batches.push(Some(batch.into()))
            }
        }
        let weak_chan = section_chan.weak_chan();
        let o = *offset;
        let msg = ArrowMsg::new(
            origin,
            batches,
            Some(Box::pin(async move { weak_chan.ack(Box::new(o)).await })),
        );
        Ok(Some(Box::new(msg)))
    }

    fn basic_auth(&self) -> String {
        format!("Basic {}", BASE64.encode(format!("{}:", self.token)))
    }
}

impl<Input, Output, SectionChan> Section<Input, Output, SectionChan> for Mycelial
where
    Input: Stream<Item = SectionMessage> + Send + 'static,
    Output: Sink<SectionMessage, Error = SectionError> + Send + 'static,
    SectionChan: SectionChannel + Send + 'static,
{
    // FIXME: define proper error
    type Error = SectionError;
    type Future = SectionFuture;

    fn start(self, input: Input, output: Output, section_chan: SectionChan) -> Self::Future {
        Box::pin(async move { self.enter_loop(input, output, section_chan).await })
    }
}

//! Mycelial Net

use crate::{
    config::Map,
    message::{Message, RecordBatch},
    types::{DynSection, SectionError, SectionFuture},
};
use arrow::ipc::reader::StreamReader;
use futures::{stream::FusedStream, FutureExt};
use futures::{Sink, SinkExt, Stream, StreamExt};
use reqwest::Client;
use section::{Command, Section, SectionChannel, State, WeakSectionChannel};
use tokio::time::{Instant, Interval};

use base64::engine::{general_purpose::STANDARD as BASE64, Engine};
use std::pin::{pin, Pin};
use std::time::Duration;

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
        Input: Stream<Item = Message> + Send + 'static,
        Output: Sink<Message, Error = SectionError> + Send + 'static,
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
                                    break Err("Failed to downcast incoming Ack message to SqliteRecordBatch".into()),
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
    ) -> Result<Option<Message>, SectionError> {
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

        // FIXME: it's not possible to stream to arrow's StreamReader directl
        // StreamReader expects sync std::io::Read implementation

        let body = res.bytes().await?.to_vec();
        // FIXME: unwrap
        let reader = StreamReader::try_new(body.as_slice(), None).unwrap();
        let weak_chan = section_chan.weak_chan();
        let vec = reader.collect::<Result<Vec<_>, _>>()?;
        match vec {
            mut vec if vec.len() == 1 => {
                let batch = RecordBatch(vec.pop().unwrap());
                let o = *offset;
                let message = Message::new(
                    origin,
                    batch,
                    Some(Box::pin(async move { weak_chan.ack(Box::new(o)).await })),
                );
                Ok(Some(message))
            }
            _ => Err("multiple batches are not supported")?,
        }
    }

    fn basic_auth(&self) -> String {
        format!("Basic {}", BASE64.encode(format!("{}:", self.token)))
    }
}

impl<Input, Output, SectionChan> Section<Input, Output, SectionChan> for Mycelial
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

/// constructor for mycelial net section
///
/// # Config example:
/// ```toml
/// [[section]]
/// name = "mycelial_net"
/// endpoint = "http://localhost:8080/ingestion"
/// token = "token"
/// topic = "some_topic"
/// ```
pub fn constructor<S: SectionChannel>(config: &Map) -> Result<Box<dyn DynSection<S>>, SectionError> {
    let endpoint = config
        .get("endpoint")
        .ok_or("mycelial net section requires 'endpoint' url")?
        .as_str()
        .ok_or("endpoint should be string")?;
    let token = config
        .get("token")
        .ok_or("mycelian net section requires 'token'")?
        .as_str()
        .ok_or("token should be string")?;
    // FIXME: validate topic is not empty
    let topic = config
        .get("topic")
        .ok_or("mycelian net section requires 'topic'")?
        .as_str()
        .ok_or("token should be string")?;
    Ok(Box::new(Mycelial::new(endpoint, token, topic)))
}

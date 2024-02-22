//! Mycelial Net

use arrow_msg::{arrow::ipc::reader::StreamReader, ArrowMsg};
use reqwest::Client;
use section::{
    command_channel::{Command, SectionChannel, WeakSectionChannel},
    futures::{self, stream::FusedStream, FutureExt, Sink, SinkExt, Stream, StreamExt},
    message::{Ack, Chunk, Message},
    section::Section,
    state::State,
    SectionError, SectionFuture, SectionMessage,
};
use tokio::time::{Instant, Interval};

use std::time::Duration;
use std::{
    io::Cursor,
    pin::{pin, Pin},
};

#[derive(Debug)]
pub struct Mycelial {
    /// endpoint URL
    endpoint: String,

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
    pub fn new(endpoint: impl Into<String>, topic: impl Into<String>) -> Self {
        Self {
            endpoint: endpoint.into(),
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
                    match self.get_next_chunk(&mut client, &section_chan, &mut offset).await {
                        Ok(Some(msg)) => {
                            output.send(msg).await.ok();
                            interval.reset();
                        },
                        Ok(None) => (),
                        Err(e) => section_chan.log(format!("failed to retrieve next chunk: {:?}", e)).await?,
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

    async fn get_next_chunk<SectionChan: SectionChannel>(
        &self,
        client: &mut Client,
        section_chan: &SectionChan,
        offset: &mut u64,
    ) -> Result<Option<SectionMessage>, SectionError> {
        let response = client
            .get(format!(
                "{}/{}/{}",
                self.endpoint.as_str().trim_end_matches('/'),
                self.topic,
                offset
            ))
            .send()
            .await?;

        let origin = match response.headers().get("x-message-origin") {
            None => Err("response needs to have x-message-origin header")?,
            Some(v) => v
                .to_str()
                .map_err(|_| "bad header 'x-message-origin' value")?
                .to_string(),
        };

        let stream_type = match response.headers().get("x-stream-type") {
            None => Err("response needs to have x-stream-type header")?,
            Some(v) => v
                .to_str()
                .map_err(|_| "bad header 'x-stream-type' value")?
                .to_string(),
        };

        let maybe_new_offset = match response.headers().get("x-message-id") {
            None => Err("response needs to have x-message-id header")?,
            Some(v) => v
                .to_str()
                .map_err(|_| "bad header 'x-message-id' value")?
                .parse()?,
        };

        if maybe_new_offset == *offset {
            return Ok(None);
        }
        *offset = maybe_new_offset;

        let weak_chan = section_chan.weak_chan();
        let ack: Option<Ack> = Some(Box::pin(async move {
            weak_chan.ack(Box::new(maybe_new_offset)).await
        }));
        match stream_type.as_str() {
            "binary" => {
                let stream = response.bytes_stream().map(|chunk| {
                    chunk
                        .map(|bytes| bytes.to_vec())
                        .map_err(|err| -> SectionError { err.into() })
                });
                Ok(Some(Box::new(BinStream::new(origin, stream, ack))))
            }
            "arrow" => {
                // FIXME: add async stream interface for arrow StreamReader
                let body = response.bytes().await?.to_vec();
                let len = body.len() as u64;
                let mut body = Cursor::new(body);
                let mut batches = vec![];
                while body.position() < len {
                    let reader = StreamReader::try_new_unbuffered(&mut body, None)?;
                    for batch in reader {
                        batches.push(Some(batch?.into()))
                    }
                }
                Ok(Some(Box::new(ArrowMsg::new(origin, batches, ack))))
            }
            _ => Err(format!("unsupported stream_type '{stream_type}'"))?,
        }
    }
}

struct BinStream<T> {
    origin: String,
    stream: T,
    ack: Option<Ack>,
}

impl<T> std::fmt::Debug for BinStream<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BinStream")
            .field("origin", &self.origin)
            .finish()
    }
}

impl<T> BinStream<T> {
    fn new(origin: impl Into<String>, stream: T, ack: Option<Ack>) -> Self {
        Self {
            origin: origin.into(),
            stream,
            ack,
        }
    }
}

impl<T: Stream<Item = Result<Vec<u8>, SectionError>> + Send + Unpin> Message for BinStream<T> {
    fn origin(&self) -> &str {
        self.origin.as_str()
    }

    fn next(&mut self) -> section::message::Next<'_> {
        Box::pin(async {
            match self.stream.next().await {
                Some(Ok(b)) => Ok(Some(Chunk::Byte(b))),
                Some(Err(e)) => Err(e),
                None => Ok(None),
            }
        })
    }

    fn ack(&mut self) -> Ack {
        self.ack.take().unwrap_or(Box::pin(async {}))
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

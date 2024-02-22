//! Mycelial Net
//!
//! network section, dumps incoming messages to provided http endpoint
use arrow_msg::{arrow::ipc::writer::StreamWriter, df_to_recordbatch};
use async_stream::stream;
use reqwest::Body;
use section::{
    command_channel::{Command, SectionChannel},
    futures::{self, FutureExt, Sink, Stream, StreamExt, TryStreamExt},
    message::{Chunk, Message},
    section::Section,
    SectionError, SectionFuture, SectionMessage,
};
use std::fmt::Display;
use std::{
    pin::{pin, Pin},
    task::{Context, Poll},
};

#[derive(Debug)]
pub struct Mycelial {
    endpoint: String,
    topic: String,
}

// should we just introduce additional method in message trait to indicate stream type?
#[derive(Debug)]
pub(crate) enum StreamType<T> {
    DataFrame(T),
    BinStream(T),
}

impl<T> StreamType<T> {
    fn into_inner(self) -> T {
        match self {
            Self::DataFrame(s) => s,
            Self::BinStream(s) => s,
        }
    }
}

impl<T> Display for StreamType<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let desc = match self {
            StreamType::DataFrame(_) => "arrow", // dataframe will be converted to arrow record batch
            StreamType::BinStream(_) => "binary",
        };
        write!(f, "{}", desc)
    }
}

async fn to_stream(
    mut msg: Box<dyn Message>,
) -> StreamType<impl Stream<Item = Result<Chunk, SectionError>>> {
    let chunk = msg.next().await;
    let is_df = matches!(chunk, Ok(Some(Chunk::DataFrame(_))));
    let stream = stream! {
        match chunk {
            Ok(Some(v)) => yield Ok(v),
            Err(e) => yield Err(e),
            Ok(None) => return
        }
        loop {
            match msg.next().await {
                Ok(Some(v)) => yield Ok(v),
                Err(e) => yield Err(e),
                Ok(None) => return
            }
        }
    };
    match is_df {
        true => StreamType::DataFrame(stream),
        false => StreamType::BinStream(stream),
    }
}

struct S<T: Stream> {
    inner: T,
}

unsafe impl<T: Stream> Sync for S<T> {}

impl<T: Stream> Stream for S<T> {
    type Item = <T as Stream>::Item;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        unsafe {
            let this = self.get_unchecked_mut();
            Stream::poll_next(Pin::new_unchecked(&mut this.inner), cx)
        }
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
                    let msg_stream = to_stream(msg).await;
                    let stream_type = msg_stream.to_string();
                    let msg_stream = msg_stream
                        .into_inner()
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
                    let body = Body::wrap_stream(S{ inner: msg_stream });
                    let _ = client
                        .post(format!(
                            "{}/{}",
                            self.endpoint.as_str().trim_end_matches('/'),
                            self.topic
                        ))
                        .header("x-message-origin", origin)
                        .header("x-stream-type", stream_type)
                        .body(body)
                        .send()
                        .await?;
                    ack.await;
                },
            }
        }
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

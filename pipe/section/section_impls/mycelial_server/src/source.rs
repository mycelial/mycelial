//! Mycelial Net

use reqwest::Client;
use section::{
    decimal,
    futures::{self, stream::FusedStream, Stream, StreamExt, Sink, SinkExt, FutureExt},
    command_channel::{Command, SectionChannel, WeakSectionChannel},
    state::State,
    section::Section, SectionMessage, SectionError, SectionFuture, message::{DataFrame, ValueView, DataType, Column, Message, Ack, Chunk}
};
use arrow::{
    ipc::reader::StreamReader,
    datatypes::{
        DataType as ArrowDataType,
        Field,
        Schema, TimeUnit, UnionFields, UnionMode,
        DECIMAL128_MAX_PRECISION, DECIMAL_DEFAULT_SCALE, Int8Type, Int16Type, UInt8Type, UInt16Type, UInt32Type, UInt64Type, Float32Type, Float64Type, Int64Type, Int32Type, BooleanType, Time64MicrosecondType, Date64Type, TimestampMicrosecondType, Decimal128Type,
    },
    record_batch::RecordBatch as ArrowRecordBatch, array::AsArray,
};
use tokio::time::{Instant, Interval};

use base64::engine::{general_purpose::STANDARD as BASE64, Engine};
use std::{pin::{pin, Pin}, sync::Arc, io::Cursor};
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

// Wrap around arrow record batch, which implements dataframe
#[derive(Debug)]
struct RecordBatch{
    inner: ArrowRecordBatch,
    schema: Arc<Schema>
}

impl DataFrame for RecordBatch {
    fn columns(&self) -> Vec<section::message::Column<'_>> {
        self
            .schema
            .fields()
            .iter()
            .zip(self.inner.columns())
            .map(|(field, column)| {
                let (dt, iter): (DataType, Box<dyn Iterator<Item=ValueView> + Send>) = match field.data_type() {
                    ArrowDataType::Int8 => {
                        let arr = column.as_primitive::<Int8Type>();
                        (
                            DataType::I8,
                            Box::new(arr.iter().map(|val| val.map(ValueView::I8).unwrap_or(ValueView::Null)))
                        )
                    },
                    ArrowDataType::Int16 => {
                        let arr = column.as_primitive::<Int16Type>();
                        (
                            DataType::I16,
                            Box::new(arr.iter().map(|val| val.map(ValueView::I16).unwrap_or(ValueView::Null)))
                        )
                    },
                    ArrowDataType::Int32 => {
                        let arr = column.as_primitive::<Int32Type>();
                        (
                            DataType::I32,
                            Box::new(arr.iter().map(|val| val.map(ValueView::I32).unwrap_or(ValueView::Null)))
                        )
                    },
                    ArrowDataType::Int64 => {
                        let arr = column.as_primitive::<Int64Type>();
                        (
                            DataType::I64,
                            Box::new(arr.iter().map(|val| val.map(ValueView::I64).unwrap_or(ValueView::Null)))
                        )
                    },
                    ArrowDataType::UInt8 => {
                        let arr = column.as_primitive::<UInt8Type>();
                        (
                            DataType::U8,
                            Box::new(arr.iter().map(|val| val.map(ValueView::U8).unwrap_or(ValueView::Null)))
                        )
                    },
                    ArrowDataType::UInt16 => {
                        let arr = column.as_primitive::<UInt16Type>();
                        (
                            DataType::U16,
                            Box::new(arr.iter().map(|val| val.map(ValueView::U16).unwrap_or(ValueView::Null)))
                        )
                    },
                    ArrowDataType::UInt32 => {
                        let arr = column.as_primitive::<UInt32Type>();
                        (
                            DataType::U32,
                            Box::new(arr.iter().map(|val| val.map(ValueView::U32).unwrap_or(ValueView::Null)))
                        )
                    },
                    ArrowDataType::UInt64 => {
                        let arr = column.as_primitive::<UInt64Type>();
                        (
                            DataType::U64,
                            Box::new(arr.iter().map(|val| val.map(ValueView::U64).unwrap_or(ValueView::Null)))
                        )
                    },
                    ArrowDataType::Float32 => {
                        let arr = column.as_primitive::<Float32Type>();
                        (
                            DataType::F32,
                            Box::new(arr.iter().map(|val| val.map(ValueView::F32).unwrap_or(ValueView::Null)))
                        )
                    },
                    ArrowDataType::Float64 => {
                        let arr = column.as_primitive::<Float64Type>();
                        (
                            DataType::F64,
                            Box::new(arr.iter().map(|val| val.map(ValueView::F64).unwrap_or(ValueView::Null)))
                        )
                    },
                    ArrowDataType::Utf8 => {
                        let arr = column.as_string::<i32>();
                        (
                            DataType::Str,
                            Box::new(arr.iter().map(|val| val.map(ValueView::Str).unwrap_or(ValueView::Null)))
                        )
                    },
                    ArrowDataType::Binary => {
                        let arr = column.as_binary::<i32>();
                        (
                            DataType::Bin,
                            Box::new(arr.iter().map(|val| val.map(ValueView::Bin).unwrap_or(ValueView::Null)))
                        )
                    },
                    ArrowDataType::Boolean => {
                        let arr = column.as_boolean();
                        (
                            DataType::Bool,
                            Box::new(arr.iter().map(|val| val.map(ValueView::Bool).unwrap_or(ValueView::Null)))
                        )
                    },
                    ArrowDataType::Time64(_tu) => {
                        let arr = column.as_primitive::<Time64MicrosecondType>();
                        (
                            DataType::Time,
                            Box::new(arr.iter().map(|val| val.map(ValueView::Time).unwrap_or(ValueView::Null)))
                        )
                    },
                    ArrowDataType::Date64 => {
                        let arr = column.as_primitive::<Date64Type>();
                        (
                            DataType::Date,
                            Box::new(arr.iter().map(|val| val.map(ValueView::Date).unwrap_or(ValueView::Null)))
                        )
                    },
                    ArrowDataType::Timestamp(_tu, _tz) => {
                        let arr = column.as_primitive::<TimestampMicrosecondType>();
                        (
                            DataType::TimeStamp,
                            Box::new(arr.iter().map(|val| val.map(ValueView::TimeStamp).unwrap_or(ValueView::Null)))
                        )
                    },
                    ArrowDataType::Null => {
                        let arr = column.as_primitive::<Int8Type>();
                        (
                            DataType::Null,
                            Box::new(arr.iter().map(|_| ValueView::Null))
                        )
                    },
                    ArrowDataType::Decimal128(precision, scale) => {
                        let arr = column.as_primitive::<Decimal128Type>();
                        (
                            DataType::Decimal,
                            Box::new(arr.iter().map(|val| 
                                val.map(|num| 
                                        ValueView::Decimal(decimal::Decimal::from_i128_with_scale(num, *scale as _))
                                ).unwrap_or(ValueView::Null)
                            ))
                        )
                    },
                    ArrowDataType::Union(_uf, _mode) => {
                        unimplemented!()
                    },
                    dt => panic!("unsupported arrow datatype: {:?}", dt),
                };
                Column::new(field.name(), dt, iter)
        })
            .collect()
    }
}

struct Msg {
    inner: Vec<Option<RecordBatch>>,
    pos: usize,
    ack: Option<Ack>,
    origin: String,
}

impl std::fmt::Debug for Msg {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Msg")
            .field("inner", &self.inner)
            .field("pos", &self.pos)
            .field("origin", &self.origin)
            .finish()
    }
}

impl Message for Msg {
    fn origin(&self) -> &str {
        self.origin.as_str()
    }

    fn next(&mut self) -> section::message::Next<'_> {
        match self.pos >= self.inner.len() {
            true => Box::pin(async { Ok(None) }),
            false => {
                let rb = self.inner[self.pos].take().unwrap();
                self.pos += 1;
                Box::pin(async move { 
                    Ok(Some(Chunk::DataFrame(Box::new(rb))))
                })
            }
        }
    }

    fn ack(&mut self) -> section::message::Ack {
        self.ack.take().unwrap_or(Box::pin(async {}))
    }

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
                batches.push(Some(RecordBatch{ schema: batch.schema(), inner: batch }))
            }
        };
        let weak_chan = section_chan.weak_chan();
        let o = *offset;
        let msg = Msg {
            inner: batches,
            pos: 0,
            origin,
            ack: Some( Box::pin(async move { weak_chan.ack(Box::new(o)).await })),
        };
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

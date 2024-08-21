//! convert incoming dataframe into binary stream

use std::pin::pin;

use chrono::{DateTime, NaiveDateTime};
use section::{
    command_channel::{Command, SectionChannel},
    futures::{self, FutureExt, Sink, SinkExt, Stream, StreamExt},
    message::{Ack, Chunk, Column, Message, TimeUnit, ValueView},
    section::Section,
    SectionError, SectionFuture, SectionMessage,
};
use tokio::sync::mpsc::{channel, Receiver, Sender};
use crate::ToCsv;


impl ToCsv {
    pub fn new(buf_size: usize) -> Self {
        Self { buf_size }
    }

    fn get_writer(&self) -> csv::Writer<Vec<u8>> {
        csv::Writer::from_writer(Vec::<u8>::with_capacity(self.buf_size))
    }

    fn write_header(
        &self,
        writer: &mut csv::Writer<Vec<u8>>,
        columns: &[Column<'_>],
    ) -> Result<(), SectionError> {
        for column in columns {
            let name = column.name();
            writer.write_field(name)?;
        }
        writer.write_record(None::<&[u8]>)?;
        writer.flush()?;
        Ok(())
    }

    async fn maybe_send_chunk(
        &self,
        writer: &mut csv::Writer<Vec<u8>>,
        tx: &Sender<Option<Chunk>>,
        last: bool,
    ) -> Result<(), SectionError> {
        let current_pos = writer.get_ref().len();
        if (current_pos != 0 && last) || (current_pos >= self.buf_size) {
            let mut new_writer = self.get_writer();
            std::mem::swap(&mut new_writer, writer);
            let mut buf = new_writer.into_inner()?;
            unsafe { buf.set_len(current_pos) };
            tx.send(Some(Chunk::Byte(buf)))
                .await
                .map_err(|_| "stream error")?;
        }
        if last {
            tx.send(None).await.map_err(|_| "stream error")?;
        }
        Ok(())
    }
}

struct ToCsvMsg {
    origin: String,
    ack: Option<Ack>,
    rx: Receiver<Option<Chunk>>,
}

impl std::fmt::Debug for ToCsvMsg {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ToCvsMsg")
            .field("origin", &self.origin)
            .finish()
    }
}

impl ToCsvMsg {
    fn new(origin: String, ack: Ack, rx: Receiver<Option<Chunk>>) -> Self {
        Self {
            origin,
            ack: Some(ack),
            rx,
        }
    }
}

impl Message for ToCsvMsg {
    fn origin(&self) -> &str {
        self.origin.as_str()
    }

    fn next(&mut self) -> section::message::Next<'_> {
        Box::pin(async move {
            match self.rx.recv().await {
                Some(res) => Ok(res),
                None => Err("stream closed".into()),
            }
        })
    }

    fn ack(&mut self) -> Ack {
        self.ack.take().unwrap_or(Box::pin(async {}))
    }
}

impl<Input, Output, SectionChan> Section<Input, Output, SectionChan> for ToCsv
where
    Input: Stream<Item = SectionMessage> + Send + 'static,
    Output: Sink<SectionMessage, Error = SectionError> + Send + 'static,
    SectionChan: SectionChannel,
{
    type Error = SectionError;
    type Future = SectionFuture;

    fn start(self, input: Input, output: Output, mut section_channel: SectionChan) -> Self::Future {
        Box::pin(async move {
            let mut input = pin!(input);
            let mut output = pin!(output);
            loop {
                futures::select! {
                    cmd = section_channel.recv().fuse() => {
                        if let Command::Stop = cmd? {
                            return Ok(())
                        }
                    },
                    msg = input.next().fuse() => {
                        let mut msg = msg.ok_or("input closed")?;
                        let mut header_written = false;
                        let (tx, rx) = channel(1);
                        let out_msg = ToCsvMsg::new(msg.origin().to_string(), msg.ack(), rx);
                        output.send(Box::new(out_msg)).await?;
                        let mut writer = self.get_writer();
                        while let Some(chunk) = msg.next().await? {
                            let df = match chunk {
                                Chunk::DataFrame(df) => df,
                                _ => Err("csv destination expects dataframe input")?
                            };
                            let mut columns = df.columns();
                            if !header_written {
                                header_written = true;
                                self.write_header(&mut writer, columns.as_slice())?;
                                self.maybe_send_chunk(&mut writer, &tx, false).await?;
                            }
                            'outer: loop {
                                for column in columns.iter_mut() {
                                    let value = match column.next() {
                                        Some(value) => value,
                                        None => break 'outer
                                    };
                                    match value {
                                        ValueView::Null => {
                                            writer.write_field([])?;
                                        },
                                        ValueView::Bin(_) => {
                                            Err(format!("'{}' is a binary column, which are not supported", column.name()))?
                                        },
                                        ValueView::Time(tu, t) => {
                                            let datetime = to_naive_datetime(tu, t)
                                                .ok_or(format!("failed to convert '{}' to naivedate", column.name()))?;
                                            writer.write_field(datetime.time().to_string())?;
                                        },
                                        ValueView::Date(tu, t) => {
                                            let datetime = to_naive_datetime(tu, t)
                                                .ok_or(format!("failed to convert '{}' to naivedate", column.name()))?;
                                            writer.write_field(datetime.date().to_string())?;
                                        },
                                        ValueView::TimeStamp(tu, t) | ValueView::TimeStampUTC(tu, t) => {
                                            let datetime = to_naive_datetime(tu, t)
                                                .ok_or(format!("failed to convert '{}' to naivedate", column.name()))?;
                                            writer.write_field(datetime.to_string())?;
                                        }
                                        _ => {
                                            let value = value.to_string();
                                            writer.write_field(&value)?;
                                        }
                                    }
                                }
                                writer.write_record(None::<&[u8]>)?;
                                writer.flush()?;
                                self.maybe_send_chunk(&mut writer, &tx, false).await?;
                            }
                        }
                        self.maybe_send_chunk(&mut writer, &tx, true).await?;
                    },
                }
            }
        })
    }
}

// FIXME: move this function to lib
fn to_naive_datetime(tu: TimeUnit, t: i64) -> Option<NaiveDateTime> {
    match tu {
        TimeUnit::Second => DateTime::from_timestamp(t, 0),
        TimeUnit::Millisecond => DateTime::from_timestamp_micros(t * 1000),
        TimeUnit::Microsecond => DateTime::from_timestamp_micros(t),
        TimeUnit::Nanosecond => {
            DateTime::from_timestamp(t / 1_000_000_000, (t % 1_000_000_000) as _)
        }
    }
    .map(|d| d.naive_utc())
}

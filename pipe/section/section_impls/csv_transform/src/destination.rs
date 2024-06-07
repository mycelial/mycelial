//! convert incoming dataframe into binary stream

use std::pin::pin;

use section::{
    command_channel::{Command, SectionChannel},
    futures::{self, FutureExt, Sink, SinkExt, Stream, StreamExt},
    message::{Ack, Chunk, Message},
    section::Section,
    SectionError, SectionFuture, SectionMessage,
};
use tokio::sync::mpsc::{channel, Receiver};

#[derive(Debug)]
pub struct ToCsv {
    buf_size: usize,
}

impl ToCsv {
    pub fn new(buf_size: usize) -> Self {
        Self { buf_size }
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

fn get_writer(buf_size: usize) -> csv::Writer<Vec<u8>> {
    csv::Writer::from_writer(Vec::<u8>::with_capacity(buf_size))
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
                        let mut writer = get_writer(self.buf_size);
                        while let Some(chunk) = msg.next().await? {
                            let df = match chunk {
                                Chunk::DataFrame(df) => df,
                                _ => Err("csv destination expects dataframe input")?
                            };
                            let mut columns = df.columns();
                            if !header_written {
                                header_written = true;
                                for column in columns.iter() {
                                    let name = column.name();
                                    writer.write_field(name)?;
                                }
                                writer.write_record(None::<&[u8]>)?;
                            }
                            'outer: loop {
                                for column in columns.iter_mut() {
                                    let value = match column.next() {
                                        Some(value) => value,
                                        None => break 'outer
                                    };
                                    let value = value.to_string();
                                    writer.write_field(&value)?;
                                }
                                writer.write_record(None::<&[u8]>)?;
                                writer.flush()?;
                                let current_pos = writer.get_ref().len();
                                if current_pos >= self.buf_size {
                                    let mut buf = writer.into_inner()?;
                                    unsafe { buf.set_len(current_pos as usize) };
                                    tx.send(Some(Chunk::Byte(buf))).await.map_err(|_| "stream error")?;
                                    writer = get_writer(self.buf_size);
                                }
                            }
                        }
                        let current_pos = writer.get_ref().len();
                        if current_pos != 0 {
                            let mut buf = writer.into_inner()?;
                            unsafe { buf.set_len(current_pos as usize) };
                            tx.send(Some(Chunk::Byte(buf))).await.map_err(|_| "stream error")?;
                        }
                        tx.send(None).await.map_err(|_| "stream error")?;
                    },
                }
            }
        })
    }
}

//! Transforms incoming binary csv stream into dataframe stream

use csv::StringRecord;
use section::{
    command_channel::{Command, SectionChannel},
    futures::{self, FutureExt, Sink, SinkExt, Stream, StreamExt},
    message::{Ack, Chunk, Column, DataFrame, DataType, Message, Next, ValueView},
    section::Section,
    SectionError, SectionFuture, SectionMessage,
};
use std::{pin::pin, sync::Arc};
use tokio::sync::mpsc::{channel, Receiver, Sender};
use crate::FromCsv;

type Result<T, E = SectionError> = std::result::Result<T, E>;


struct FromCsvMsg {
    origin: String,
    ack: Option<Ack>,
    rx: Receiver<Result<Option<Chunk>>>,
}

impl std::fmt::Debug for FromCsvMsg {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FromCsvMsg")
            .field("origin", &self.origin)
            .finish()
    }
}

impl FromCsvMsg {
    fn new(origin: &str, ack: Ack, rx: Receiver<Result<Option<Chunk>>>) -> Self {
        Self {
            origin: origin.into(),
            ack: Some(ack),
            rx,
        }
    }
}

impl Message for FromCsvMsg {
    fn origin(&self) -> &str {
        self.origin.as_str()
    }

    fn ack(&mut self) -> section::message::Ack {
        self.ack.take().unwrap_or(Box::pin(async {}))
    }

    fn next(&mut self) -> Next<'_> {
        Box::pin(async move {
            match self.rx.recv().await {
                None => Err("FromCsvMsg error: receiver closed".into()),
                Some(msg) => msg,
            }
        })
    }
}

struct ReceiverReader {
    rx: Receiver<Option<Vec<u8>>>,
    buf: Option<Vec<u8>>,
    closed: bool,
    offset: u64,
}

impl ReceiverReader {
    fn new(rx: Receiver<Option<Vec<u8>>>) -> Self {
        Self {
            rx,
            buf: None,
            closed: false,
            offset: 0,
        }
    }
}

impl std::io::Read for ReceiverReader {
    fn read(&mut self, out: &mut [u8]) -> std::io::Result<usize> {
        if self.closed {
            return Ok(0);
        }
        if self.buf.is_none() {
            self.offset = 0;
            self.buf = match self.rx.blocking_recv() {
                None => Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "receiver reader error: receiver closed",
                ))?,
                Some(buf) => buf,
            }
        }
        let mut reader = match self.buf {
            None => {
                self.closed = true;
                return Ok(0);
            }
            Some(ref buf) => {
                let mut available = buf.len() as u64 - self.offset;
                if available > out.len() as u64 {
                    available = out.len() as u64
                }
                std::io::Cursor::new(
                    &buf.as_slice()[self.offset as usize..(self.offset + available) as usize],
                )
            }
        };
        let read = std::io::copy(&mut reader, &mut std::io::Cursor::new(out))?;
        self.offset += read;
        if self.offset == self.buf.as_ref().unwrap().len() as u64 {
            self.buf = None;
        }
        Ok(read as usize)
    }
}

#[derive(Debug)]
struct CsvDataFrame {
    header: Arc<StringRecord>,
    batch: Vec<StringRecord>,
}

impl DataFrame for CsvDataFrame {
    fn columns(&self) -> Vec<section::message::Column<'_>> {
        (&*self.header)
            .into_iter()
            .enumerate()
            .map(|(pos, name)| {
                Column::new(
                    name,
                    DataType::Str,
                    Box::new(
                        self.batch
                            .iter()
                            .map(move |record| ValueView::Str(&record[pos])),
                    ),
                )
            })
            .collect()
    }
}

impl CsvDataFrame {
    fn new(header: Arc<StringRecord>, batch: Vec<StringRecord>) -> Self {
        Self { header, batch }
    }
}

fn bin_to_dataframe(
    batch_size: usize,
    rx: Receiver<Option<Vec<u8>>>,
    tx: Sender<Result<Option<Chunk>>>,
) -> Result<()> {
    let mut reader = csv::Reader::from_reader(ReceiverReader::new(rx));
    let header = Arc::new(reader.headers()?.clone());
    let mut batch = vec![];
    for record in reader.records() {
        batch.push(record?);
        if batch.len() >= batch_size {
            let mut new_batch = vec![];
            std::mem::swap(&mut new_batch, &mut batch);
            let df = Box::new(CsvDataFrame::new(Arc::clone(&header), new_batch));
            tx.blocking_send(Ok(Some(Chunk::DataFrame(df))))
                .map_err(|_| "send error")?;
        }
    }
    if !batch.is_empty() {
        let df = Box::new(CsvDataFrame::new(Arc::clone(&header), batch));
        tx.blocking_send(Ok(Some(Chunk::DataFrame(df))))
            .map_err(|_| "send error")?;
    }
    tx.blocking_send(Ok(None)).map_err(|_| "send error")?;
    Ok(())
}

async fn stream_in(mut msg: SectionMessage, tx: Sender<Option<Vec<u8>>>) -> Result<()> {
    loop {
        match msg.next().await {
            Ok(Some(Chunk::Byte(bin))) => {
                tx.send(Some(bin)).await?;
            }
            Ok(None) => {
                tx.send(None).await?;
                return Ok(());
            }
            Ok(Some(Chunk::DataFrame(_))) => Err("FromCsv section expects binary input")?,
            Err(e) => Err(e)?,
        }
    }
}

async fn stream_out(
    batch_size: usize,
    rx: Receiver<Option<Vec<u8>>>,
    tx: Sender<Result<Option<Chunk>>>,
) -> Result<()> {
    match tokio::task::spawn_blocking(move || bin_to_dataframe(batch_size, rx, tx)).await {
        Ok(Ok(())) => Ok(()),
        Ok(Err(e)) => Err(e)?,
        Err(e) => Err(format!("join error: {e}"))?,
    }
}

impl<Input, Output, SectionChan> Section<Input, Output, SectionChan> for FromCsv
where
    Input: Stream<Item = SectionMessage> + Send + 'static,
    Output: Sink<SectionMessage, Error = SectionError> + Send + 'static,
    SectionChan: SectionChannel + Send + 'static,
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
                        let mut msg = match msg {
                            None => Err("input closed")?,
                            Some(msg) => msg,
                        };
                        let (tx_in, rx_in) = channel(1);
                        let (tx_out, rx_out) = channel(1);
                        let ack = msg.ack();
                        let out = FromCsvMsg::new(msg.origin(), ack, rx_out);
                        output.send(Box::new(out)).await?;
                        futures::try_join!(
                            stream_in(msg, tx_in),
                            stream_out(self.batch_size, rx_in, tx_out)
                        )?;
                    }
                }
            }
        })
    }
}

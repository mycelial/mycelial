//! Mycelite journal source

use arrow::array::{Array, BinaryArray, Int64Array, UInt32Array, UInt64Array};
use arrow::datatypes::{DataType, Field, Schema};
use arrow::record_batch::RecordBatch;
use futures::{FutureExt, Sink, SinkExt, Stream, StreamExt};
use notify::{Event, RecursiveMode, Watcher};
use section::{Command, Section, SectionChannel, State, WeakSectionChannel};
use std::future::Future;
use std::path::Path;
use std::pin::{pin, Pin};
use std::sync::Arc;
use tokio::sync::mpsc::Sender;

#[derive(Debug)]
pub struct Mycelite {
    journal_path: String,
    cur_snapshot: Option<u64>,
    buf: Vec<(SnapshotHeader, BlobHeader, Vec<u8>)>,
    schema: Arc<Schema>,
    origin: Option<String>,
}

use journal::{AsyncJournal, BlobHeader, SnapshotHeader};
use tokio_stream::wrappers::ReceiverStream;

use crate::{Message, StdError};

#[derive(Debug)]
enum Buf {
    Ok(Message),
    More,
}

impl Mycelite {
    pub fn new(journal_path: impl Into<String>) -> Self {
        let schema = Schema::new(vec![
            Field::new("snapshot_id", DataType::UInt64, false),
            Field::new("timestamp", DataType::Int64, false),
            Field::new("page_size", DataType::UInt32, true),
            Field::new("offset", DataType::UInt64, false),
            Field::new("blob_num", DataType::UInt32, false),
            Field::new("blob_size", DataType::UInt32, false),
            Field::new("blob", DataType::Binary, false),
        ]);
        Self {
            journal_path: journal_path.into(),
            buf: vec![],
            schema: Arc::new(schema),
            cur_snapshot: None,
            origin: None,
        }
    }

    async fn enter_loop<Input, Output, SectionChan>(
        mut self,
        input: Input,
        mut output: Output,
        mut section_chan: SectionChan,
    ) -> Result<(), StdError>
    where
        Input: Stream<Item = Message> + Send + 'static,
        Output: Sink<Message, Error = StdError> + Send,
        SectionChan: SectionChannel + Send + Sync + 'static,
    {
        // section doesn't consume any input
        let _input = input;

        // open async journal
        let mut journal = AsyncJournal::try_from(self.journal_path.as_str()).await?;

        // better than empty string, but might be just configurable
        self.origin = Path::new(self.journal_path.as_str())
            .file_name()
            .unwrap()
            .to_str()
            .map(Into::into);

        // initalize fs watcher to journal
        let (tx, watcher_rx) = tokio::sync::mpsc::channel::<()>(2);
        let mut watcher_rx = pin!(ReceiverStream::new(watcher_rx).fuse());
        tx.send(()).await?;
        let _watcher = self.watch(self.journal_path.as_str(), tx).await?;

        let mut state = section_chan
            .retrieve_state()
            .await?
            .unwrap_or(<SectionChan as SectionChannel>::State::new());
        self.cur_snapshot = state.get::<u64>("snapshot_id")?;

        let mut output = pin!(output);
        loop {
            futures::select! {
                cmd = section_chan.recv().fuse() => {
                    match cmd? {
                        Command::Stop => return Ok(()),
                        Command::Ack(ack) => {
                            match ack.downcast::<u64>() {
                                Ok(snapshot_id) => {
                                    state.set("snapshot_id", *snapshot_id)?;
                                    section_chan.store_state(state.clone()).await?;
                                },
                                Err(_) => Err("failed to downcast ack to u64")?
                            }
                        },
                        _ => (),
                    }
                },
                msg = watcher_rx.next() => {
                    if msg.is_none() {
                        Err("watcher down")?;
                    }
                    // FIXME: journal has no file locks
                    journal.update_header().await?;

                    // FIXME: acks are blocked until stream consumed till the end
                    // i.e. if N snapshots streamed - N messages will be waiting in section_chan Queue
                    // to be acked
                    // FIXME: journal scanned from the start every time
                    // FIXME: snapshots can be big, for now snapshot is sent as a whole message
                    let mut journal_stream = pin!(journal.stream());
                    let cur_snapshot = self.cur_snapshot;
                    while let Some(data) = journal_stream.next().await {
                        let (snapshot_header, blob_header, blob) = data?;
                        if Some(snapshot_header.id) <= cur_snapshot {
                            continue
                        }
                        match self.bufferize(snapshot_header, blob_header, blob, &section_chan)? {
                            Buf::Ok(msg) => output.send(msg).await?,
                            Buf::More => (),
                        }
                    }

                    // flush last message
                    if let Some(msg) = self.build_message(&section_chan)? {
                        output.send(msg).await?
                    }
                },
            }
        }
    }

    fn bufferize<SectionChan: SectionChannel>(
        &mut self,
        snapshot_header: SnapshotHeader,
        blob_header: BlobHeader,
        blob: Vec<u8>,
        section_chan: &SectionChan,
    ) -> Result<Buf, StdError> {
        match self.cur_snapshot != Some(snapshot_header.id) {
            true => {
                self.cur_snapshot = Some(snapshot_header.id);
                let res = self
                    .build_message(section_chan)?
                    .map(Buf::Ok)
                    .unwrap_or(Buf::More);
                self.buf.push((snapshot_header, blob_header, blob));
                Ok(res)
            }
            false => {
                self.buf.push((snapshot_header, blob_header, blob));
                Ok(Buf::More)
            }
        }
    }

    fn build_message<SectionChan: SectionChannel>(
        &mut self,
        section_chan: &SectionChan,
    ) -> Result<Option<Message>, StdError> {
        if self.buf.is_empty() {
            return Ok(None);
        }
        let columns: Vec<Arc<dyn Array>> = vec![
            Arc::new(
                self.buf
                    .iter()
                    .map(|(s, _, _)| s.id)
                    .collect::<UInt64Array>(),
            ),
            Arc::new(
                self.buf
                    .iter()
                    .map(|(s, _, _)| s.timestamp)
                    .collect::<Int64Array>(),
            ),
            Arc::new(
                self.buf
                    .iter()
                    .map(|(s, _, _)| s.page_size)
                    .collect::<UInt32Array>(),
            ),
            Arc::new(
                self.buf
                    .iter()
                    .map(|(_, b, _)| b.offset)
                    .collect::<UInt64Array>(),
            ),
            Arc::new(
                self.buf
                    .iter()
                    .map(|(_, b, _)| b.blob_num)
                    .collect::<UInt32Array>(),
            ),
            Arc::new(
                self.buf
                    .iter()
                    .map(|(_, b, _)| b.blob_size)
                    .collect::<UInt32Array>(),
            ),
            Arc::new(
                self.buf
                    .iter()
                    .map(|(_, _, d)| Some(d.as_slice()))
                    .collect::<BinaryArray>(),
            ),
        ];
        let snapshot_id = self
            .buf
            .iter()
            .take(1)
            .map(|(s, _, _)| s.id)
            .next()
            .unwrap();
        self.buf = vec![];
        let record_batch = RecordBatch::try_new(Arc::clone(&self.schema), columns)?;
        let weak_chan = section_chan.weak_chan();
        let ack = Box::pin(async move { weak_chan.ack(Box::new(snapshot_id)).await });
        Ok(Some(Message::new(
            self.origin.as_deref().unwrap_or(""),
            record_batch,
            Some(ack),
        )))
    }

    // initiate first check on startup
    async fn watch(&self, path: &str, tx: Sender<()>) -> notify::Result<impl Watcher> {
        tx.send(()).await.ok();
        let mut watcher = notify::recommended_watcher(move |res: Result<Event, _>| match res {
            Ok(event) if event.kind.is_modify() || event.kind.is_create() => {
                tx.blocking_send(()).ok();
            }
            Ok(_) => (),
            Err(_e) => (),
        })?;
        watcher.watch(Path::new(path), RecursiveMode::NonRecursive)?;
        Ok(watcher)
    }
}

impl<Input, Output, SectionChan> Section<Input, Output, SectionChan> for Mycelite
where
    Input: Stream<Item = Message> + Send + 'static,
    Output: Sink<Message, Error = StdError> + Send + 'static,
    SectionChan: SectionChannel + Send + Sync + 'static,
{
    type Error = Box<dyn std::error::Error + Send + Sync + 'static>;
    type Future = Pin<Box<dyn Future<Output = Result<(), Self::Error>> + Send + 'static>>;

    fn start(self, input: Input, output: Output, section_chan: SectionChan) -> Self::Future {
        Box::pin(async move { self.enter_loop(input, output, section_chan).await })
    }
}

//! Mycelite journal destination

use std::collections::HashSet;
use std::path::Path;
use std::io::SeekFrom;
use std::pin::{pin, Pin};
use std::future::Future;
use arrow::array::{UInt64Array, BinaryArray, Int64Array, UInt32Array};
use arrow::datatypes::{Schema, Field, DataType};
use futures::{Sink, Stream, StreamExt, FutureExt};

#[derive(Debug)]
pub struct Mycelite {
    journal_path: String,
    database_path: Option<String>,
    schema: Schema,
}

use journal::{AsyncJournal, SnapshotHeader, BlobHeader};
use section::{SectionChannel, Section, Command};
use tokio::io::{AsyncSeekExt, AsyncWriteExt};

use crate::{Message, StdError};

impl Mycelite {
    pub fn new(journal_path: impl Into<String>, database_path: Option<impl Into<String>>) -> Self {
        let schema = Schema::new(
            vec![
                Field::new("snapshot_id", DataType::UInt64, false),
                Field::new("timestamp", DataType::Int64, false),
                Field::new("page_size", DataType::UInt32, true),
                Field::new("offset", DataType::UInt64, false),
                Field::new("blob_num", DataType::UInt32, false),
                Field::new("blob_size", DataType::UInt32, false),
                Field::new("blob", DataType::Binary, false),
            ]
        );
        Self {
            journal_path: journal_path.into(),
            database_path: database_path.map(Into::into),
            schema,
        }
    }

    async fn enter_loop<Input, Output, SectionChan> (
        self,
        input: Input,
        output: Output,
        mut section_chan: SectionChan,
    ) -> Result<(), StdError>
        where Input: Stream<Item=Message> + Send + 'static,
              Output: Sink<Message, Error=StdError> + Send,
              SectionChan: SectionChannel + Send + Sync + 'static,
    {
        let mut db_fd = match self.database_path.as_ref() {
            Some(path) =>
                Some(tokio::fs::OpenOptions::new()
                    .create(true)
                    .write(true)
                    .open(Path::new(path)).await?),
            None => None,
        };

        // section doesn't produce any output
        let _output = output;
        let mut input = pin!(input.fuse());

        // open async journal
        let mut journal = match AsyncJournal::try_from(self.journal_path.as_str()).await {
            Ok(j) => Ok(j),
            Err(e) if e.journal_not_exists() => AsyncJournal::create(self.journal_path.as_str()).await,
            Err(e) => Err(e),
        }?;

        loop {
            futures::select! {
                cmd = section_chan.recv().fuse() => {
                    if let Command::Stop = cmd? { return Ok(()) };
                },
                msg = input.next() => {
                    if msg.is_none() {
                        Err("input closed")?
                    }
                    let mut msg = msg.unwrap();

                    if msg.payload.schema().as_ref() != &self.schema {
                        Err("unexpected schema message")?
                    }

                    journal.update_header().await?;
                    let header = *journal.get_header();

                    let payload = &msg.payload;
                    let len = payload.num_rows();
                    let snapshot_id: UInt64Array = payload["snapshot_id"].to_data().into();
                    let timestamp: Int64Array = payload["timestamp"].to_data().into();
                    let page_size: UInt32Array = payload["page_size"].to_data().into();
                    let offset: UInt64Array = payload["offset"].to_data().into();
                    let blob_num: UInt32Array = payload["blob_num"].to_data().into();
                    let blob_size: UInt32Array = payload["blob_size"].to_data().into();
                    let blob: BinaryArray = payload["blob"].to_data().into();

                    let snapshot_id = HashSet::<u64>::from_iter(snapshot_id.iter().map(|x| x.unwrap()));
                    if snapshot_id.len() != 1 {
                        Err("arrow message with multiple snapshots are not supported yet")?
                    };
                    let snapshot_id = snapshot_id.into_iter().next().unwrap();

                    // if snapshot id already exists - skip
                    if snapshot_id + 1 > header.snapshot_counter {
                        let timestamp = timestamp.value(0);
                        let page_size = page_size.value(0);
                        let snapshot_header = SnapshotHeader::new(snapshot_id, timestamp, Some(page_size));

                        journal.add_snapshot(&snapshot_header).await?;
                        for pos in 0..len {
                            let blob_h = BlobHeader::new(offset.value(pos), blob_num.value(pos), blob_size.value(pos));
                            let blob = blob.value(pos);
                            journal.add_blob(&blob_h, blob).await?;
                        }
                        journal.commit().await?;
                    }
                    msg.ack().await;
                    
                    // FIXME: db recovered from scratch
                    let mut journal_stream = pin!(journal.stream());
                    if let Some(ref mut fd) = db_fd {
                        while let Some(data) = journal_stream.next().await {
                            let (_, blob_h, blob) = data?;
                            fd.seek(SeekFrom::Start(blob_h.offset)).await?;
                            fd.write_all(&blob).await?;
                        }
                        fd.flush().await?;
                    }
                }
            }
        }
    }
}

impl<Input, Output, SectionChan> Section <Input, Output, SectionChan> for Mycelite
    where Input: Stream<Item=Message> + Send + 'static,
          Output: Sink<Message, Error=StdError> + Send + 'static,
          SectionChan: SectionChannel + Send + Sync + 'static,
{

    type Error = Box<dyn std::error::Error + Send + Sync + 'static>;
    type Future = Pin<Box<dyn Future<Output = Result<(), Self::Error>> + Send + 'static>>;

    fn start(self, input: Input, output: Output, section_chan: SectionChan) -> Self::Future {
        Box::pin(async move { self.enter_loop(input, output, section_chan).await })
    }
}

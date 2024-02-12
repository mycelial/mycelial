use crate::{SqliteColumn, SqliteMessage, SqlitePayload};
use notify::{Event, RecursiveMode, Watcher};
use section::{
    command_channel::{Command, SectionChannel, WeakSectionChannel},
    futures::{self, FutureExt, Sink, SinkExt, Stream, StreamExt},
    message::{Chunk, DataType, Value},
    section::Section,
    state::State,
    SectionError, SectionFuture, SectionMessage,
};
use sqlx::{
    sqlite::{SqliteConnectOptions, SqliteRow},
    Column as _, ConnectOptions, Row, TypeInfo, ValueRef,
};

// FIXME: drop direct dependency
use tokio::sync::mpsc::Sender;
use tokio_stream::wrappers::ReceiverStream;

use std::path::Path;
use std::sync::Arc;
use std::{pin::pin, str::FromStr};

#[derive(Debug)]
pub struct Sqlite {
    path: String,
    origin: Arc<str>,
    query: String,
}

const LAST_MTIME: &str = "last_mtime";

impl Sqlite {
    pub fn new(path: impl Into<String>, origin: impl AsRef<str>, query: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            origin: Arc::from(origin.as_ref()),
            query: query.into(),
        }
    }

    async fn enter_loop<Input, Output, SectionChan>(
        self,
        input: Input,
        output: Output,
        mut section_channel: SectionChan,
    ) -> Result<(), SectionError>
    where
        Input: Stream + Send,
        Output: Sink<SectionMessage, Error = SectionError> + Send,
        SectionChan: SectionChannel + Send + Sync,
    {
        let mut connection = SqliteConnectOptions::from_str(self.path.as_str())?
            .create_if_missing(false)
            .connect()
            .await?;

        let (tx, rx) = tokio::sync::mpsc::channel(1);
        tx.send(()).await.ok();

        let _watcher = self.watch_sqlite_paths(self.path.as_str(), tx)?;

        let mut _input = pin!(input.fuse());
        let mut output = pin!(output);
        let mut state = section_channel
            .retrieve_state()
            .await?
            .unwrap_or(SectionChan::State::new());
        let mut last_mtime = state.get(LAST_MTIME)?.unwrap_or(0);

        let rx = ReceiverStream::new(rx);
        let mut rx = pin!(rx.fuse());
        loop {
            futures::select! {
                cmd = section_channel.recv().fuse() => {
                    match cmd? {
                        Command::Ack(any) => {
                            match any.downcast::<i64>() {
                                Ok(ack) => {
                                    state.set(LAST_MTIME, *ack)?;
                                    section_channel.store_state(state.clone()).await?;
                                },
                                Err(_) =>
                                    Err("Failed to downcast incoming Ack message to Message")?,
                            };
                        },
                        Command::Stop => return Ok(()),
                        _ => {},
                    }
                },
                msg = rx.next() => {
                    if msg.is_none() {
                        Err("file watcher exited")?
                    };
                    let mtime = self.get_mtime().await?;
                    if mtime <= last_mtime {
                        continue
                    }
                    last_mtime = mtime;
                    let mut row_stream = sqlx::query(self.query.as_str())
                        .fetch(&mut connection);

                    // check if row stream contains any result
                    let mut row_stream = match row_stream.next().await {
                        Some(res) => futures::stream::once(async { res }).chain(row_stream),
                        None => continue
                    };

                    let mut row_stream = pin!(row_stream);
                    let weak_chan = section_channel.weak_chan();
                    let ack = Box::pin(async move {
                        weak_chan.ack(Box::new(last_mtime)).await;
                    });

                    let mut buf = Vec::with_capacity(256);
                    let (mut tx, rx) = tokio::sync::mpsc::channel(1);
                    let message = SqliteMessage::new(
                        Arc::clone(&self.origin),
                        rx,
                        Some(ack)
                    );
                    output.send(Box::new(message)).await.map_err(|_| "failed to send data to sink")?;

                    'stream: loop {
                        if buf.len() == buf.capacity() {
                            self.send_chunk(&mut tx, &mut buf).await?;
                        }
                        match row_stream.next().await {
                            Some(Ok(row)) => buf.push(row),
                            Some(Err(e)) => {
                                tx.send(Err("error".into())).await.ok();
                                Err(e)?
                            },
                            None => {
                                self.send_chunk(&mut tx, &mut buf).await?;
                                break 'stream;
                            },
                        }
                    }
                },
            }
        }
    }

    async fn send_chunk(
        &self,
        tx: &mut Sender<Result<Chunk, SectionError>>,
        buf: &mut Vec<SqliteRow>,
    ) -> Result<(), SectionError> {
        if !buf.is_empty() {
            let chunk = Chunk::DataFrame(Box::new(self.build_sqlite_payload(buf.as_slice())?));
            tx.send(Ok(chunk)).await.map_err(|_| "send error")?;
            buf.truncate(0);
        }
        Ok(())
    }

    async fn get_mtime(&self) -> Result<i64, SectionError> {
        let metadata = tokio::fs::metadata(&self.path).await?;
        let mtime = metadata
            .modified()?
            .duration_since(std::time::UNIX_EPOCH)?
            .as_micros() as i64;
        Ok(mtime)
    }

    fn build_sqlite_payload(&self, rows: &[SqliteRow]) -> Result<SqlitePayload, SectionError> {
        let columns = match rows.first() {
            None => Err("empty rows")?,
            Some(row) => row
                .columns()
                .iter()
                .map(|col| SqliteColumn {
                    name: col.name().into(),
                    data_type: DataType::Any,
                })
                .collect::<Vec<_>>(),
        };
        let mut values = vec![Vec::<Value>::with_capacity(rows.len()); columns.len()];
        for row in rows.iter() {
            for column in row.columns() {
                let pos = column.ordinal();
                let raw_value = row.try_get_raw(pos)?;
                let value = match raw_value.type_info().name() {
                    "TEXT" => row.get::<Option<String>, _>(pos).map(Value::from),
                    "REAL" => row.get::<Option<f64>, _>(pos).map(Value::F64),
                    "INTEGER" => row.get::<Option<i64>, _>(pos).map(Value::I64),
                    "BLOB" => row.get::<Option<Vec<u8>>, _>(pos).map(Value::from),
                    _ => unreachable!(),
                }
                .unwrap_or(Value::Null);
                values[pos].push(value);
            }
        }
        Ok(SqlitePayload { columns, values })
    }

    /// Watch sqlite database file and sqlite WAL (if present) for changes
    /// Notify main loop when change occurs
    fn watch_sqlite_paths(
        &self,
        sqlite_path: &str,
        tx: Sender<()>,
    ) -> notify::Result<impl Watcher> {
        let mut watcher = notify::recommended_watcher(move |res: Result<Event, _>| match res {
            Ok(event) if event.kind.is_modify() || event.kind.is_create() => {
                tx.blocking_send(()).ok();
            }
            Ok(_) => (),
            Err(_e) => (),
        })?;
        // watch both sqlite and wal file
        let _ = &[
            Path::new(sqlite_path),
            Path::new(&format!("{}-wal", sqlite_path)),
        ]
        .into_iter()
        // FIXME: files may not be present
        .filter(|p| p.exists())
        .try_for_each(|path| watcher.watch(path, RecursiveMode::NonRecursive))?;
        Ok(watcher)
    }
}

impl<Input, Output, SectionChan> Section<Input, Output, SectionChan> for Sqlite
where
    Input: Stream + Send + 'static,
    Output: Sink<SectionMessage, Error = SectionError> + Send + 'static,
    SectionChan: SectionChannel + Send + Sync + 'static,
{
    type Error = SectionError;
    type Future = SectionFuture;

    fn start(self, input: Input, output: Output, command: SectionChan) -> Self::Future {
        Box::pin(async move { self.enter_loop(input, output, command).await })
    }
}

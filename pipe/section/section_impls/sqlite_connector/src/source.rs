//! Sqlite section for query-based CDC
//!
//! Query-based CDC implementation, which uses `notify` crate to detect changes to database.  
//! Sqlite-CDC section assumes append-only tables, any other usecase is not supported (yet?).
//!
//! # Configuration
//! Path to sqlite and list of tables to observe.
//! Section will automatically initialize underlying table state on initialization:
//! - column names
//! - column types
//! - query
//! - limit and offset
use crate::{SqlitePayload, Table, TableColumn, SqliteMessage};
use notify::{Event, RecursiveMode, Watcher};
use section::{
    command_channel::{SectionChannel, WeakSectionChannel, Command},
    message::{DataType, Value},
    section::Section,
    state::State,
    SectionError, SectionFuture, SectionMessage,
    futures::{self, Sink, Stream, StreamExt, FutureExt, SinkExt},
};
use sqlx::{
    sqlite::{SqliteConnectOptions, SqliteRow},
    Column as _, ConnectOptions, Row, SqliteConnection, TypeInfo, ValueRef,
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
    tables: Vec<String>,
}

async fn get_table_list(conn: &mut SqliteConnection) -> Result<Vec<(String, bool)>, SectionError> {
    let tables_list = sqlx::query("PRAGMA table_list")
        .map(|row: SqliteRow| {
            let schema = row.get::<String, _>(0);
            let name = row.get::<String, _>(1);
            let ttype = row.get::<String, _>(2);
            let strict = row.get::<i64, _>(5) != 0;
            (schema, name, ttype, strict)
        })
        .fetch_all(&mut *conn)
        .await?
        .into_iter()
        .filter_map(|(schema, name, ttype, strict)| {
            match (schema.as_str(), name.as_str(), ttype.as_str()) {
                ("main", name_str, "table") if !name_str.starts_with("sqlite_") => {
                    Some((name, strict))
                }
                _ => None,
            }
        })
        .collect();
    Ok(tables_list)
}

async fn describe_table<S: State>(
    conn: &mut SqliteConnection,
    name: String,
    strict: bool,
    state: &S,
) -> Result<Table, SectionError> {
    // NOTE: table_info doesn't show hidden or generated columns
    //       xtable_info can be used instead
    let columns = sqlx::query(format!("PRAGMA table_info({name})").as_str())
        .map(|row: SqliteRow| {
            let name = row.get::<String, _>(1);
            let data_type = match (strict, row.get::<String, _>(2).as_str()) {
                (false, _) => DataType::Any,
                (_, "INT") => DataType::I64,
                (_, "INTEGER") => DataType::I64,
                (_, "REAL") => DataType::F64,
                (_, "TEXT") => DataType::Str,
                (_, "BLOB") => DataType::Bin,
                (_, "ANY") => DataType::Any,
                (_, u) => unimplemented!("unsupported column data type: {}", u),
            };
            let nullable = row.get::<i64, _>(3) == 0;
            TableColumn {
                name: name.into(),
                data_type,
                nullable,
            }
        })
        .fetch_all(conn)
        .await?;
    if columns.is_empty() {
        unreachable!()
    }
    let query = format!(
        "SELECT {} FROM {} limit ? offset ?",
        columns
            .iter()
            .map(|col| col.name.as_ref())
            .collect::<Vec<_>>()
            .join(", "),
        name
    );
    let offset = state.get::<i64>(&name)?.unwrap_or(0);
    Ok(Table {
        name: name.into(),
        strict,
        columns: columns.into(),
        query,
        offset,
        limit: 2048,
    })
}

async fn get_tables<S: State>(
    conn: &mut SqliteConnection,
    state: &S,
) -> Result<Vec<Table>, SectionError> {
    let mut tables = vec![];
    for (table_name, strict) in get_table_list(conn).await? {
        tables.push(describe_table(conn, table_name, strict, state).await?);
    }
    Ok(tables)
}

impl Sqlite {
    pub fn new(path: impl Into<String>, tables: &[&str]) -> Self {
        Self {
            path: path.into(),
            tables: tables.iter().map(|&x| x.into()).collect(),
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

        let (tx, rx) = tokio::sync::mpsc::channel(4);

        // keep weak ref to be able to send messages to ourselfs
        let weak_tx = tx.clone().downgrade();
        tx.send(()).await.ok();

        let _watcher = self.watch_sqlite_paths(self.path.as_str(), tx);

        let mut _input = pin!(input.fuse());
        let mut output = pin!(output);
        let mut state = section_channel
            .retrieve_state()
            .await?
            .unwrap_or(SectionChan::State::new());

        let mut tables = get_tables(&mut connection, &state).await?;

        let rx = ReceiverStream::new(rx);
        let mut rx = pin!(rx.fuse());
        loop {
            futures::select_biased! {
                cmd = section_channel.recv().fuse() => {
                    match cmd? {
                        Command::Ack(any) => {
                            match any.downcast::<AckMessage>() {
                                Ok(ack) => {
                                    state.set(&ack.table, ack.offset)?;
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

                    let mut empty_count = 0;
                    for table in tables.iter_mut() {
                        let rows = sqlx::query(&table.query)
                            .bind(table.limit)
                            .bind(table.offset)
                            .fetch_all(&mut connection)
                            .await?;

                        if rows.is_empty() {
                            empty_count += 1;
                            continue
                        }
                        table.offset += rows.len() as i64;

                        let sqlite_payload = self.build_sqlite_payload(table, rows)?;
                        let weak_chan = section_channel.weak_chan();

                        let ack_msg = AckMessage{ table: Arc::clone(&table.name), offset: table.offset };
                        let ack = Box::pin(async move { weak_chan.ack(Box::new(ack_msg)).await; });
                        let message = Box::new(SqliteMessage::new(Arc::clone(&table.name), sqlite_payload, Some(ack)));
                        output.send(message).await.map_err(|_| "failed to send data to sink")?;
                    }
                    // if empty count is less than table count - we didn't reach ends of table on
                    // whole dataset
                    if let Some(tx) = weak_tx.clone().upgrade() {
                        if empty_count < self.tables.len() {
                            tx.try_send(()).ok();
                        }
                    }
                },
            }
        }
    }

    fn build_sqlite_payload(
        &self,
        table: &Table,
        rows: Vec<SqliteRow>,
    ) -> Result<SqlitePayload, SectionError> {
        let mut values: Vec<Vec<Value>> = (0..table.columns.len())
            .map(|_| Vec::with_capacity(rows.len()))
            .collect();
        for row in rows.iter() {
            for column in row.columns() {
                let pos = column.ordinal();
                let raw_value = row.try_get_raw(pos)?;
                let value = match raw_value.type_info().name() {
                    "TEXT" => row.get::<Option<String>, _>(pos).map(Value::Str),
                    "REAL" => row.get::<Option<f64>, _>(pos).map(Value::F64),
                    "INTEGER" => row.get::<Option<i64>, _>(pos).map(Value::I64),
                    "BLOB" => row.get::<Option<Vec<u8>>, _>(pos).map(Value::Bin),
                    _ => unreachable!(),
                }
                .unwrap_or(Value::Null);
                values[pos].push(value);
            }
        }
        // FIXME: use Arc
        let batch = SqlitePayload {
            columns: Arc::clone(&table.columns),
            values,
        };
        Ok(batch)
    }

    /// Watch sqlite database file and sqlite WAL (if present) for changes
    /// Notify main loop when change occurs
    fn watch_sqlite_paths(
        &self,
        sqlite_path: &str,
        tx: Sender<()>,
    ) -> notify::Result<impl Watcher> {
        // initiate first check on startup
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
        .filter(|path| path.exists())
        .try_for_each(|path| watcher.watch(path, RecursiveMode::NonRecursive))?;
        Ok(watcher)
    }
}

struct AckMessage {
    table: Arc<str>,
    offset: i64,
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

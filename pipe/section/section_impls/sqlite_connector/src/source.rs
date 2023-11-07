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
//!
//! # Known issues
//! 1. Sqlite is not strict when running by default, any column can contain any datatype, which
//!    this implementation can't handle properly
//! 2. Column names and types are derived at start, if table was changed during runtime - section will error out.

use crate::{escape_table_name, ColumnType, Message, SqlitePayload, StdError, Value};
use fallible_iterator::FallibleIterator;
use futures::{FutureExt, Sink, SinkExt, Stream, StreamExt};
use notify::{Event, RecursiveMode, Watcher};
use section::{Command, Section, SectionChannel, State, WeakSectionChannel};
use sqlite3_parser::{
    ast::{Cmd, CreateTableBody, Stmt, Type},
    lexer::sql::Parser,
};
use sqlx::{
    sqlite::{SqliteConnectOptions, SqliteRow},
    ConnectOptions, Row, SqliteConnection, ValueRef,
};

// FIXME: drop direct dependency
use tokio::sync::mpsc::Sender;
use tokio_stream::wrappers::ReceiverStream;

use std::path::Path;
use std::{future::Future, sync::Arc};
use std::{
    pin::{pin, Pin},
    str::FromStr,
};

#[derive(Debug)]
pub struct Sqlite {
    path: String,
    tables: Vec<String>,
    once: bool,
    strict: bool,
}

impl TryFrom<(ColumnType, usize, &SqliteRow, bool)> for Value {
    // FIXME: specific error instead of Box<dyn Error>
    type Error = StdError;

    fn try_from(
        (col, index, row, strict): (ColumnType, usize, &SqliteRow, bool),
    ) -> Result<Self, Self::Error> {
        // FIXME: handle sqlite type affinities properly:
        // sqlite> create table t(id);
        // sqlite> insert into t values(1), ('1'), (NULL), ("string");
        // sqlite> select typeof(id) from t;
        // +------------+
        // | typeof(id) |
        // +------------+
        // | integer    |
        // | text       |
        // | null       |
        // | text       |
        // +------------+
        // obvisouly such table will break following code
        //
        // more info: https://www.sqlite.org/datatype3.html
        if strict {
            let value = match col {
                ColumnType::Int => row.try_get::<Option<i64>, _>(index)?.map(Value::Int),
                ColumnType::Text => row.try_get::<Option<String>, _>(index)?.map(Value::Text),
                ColumnType::Blob => row.try_get::<Option<Vec<u8>>, _>(index)?.map(Value::Blob),
                ColumnType::Real => row.try_get::<Option<f64>, _>(index)?.map(Value::Real),
                _ => return Err(format!("unimplemented: {:?}", col).into()),
            }
            .unwrap_or(Value::Null);
            return Ok(value);
        } else {
            let v = row.try_get::<Option<i64>, _>(index);
            match v {
                Ok(v) => match v {
                    Some(v) => return Ok(Value::Int(v)),
                    None => return Ok(Value::Null),
                },
                Err(_e) => {}
            };
            let v = row.try_get::<Option<String>, _>(index);
            match v {
                Ok(v) => return Ok(Value::Text(v.unwrap())),
                Err(_e) => {}
            };
            let v = row.try_get::<Option<Vec<u8>>, _>(index);
            match v {
                Ok(v) => return Ok(Value::Blob(v.unwrap())),
                Err(_e) => {}
            };
            let v = row.try_get::<Option<f64>, _>(index);
            match v {
                Ok(v) => return Ok(Value::Real(v.unwrap())),
                Err(_e) => {}
            };
            return Err(format!("unimplemented: {:?}", col).into());
        }
    }
}

#[derive(Debug)]
pub struct Table {
    pub name: Arc<str>,
    pub columns: Arc<[String]>,
    pub column_types: Arc<[ColumnType]>,
    pub query: String,
    pub offset: i64,
    pub limit: i64,
}

#[derive(Debug)]
enum InnerEvent {
    NewChange,
    Stream,
    StreamEnd,
}

impl Sqlite {
    pub fn new(path: impl Into<String>, tables: &[&str], once: bool, strict: bool) -> Self {
        Self {
            path: path.into(),
            tables: tables.iter().map(|&x| x.into()).collect(),
            once,
            strict,
        }
    }

    async fn enter_loop<Input, Output, SectionChan>(
        self,
        input: Input,
        output: Output,
        mut section_channel: SectionChan,
    ) -> Result<(), StdError>
    where
        Input: Stream + Send,
        Output: Sink<Message, Error = StdError> + Send,
        SectionChan: SectionChannel + Send + Sync,
    {
        let mut connection = SqliteConnectOptions::from_str(self.path.as_str())?
            .create_if_missing(false)
            .connect()
            .await?;

        let (tx, rx) = tokio::sync::mpsc::channel(4);
        tx.send(InnerEvent::NewChange).await?;

        // keep weak ref to be able to send messages to ourselfs
        let weak_tx = tx.clone().downgrade();

        let _watcher = self.watch_sqlite_paths(self.path.as_str(), tx);

        let mut _input = pin!(input.fuse());
        let mut output = pin!(output);
        let mut state = section_channel
            .retrieve_state()
            .await?
            .unwrap_or(<<SectionChan as SectionChannel>::State>::new());

        let mut tables = self
            .init_tables::<SectionChan>(&mut connection, &state, self.strict)
            .await?;

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
                    match msg {
                        Some(InnerEvent::StreamEnd) if self.once => return Ok(()),
                        Some(_) => {},
                        None => Err("sqlite file watched exited")?
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
                            break
                        }
                        table.offset += rows.len() as i64;

                        let sqlite_payload = self.build_sqlite_payload(table, rows, self.strict)?;
                        let weak_chan = section_channel.weak_chan();
                        let ack_message = Box::new(AckMessage {
                            table: Arc::clone(&table.name),
                            offset: table.offset,
                        });
                        let message = Message::new(table.name.to_string(), sqlite_payload, Some(Box::pin(async move {
                            weak_chan.ack(ack_message).await;
                        })));
                        output.send(message).await.map_err(|_| "failed to send data to sink")?;
                    }
                    // if empty count is less than table count - we didn't reach ends of table on
                    // whole dataset
                    if let Some(tx) = weak_tx.clone().upgrade() {
                        let event = match empty_count < self.tables.len() {
                            true => InnerEvent::Stream,
                            false => InnerEvent::StreamEnd,
                        };
                        tx.try_send(event).ok();
                    }
                },
            }
        }
    }

    // init tables by filling column names/types and generate query for provided table names
    //
    // function will pull table definition from `sqlite_master` table, parse it with aka sqlite3_parser to pull
    // column names/column types out
    async fn init_tables<C: SectionChannel>(
        &self,
        connection: &mut SqliteConnection,
        state: &<C as SectionChannel>::State,
        strict: bool,
    ) -> Result<Vec<Table>, StdError> {
        let mut tables = Vec::with_capacity(self.tables.len());

        let all_tables: Vec<String>;
        let table_names = match self.tables.iter().any(|table| table == "*") {
            true => {
                all_tables = sqlx::query("SELECT name FROM sqlite_master WHERE type='table'")
                    .map(|row: SqliteRow| row.get::<String, _>(0))
                    .fetch_all(&mut *connection)
                    .await?;
                all_tables.as_slice()
            }
            false => self.tables.as_slice(),
        };

        for table in table_names.iter() {
            let name = escape_table_name(table);
            let query = format!("SELECT * FROM \"{name}\" limit ? offset ?");
            let table_sql = sqlx::query("SELECT sql FROM sqlite_master WHERE name = ?")
                .bind(&name)
                .fetch_one(&mut *connection)
                .await?
                .try_get::<String, _>(0)?;
            let mut parser = Parser::new(table_sql.as_bytes());
            let columns = match parser.next()? {
                Some(Cmd::Stmt(Stmt::CreateTable {
                    body: CreateTableBody::ColumnsAndConstraints { columns, .. },
                    ..
                })) => columns,
                other => Err(format!("unexpected parser response: {:?}", other))?,
            };
            let mut cols = Vec::with_capacity(columns.len());
            let mut col_types = Vec::with_capacity(columns.len());
            let t = &Type {
                name: "none".to_string(),
                size: None,
            };
            for column in columns {
                let col_name = column
                    .col_name
                    .to_string()
                    .trim_end_matches('"')
                    .trim_start_matches('"')
                    .to_string();
                cols.push(col_name);
                let ty = match column.col_type {
                    Some(ref ty) => ty,
                    None => t,
                };
                // FIXME: Numeric type as a wildcard match
                let ty = match &ty.name.to_lowercase() {
                    ty if ty.contains("int") => ColumnType::Int,
                    ty if ty.contains("text")
                        || ty.contains("char")
                        || ty.contains("text")
                        || ty.contains("clob") =>
                    {
                        ColumnType::Text
                    }
                    ty if ty.contains("blob") => ColumnType::Blob,
                    ty if ty.contains("real") || ty.contains("double") || ty.contains("real") => {
                        ColumnType::Real
                    }
                    _ => ColumnType::Any,
                };
                if strict {
                    col_types.push(ty);
                } else {
                    col_types.push(ColumnType::Any);
                }
            }
            let offset = state.get::<i64>(&name)?.unwrap_or(0);
            let table = Table {
                name: Arc::from(name),
                columns: Arc::from(cols),
                column_types: Arc::from(col_types),
                query,
                limit: 2500,
                offset,
            };
            tables.push(table);
        }
        Ok(tables)
    }

    fn build_sqlite_payload(
        &self,
        table: &Table,
        rows: Vec<SqliteRow>,
        strict: bool,
    ) -> Result<SqlitePayload, StdError> {
        let mut values: Vec<Vec<Value>> = vec![];
        for row in rows.iter() {
            if values.len() != row.columns().len() {
                values = row
                    .columns()
                    .iter()
                    .map(|_| Vec::with_capacity(rows.len()))
                    .collect();
            }
            for (index, &column) in table.column_types.iter().enumerate() {
                let value = Value::try_from((column, index, row, strict))?;
                values[index].push(value);
            }
        }
        let batch = SqlitePayload {
            columns: Arc::clone(&table.columns),
            column_types: Arc::clone(&table.column_types),
            values,
            offset: table.offset,
        };
        Ok(batch)
    }

    /// Watch sqlite database file and sqlite WAL (if present) for changes
    /// Notify main loop when change occurs
    fn watch_sqlite_paths(
        &self,
        sqlite_path: &str,
        tx: Sender<InnerEvent>,
    ) -> notify::Result<impl Watcher> {
        // initiate first check on startup
        let mut watcher = notify::recommended_watcher(move |res: Result<Event, _>| match res {
            Ok(event) if event.kind.is_modify() || event.kind.is_create() => {
                tx.blocking_send(InnerEvent::NewChange).ok();
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
    Output: Sink<Message, Error = StdError> + Send + 'static,
    SectionChan: SectionChannel + Send + Sync + 'static,
{
    type Error = StdError;
    type Future = Pin<Box<dyn Future<Output = Result<(), Self::Error>> + Send + 'static>>;

    fn start(self, input: Input, output: Output, command: SectionChan) -> Self::Future {
        Box::pin(async move { self.enter_loop(input, output, command).await })
    }
}

pub fn new(path: impl Into<String>, tables: &[&str], once: bool, strict: bool) -> Sqlite {
    Sqlite::new(path, tables, once, strict)
}

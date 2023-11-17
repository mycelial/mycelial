use crate::{ColumnType, Message, PostgresPayload, StdError, Value};
use futures::{FutureExt, Sink, SinkExt, Stream, StreamExt};
use section::{Command, Section, SectionChannel, State};
use sqlx::{
    postgres::{PgConnectOptions, PgConnection, PgRow},
    ConnectOptions, Row,
};

use std::time::Duration;
use std::{future::Future, sync::Arc};
use std::{
    pin::{pin, Pin},
    str::FromStr,
};

#[derive(Debug)]
pub struct Postgres {
    url: String,
    schema: String,
    tables: Vec<String>,
    poll_interval: Duration,
}

impl TryFrom<(ColumnType, usize, &PgRow)> for Value {
    type Error = StdError;

    fn try_from((col, index, row): (ColumnType, usize, &PgRow)) -> Result<Self, Self::Error> {
        let value = match col {
            ColumnType::I16 => Value::I16(row.get::<i16, _>(index)),
            ColumnType::I32 => Value::I32(row.get::<i32, _>(index)),
            ColumnType::I64 => Value::I64(row.get::<i64, _>(index)),
            ColumnType::F32 => Value::F32(row.get::<f32, _>(index)),
            ColumnType::F64 => Value::F64(row.get::<f64, _>(index)),
            ColumnType::Blob => Value::Blob(row.get::<Vec<u8>, _>(index)),
            ColumnType::Text => Value::Text(row.get::<String, _>(index)),
            ColumnType::Bool => Value::Bool(row.get::<bool, _>(index)),
            _ => Err(format!("unsupported type {}", col))?,
        };
        Ok(value)
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

impl Postgres {
    pub fn new(
        url: impl Into<String>,
        schema: impl Into<String>,
        tables: &[&str],
        poll_interval: Duration,
    ) -> Self {
        Self {
            url: url.into(),
            schema: schema.into(),
            tables: tables.iter().map(|&x| x.into()).collect(),
            poll_interval,
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
        let mut connection = PgConnectOptions::from_str(self.url.as_str())?
            .connect()
            .await?;

        let mut _input = pin!(input.fuse());
        let mut output = pin!(output);
        let mut state = section_channel
            .retrieve_state()
            .await?
            .unwrap_or(<<SectionChan as SectionChannel>::State>::new());

        let mut tables = self
            .init_tables::<SectionChan>(&mut connection, &state)
            .await?;

        let mut interval = tokio::time::interval(self.poll_interval);
        loop {
            futures::select! {
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
                _ = interval.tick().fuse() => {
                    for table in tables.iter_mut() {
                        // FIXME: full table selected
                        let rows = sqlx::query(table.query.as_str())
                          //.bind(table.limit)
                          //.bind(table.offset)
                            .fetch_all(&mut connection)
                            .await?;

                        if rows.is_empty() {
                            continue
                        }

                        let payload = self.build_payload(table, rows)?;
                        let message = Message::new(table.name.as_ref().to_string(), payload, None);
                        output.send(message).await?;
                    }
                },
            }
        }
    }

    async fn init_tables<C: SectionChannel>(
        &self,
        connection: &mut PgConnection,
        _state: &<C as SectionChannel>::State,
    ) -> Result<Vec<Table>, StdError> {
        let all = self.tables.iter().any(|t| t == "*");
        let names =
            sqlx::query("SELECT table_name FROM information_schema.tables WHERE table_schema=$1")
                .bind(self.schema.as_str())
                .fetch_all(&mut *connection)
                .await?
                .into_iter()
                .map(|row| row.get::<String, _>(0))
                .filter(|table_name| all || self.tables.contains(table_name));

        let mut tables = vec![];
        for name in names {
            let name: Arc<str> = name.into();
            let (columns, column_types) = sqlx::query(
                "SELECT column_name, data_type FROM information_schema.columns WHERE table_name = $1 and table_schema=$2"
            )
                .bind(name.as_ref())
                .bind(self.schema.as_str())
                .fetch_all(&mut *connection)
                .await?
                .into_iter()
                .map(|row| (row.get::<String, _>(0), row.get::<String, _>(1)))
                .fold((vec![], vec![]), |(mut col_names, mut col_types), (name, ty)| {
                    let ty = match ty.as_str() {
                        "smallint" => ColumnType::I16,
                        "integer"  => ColumnType::I32,
                        "bigint" => ColumnType::I64,
                        "real" => ColumnType::F32,
                        "double precision" => ColumnType::F64,
                        "text" | "character" | "character varying" => ColumnType::Text,
                        "bytea" => ColumnType::Blob,
                        "boolean" => ColumnType::Bool,
                        _ => unimplemented!("unsupported column type '{}' in table '{}'", ty, name)
                    };
                    col_names.push(name);
                    col_types.push(ty);
                    (col_names, col_types)
                });
            //let query = format!("SELECT * FROM \"{}\".\"{name}\" LIMIT=$1 OFFSET=$2", self.schema);
            // FIXME: no batching for now
            let query = format!("SELECT * FROM \"{}\".\"{name}\"", self.schema);
            let table = Table {
                name,
                query,
                columns: columns.into(),
                column_types: column_types.into(),
                limit: 1024,
                offset: 0,
            };
            tables.push(table);
        }
        Ok(tables)
    }

    fn build_payload(&self, table: &Table, rows: Vec<PgRow>) -> Result<PostgresPayload, StdError> {
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
                let value = Value::try_from((column, index, row))?;
                values[index].push(value);
            }
        }
        let batch = PostgresPayload {
            columns: Arc::clone(&table.columns),
            column_types: Arc::clone(&table.column_types),
            values,
            offset: table.offset,
        };
        Ok(batch)
    }
}

struct AckMessage {
    table: Arc<str>,
    offset: i64,
}

impl<Input, Output, SectionChan> Section<Input, Output, SectionChan> for Postgres
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

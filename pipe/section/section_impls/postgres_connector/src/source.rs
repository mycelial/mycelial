use section::{
    command_channel::{Command, SectionChannel},
    decimal,
    futures::{self, FutureExt, Sink, SinkExt, Stream, StreamExt},
    message::{DataType, Value},
    section::Section,
    time, uuid, SectionError, SectionFuture, SectionMessage,
};
use sqlx::{
    postgres::{
        types::{PgMoney, PgTimeTz},
        PgConnectOptions, PgConnection, PgRow, PgValue,
    },
    types::{Json, JsonRawValue},
    Column, ConnectOptions, Row, TypeInfo, Value as _, ValueRef,
};

use std::sync::Arc;
use std::time::Duration;
use std::{pin::pin, str::FromStr};

use crate::{PostgresMessage, PostgresPayload, TableColumn};

#[derive(Debug)]
pub struct Postgres {
    url: String,
    schema: String,
    tables: Vec<String>,
    poll_interval: Duration,
}

#[derive(Debug)]
#[allow(unused)]
pub(crate) struct Table {
    pub name: Arc<str>,
    pub columns: Arc<[TableColumn]>,
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
    ) -> Result<(), SectionError>
    where
        Input: Stream + Send,
        Output: Sink<SectionMessage, Error = SectionError> + Send,
        SectionChan: SectionChannel + Send + Sync,
    {
        let mut connection = PgConnectOptions::from_str(self.url.as_str())?
            .extra_float_digits(2)
            .connect()
            .await?;

        let mut _input = pin!(input.fuse());
        let mut output = pin!(output);

        let mut tables = self.init_tables(&mut connection).await?;

        let mut interval = tokio::time::interval(self.poll_interval);
        loop {
            futures::select! {
                cmd = section_channel.recv().fuse() => {
                   if let Command::Stop = cmd? { return Ok(()) };
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
                        let message = Box::new(PostgresMessage::new(Arc::clone(&table.name), payload, None));
                        output.send(message).await?;
                    }
                },
            }
        }
    }

    async fn init_tables(&self, connection: &mut PgConnection) -> Result<Vec<Table>, SectionError> {
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
            let columns: Vec<TableColumn> = sqlx::query(
                "SELECT column_name, data_type FROM information_schema.columns WHERE table_name = $1 and table_schema=$2"
            )
                .bind(name.as_ref())
                .bind(self.schema.as_str())
                .fetch_all(&mut *connection)
                .await?
                .into_iter()
                .map(|row| {
                    let col_name = row.get::<String, _>(0);
                    let data_type = match row.get::<String, _>(1).as_str() {
                        "smallint" => DataType::I16,
                        "integer"  => DataType::I32,
                        "bigint" => DataType::I64,
                        "real" => DataType::F32,
                        "double precision" => DataType::F64,
                        "text" | "character" | "character varying" => DataType::Str,
                        "bytea" => DataType::Bin,
                        "boolean" => DataType::Bool,
                        "date" => DataType::Date,
                        "time without time zone" => DataType::Time,
                        "json" => DataType::RawJson,
                        "jsonb" => DataType::RawJson,
                        "money" => DataType::Decimal,
                        "numeric" => DataType::Decimal,
                        "time with time zone" => DataType::Str, // legacy type
                        "timestamp without time zone" => DataType::TimeStamp,
                        "timestamp with time zone" => DataType::TimeStampTz,
                        "uuid" => DataType::Uuid,
                        ty => unimplemented!("unsupported column type '{}' for column '{}' in table '{}'", ty, col_name, name)
                    };
                    TableColumn { name: col_name.into(), data_type }
                })
                .collect();
            //let query = format!("SELECT * FROM \"{}\".\"{name}\" LIMIT=$1 OFFSET=$2", self.schema);
            // FIXME: no batching for now
            let query = format!(
                "SELECT {} FROM \"{}\".\"{name}\"",
                columns
                    .iter()
                    .map(|col| col.name.as_ref())
                    .collect::<Vec<_>>()
                    .join(", "),
                self.schema,
            );
            let table = Table {
                name,
                query,
                columns: columns.into(),
                limit: 1024,
                offset: 0,
            };
            tables.push(table);
        }
        Ok(tables)
    }

    fn build_payload(
        &self,
        table: &Table,
        rows: Vec<PgRow>,
    ) -> Result<PostgresPayload, SectionError> {
        let values = match rows.is_empty() {
            true => vec![],
            false => {
                let mut values: Vec<Vec<Value>> =
                    vec![Vec::with_capacity(rows[0].len()); table.columns.len()];
                for row in rows.iter() {
                    for col in row.columns() {
                        let pos = col.ordinal();
                        let raw_value = row.try_get_raw(pos)?;
                        let pg_value: PgValue = ValueRef::to_owned(&raw_value);
                        let type_info = raw_value.type_info();
                        let value = match type_info.name() {
                            "INT2" => pg_value.try_decode::<Option<i16>>()?.map(Value::I16),
                            "INT4" => pg_value.try_decode::<Option<i32>>()?.map(Value::I32),
                            "INT8" => pg_value.try_decode::<Option<i64>>()?.map(Value::I64),
                            "BYTEA" => pg_value.try_decode::<Option<Vec<u8>>>()?.map(Value::from),
                            "CHAR" | "VARCHAR" | "TEXT" => {
                                pg_value.try_decode::<Option<String>>()?.map(Value::from)
                            }
                            "DATE" => pg_value
                                .try_decode::<Option<time::Date>>()?
                                .map(Value::Date),
                            "TIME" => pg_value
                                .try_decode::<Option<time::Time>>()?
                                .map(Value::Time),
                            "TIMETZ" => pg_value
                                .try_decode::<Option<PgTimeTz>>()?
                                .map(|v| format!("{} {}", v.time, v.offset).into()),
                            "TIMESTAMP" => pg_value
                                .try_decode::<Option<time::PrimitiveDateTime>>()?
                                .map(Value::TimeStamp),
                            "TIMESTAMPTZ" => pg_value
                                .try_decode::<Option<time::OffsetDateTime>>()?
                                .map(Value::TimeStampTz),
                            "FLOAT4" => pg_value.try_decode::<Option<f32>>()?.map(Value::F32),
                            "FLOAT8" => pg_value.try_decode::<Option<f64>>()?.map(Value::F64),
                            "JSON" => pg_value
                                .try_decode::<Option<Json<Box<JsonRawValue>>>>()?
                                .map(|v| Value::Str(v.0.into())),
                            "JSONB" => pg_value
                                .try_decode::<Option<Json<Box<JsonRawValue>>>>()?
                                .map(|v| Value::Str(v.0.into())),
                            "MONEY" => pg_value
                                .try_decode::<Option<PgMoney>>()?
                                .map(|v| Value::Decimal(v.to_decimal(2))),
                            "NUMERIC" => pg_value
                                .try_decode::<Option<decimal::Decimal>>()?
                                .map(Value::Decimal),
                            "UUID" => pg_value
                                .try_decode::<Option<uuid::Uuid>>()?
                                .map(Value::Uuid),
                            name => panic!("{}", name),
                        }
                        .unwrap_or(Value::Null);
                        values[pos].push(value)
                    }
                }
                values
            }
        };
        Ok(PostgresPayload {
            columns: Arc::clone(&table.columns),
            values,
        })
    }
}

impl<Input, Output, SectionChan> Section<Input, Output, SectionChan> for Postgres
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

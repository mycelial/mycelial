use chrono::{NaiveDate, NaiveDateTime, NaiveTime, Timelike};
use section::{
    command_channel::{Command, SectionChannel},
    decimal,
    futures::{self, FutureExt, Sink, SinkExt, Stream, StreamExt},
    message::{DataType, TimeUnit, Value},
    section::Section,
    SectionError, SectionFuture, SectionMessage,
};
use sqlx::{
    mysql::{MySqlConnectOptions, MySqlConnection, MySqlRow, MySqlValue},
    types::{Json, JsonRawValue},
    Column, ConnectOptions, Row, TypeInfo, Value as _, ValueRef,
};
use std::sync::Arc;
use std::time::Duration;
use std::{pin::pin, str::FromStr};

use crate::{MysqlMessage, MysqlPayload, TableColumn};

#[derive(Debug)]
pub struct Mysql {
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

impl Mysql {
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
        let connection = &mut MySqlConnectOptions::from_str(self.url.as_str())?
            .connect()
            .await?;

        let mut _input = pin!(input.fuse());
        let mut output = pin!(output);

        let mut tables = self.init_tables(&mut *connection).await?;

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
                            .fetch_all(&mut *connection)
                            .await?;

                        if rows.is_empty() {
                            continue
                        }

                        let payload = self.build_payload(table, rows)?;
                        let message = Box::new(MysqlMessage::new(Arc::clone(&table.name), payload, None));
                        output.send(message).await?;
                    }
                },
            }
        }
    }

    async fn init_tables(
        &self,
        connection: &mut MySqlConnection,
    ) -> Result<Vec<Table>, SectionError> {
        let all = self.tables.iter().any(|t| t == "*");
        let names =
            sqlx::query("SELECT table_name FROM information_schema.tables WHERE table_schema = ?")
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
                "SELECT column_name, data_type FROM information_schema.columns WHERE table_name = ? and table_schema = ?"
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
                        "bigint" => DataType::I64,
                        "text" => DataType::Str,
                        "date" => DataType::Date(TimeUnit::Microsecond),
                        "json" => DataType::RawJson,
                        "numeric" => DataType::Decimal,
                        "uuid" => DataType::Uuid,
                        "binary" => DataType::Bin,
                        "bit" => DataType::Bool,
                        "blob" => DataType::Bin,
                        "char" => DataType::Str,
                        "datetime" => DataType::TimeStamp(TimeUnit::Microsecond),
                        "decimal" => DataType::Decimal,
                        "double" => DataType::F64,
                        "enum" => DataType::Str,
                        "float" => DataType::F32,
                        "int" => DataType::I32,
                        "longblob" => DataType::Bin,
                        "longtext" => DataType::Str,
                        "mediumblob" => DataType::Bin,
                        "mediumtext" => DataType::Str,
                        "mediumint" => DataType::I32,
                        "set" => DataType::Str,
                        "time" => DataType::Time(TimeUnit::Microsecond),
                        // FIXME: it is TimestampUTC?
                        "timestamp" => DataType::TimeStamp(TimeUnit::Microsecond),
                        "tinyblob" => DataType::Bin,
                        "tinytext" => DataType::Str,
                        "tinyint" => DataType::I8,
                        "varbinary" => DataType::Bin,
                        "varchar" => DataType::Str,
                        "year" => DataType::Str,
                        ty => unimplemented!("unsupported column type '{}' for column '{}' in table '{}'", ty, col_name, name)
                    };
                    TableColumn { name: col_name.into(), data_type }
                })
                .collect();
            //let query = format!("SELECT * FROM \"{}\".\"{name}\" LIMIT=$1 OFFSET=$2", self.schema);
            // FIXME: no batching for now
            let query = format!(
                "SELECT {} FROM `{}`.`{name}`",
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
        rows: Vec<MySqlRow>,
    ) -> Result<MysqlPayload, SectionError> {
        let values = match rows.is_empty() {
            true => vec![],
            false => {
                let mut values: Vec<Vec<Value>> =
                    vec![Vec::with_capacity(rows[0].len()); table.columns.len()];
                for row in rows.iter() {
                    for col in row.columns() {
                        let pos = col.ordinal();
                        let raw_value = row.try_get_raw(pos)?;
                        let mysql_value: MySqlValue = ValueRef::to_owned(&raw_value);
                        let type_info = raw_value.type_info();
                        let value = match type_info.name() {
                            "TINYINT" => mysql_value.try_decode::<Option<i8>>()?.map(Value::I8),
                            "SMALLINT" => mysql_value.try_decode::<Option<i16>>()?.map(Value::I16),
                            "MEDIUMINT" | "INT" => {
                                mysql_value.try_decode::<Option<i32>>()?.map(Value::I32)
                            }
                            "BIGINT" => mysql_value.try_decode::<Option<i64>>()?.map(Value::I64),
                            "DECIMAL" | "NUMERIC" => mysql_value
                                .try_decode::<Option<decimal::Decimal>>()?
                                .map(Value::Decimal),
                            "FLOAT" => mysql_value.try_decode::<Option<f32>>()?.map(Value::F32),
                            "DOUBLE" => mysql_value.try_decode::<Option<f64>>()?.map(Value::F64),
                            "BIT" => mysql_value.try_decode::<Option<bool>>()?.map(Value::Bool),
                            "DATE" => mysql_value.try_decode::<Option<NaiveDate>>()?.map(|v| {
                                Value::Date(
                                    TimeUnit::Microsecond,
                                    v.and_hms_opt(0, 0, 0).unwrap().timestamp_micros(),
                                )
                            }),
                            "TIME" => mysql_value.try_decode::<Option<NaiveTime>>()?.map(|v| {
                                let micros = NaiveDateTime::from_timestamp_opt(
                                    v.num_seconds_from_midnight() as _,
                                    v.nanosecond(),
                                )
                                .unwrap()
                                .timestamp_micros();
                                Value::Time(TimeUnit::Microsecond, micros)
                            }),
                            "CHAR" | "VARCHAR" | "TEXT" | "ENUM" | "LONGTEXT" | "MEDIUMTEXT"
                            | "MULTILINESTRING" => {
                                mysql_value.try_decode::<Option<String>>()?.map(Value::from)
                            }
                            "BINARY" | "VARBINARY" => mysql_value
                                .try_decode::<Option<Vec<u8>>>()?
                                .map(Value::from),
                            "BLOB" => mysql_value
                                .try_decode::<Option<Vec<u8>>>()?
                                .map(Value::from),
                            "DATETIME" => {
                                mysql_value.try_decode::<Option<NaiveDateTime>>()?.map(|v| {
                                    Value::TimeStamp(TimeUnit::Microsecond, v.timestamp_micros())
                                })
                            }
                            "JSON" => mysql_value
                                .try_decode::<Option<Json<Box<JsonRawValue>>>>()?
                                .map(|v| Value::Str(v.0.into())),
                            "YEAR" => mysql_value.try_decode::<Option<u32>>()?.map(Value::U32),

                            name => panic!("{}", name),
                        }
                        .unwrap_or(Value::Null);
                        values[pos].push(value)
                    }
                }
                values
            }
        };
        Ok(MysqlPayload {
            columns: Arc::clone(&table.columns),
            values,
        })
    }
}

impl<Input, Output, SectionChan> Section<Input, Output, SectionChan> for Mysql
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

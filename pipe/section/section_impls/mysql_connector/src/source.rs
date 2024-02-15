use chrono::{DateTime, NaiveDate, NaiveDateTime, NaiveTime, Timelike, Utc};
use section::{
    command_channel::{Command, SectionChannel},
    decimal,
    futures::{self, FutureExt, Sink, SinkExt, Stream, StreamExt},
    message::{Chunk, DataType, TimeUnit, Value},
    section::Section,
    uuid, SectionError, SectionFuture, SectionMessage,
};
use sqlx::{
    mysql::{MySqlConnectOptions, MySqlRow, MySqlValue},
    types::{Json, JsonRawValue},
    Column, ConnectOptions, Row, TypeInfo, Value as _, ValueRef,
};
use std::sync::Arc;
use std::time::Duration;
use std::{pin::pin, str::FromStr};
use tokio::sync::mpsc::Sender;

use crate::{MysqlColumn, MysqlMessage, MysqlPayload};

#[derive(Debug)]
pub struct Mysql {
    url: String,
    origin: Arc<str>,
    query: String,
    poll_interval: Duration,
}

impl Mysql {
    pub fn new(
        url: impl Into<String>,
        origin: impl AsRef<str>,
        query: impl Into<String>,
        poll_interval: Duration,
    ) -> Self {
        Self {
            url: url.into(),
            origin: Arc::from(origin.as_ref()),
            query: query.into(),
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

        // set connection session timezone to UTC so we can treat all returned timestamps/datetimes as timestamp UTC
        sqlx::query("SET time_zone = \"+00:00\"")
            .execute(&mut *connection)
            .await?;

        let mut _input = pin!(input.fuse());
        let mut output = pin!(output);

        let mut interval = tokio::time::interval(self.poll_interval);
        loop {
            futures::select! {
                cmd = section_channel.recv().fuse() => {
                   if let Command::Stop = cmd? { return Ok(()) };
                },
                _ = interval.tick().fuse() => {
                    let mut row_stream = sqlx::query(self.query.as_str())
                        .fetch(&mut *connection);

                    // check if row stream contains any result
                    let mut row_stream = match row_stream.next().await {
                        Some(res) => futures::stream::once(async { res }).chain(row_stream),
                        None => continue
                    };

                    let mut row_stream = pin!(row_stream);
                    let mut buf = Vec::with_capacity(256);
                    let (mut tx, rx) = tokio::sync::mpsc::channel(1);
                    let message = MysqlMessage::new(Arc::clone(&self.origin), rx);
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
        buf: &mut Vec<MySqlRow>,
    ) -> Result<(), SectionError> {
        if !buf.is_empty() {
            let chunk = Chunk::DataFrame(Box::new(self.build_payload(buf.as_slice())?));
            tx.send(Ok(chunk)).await.map_err(|_| "send error")?;
            buf.truncate(0);
        }
        Ok(())
    }

    fn build_payload(&self, rows: &[MySqlRow]) -> Result<MysqlPayload, SectionError> {
        let first_row = match rows.first() {
            Some(row) => row,
            None => Err("no rows")?,
        };
        let columns = first_row.columns();
        let mut values: Vec<Vec<Value>> = vec![Vec::with_capacity(rows.len()); columns.len()];
        let (columns, funcs) = columns.iter().fold(
            (
                Vec::with_capacity(columns.len()),
                Vec::with_capacity(columns.len()),
            ),
            |(mut cols, mut funcs), col| {
                let (dt, func) = from_mysql_type_name(col.type_info().name());
                cols.push(MysqlColumn::new(col.name(), dt));
                funcs.push(func);
                (cols, funcs)
            },
        );
        for row in rows.iter() {
            for (col, parse) in row.columns().iter().zip(funcs.iter()) {
                let pos = col.ordinal();
                let raw_value = row.try_get_raw(pos)?;
                let mysql_value: MySqlValue = ValueRef::to_owned(&raw_value);
                values[pos].push(parse(mysql_value)?);
            }
        }
        Ok(MysqlPayload { columns, values })
    }
}

#[allow(clippy::type_complexity)]
pub(crate) fn from_mysql_type_name(
    type_name: &str,
) -> (DataType, fn(MySqlValue) -> Result<Value, SectionError>) {
    match type_name {
        "TINYINT UNSIGNED" => (DataType::U8, |mysql_value| {
            Ok(mysql_value
                .try_decode::<Option<u8>>()?
                .map(Value::U8)
                .unwrap_or(Value::Null))
        }),
        "SMALLINT UNSIGNED" => (DataType::U16, |mysql_value| {
            Ok(mysql_value
                .try_decode::<Option<u16>>()?
                .map(Value::U16)
                .unwrap_or(Value::Null))
        }),
        "INT UNSIGNED" => (DataType::U32, |mysql_value| {
            Ok(mysql_value
                .try_decode::<Option<u32>>()?
                .map(Value::U32)
                .unwrap_or(Value::Null))
        }),
        "BIGINT UNSIGNED" => (DataType::U64, |mysql_value| {
            Ok(mysql_value
                .try_decode::<Option<u64>>()?
                .map(Value::U64)
                .unwrap_or(Value::Null))
        }),
        "TINYINT" => (DataType::I8, |mysql_value| {
            Ok(mysql_value
                .try_decode::<Option<i8>>()?
                .map(Value::I8)
                .unwrap_or(Value::Null))
        }),
        "SMALLINT" => (DataType::I16, |mysql_value| {
            Ok(mysql_value
                .try_decode::<Option<i16>>()?
                .map(Value::I16)
                .unwrap_or(Value::Null))
        }),
        "INT" => (DataType::I32, |mysql_value| {
            Ok(mysql_value
                .try_decode::<Option<i32>>()?
                .map(Value::I32)
                .unwrap_or(Value::Null))
        }),
        "BIGINT" => (DataType::I64, |mysql_value| {
            Ok(mysql_value
                .try_decode::<Option<i64>>()?
                .map(Value::I64)
                .unwrap_or(Value::Null))
        }),
        "BLOB" => (DataType::Bin, |mysql_value| {
            Ok(mysql_value
                .try_decode::<Option<Vec<u8>>>()?
                .map(Value::from)
                .unwrap_or(Value::Null))
        }),
        "CHAR" | "VARCHAR" | "TEXT" => (DataType::Str, |mysql_value| {
            Ok(mysql_value
                .try_decode::<Option<String>>()?
                .map(Value::from)
                .unwrap_or(Value::Null))
        }),
        "DATE" => (DataType::Date(TimeUnit::Second), |mysql_value| {
            let value = mysql_value.try_decode::<Option<NaiveDate>>()?.map(|v| {
                Value::Date(
                    TimeUnit::Second,
                    v.and_hms_opt(0, 0, 0).unwrap().timestamp(),
                )
            });
            Ok(value.unwrap_or(Value::Null))
        }),
        "YEAR" => (DataType::U32, |mysql_value| {
            Ok(mysql_value
                .try_decode::<Option<u32>>()?
                .map(Value::U32)
                .unwrap_or(Value::Null))
        }),
        "TIME" => (DataType::Time(TimeUnit::Microsecond), |mysql_value| {
            let value = mysql_value.try_decode::<Option<NaiveTime>>()?.map(|v| {
                let micros = NaiveDateTime::from_timestamp_opt(
                    v.num_seconds_from_midnight() as _,
                    v.nanosecond(),
                )
                .unwrap()
                .timestamp_micros();
                Value::Time(TimeUnit::Microsecond, micros)
            });
            Ok(value.unwrap_or(Value::Null))
        }),
        "TIMESTAMP" => (
            DataType::TimeStampUTC(TimeUnit::Microsecond),
            |mysql_value| {
                let value = mysql_value
                    .try_decode::<Option<DateTime<Utc>>>()?
                    .map(|v| Value::TimeStampUTC(TimeUnit::Microsecond, v.timestamp_micros()));
                Ok(value.unwrap_or(Value::Null))
            },
        ),
        "DATETIME" => (
            DataType::TimeStampUTC(TimeUnit::Microsecond),
            |mysql_value| {
                let value = mysql_value
                    .try_decode::<Option<DateTime<Utc>>>()?
                    .map(|v| Value::TimeStampUTC(TimeUnit::Microsecond, v.timestamp_micros()));
                Ok(value.unwrap_or(Value::Null))
            },
        ),
        "FLOAT" => (DataType::F32, |mysql_value| {
            let value = mysql_value
                .try_decode::<Option<f32>>()?
                .map(Value::F32)
                .unwrap_or(Value::Null);
            Ok(value)
        }),
        "DOUBLE" => (DataType::F64, |mysql_value| {
            let value = mysql_value
                .try_decode::<Option<f64>>()?
                .map(Value::F64)
                .unwrap_or(Value::Null);
            Ok(value)
        }),
        "JSON" => (DataType::RawJson, |mysql_value| {
            let value = mysql_value
                .try_decode::<Option<Json<Box<JsonRawValue>>>>()?
                .map(|v| Value::Str(v.0.into()))
                .unwrap_or(Value::Null);
            Ok(value)
        }),
        "JSONB" => (DataType::RawJson, |mysql_value| {
            let value = mysql_value
                .try_decode::<Option<Json<Box<JsonRawValue>>>>()?
                .map(|v| Value::Str(v.0.into()))
                .unwrap_or(Value::Null);
            Ok(value)
        }),
        "DECIMAL" => (DataType::Decimal, |mysql_value| {
            let value = mysql_value
                .try_decode::<Option<decimal::Decimal>>()?
                .map(Value::Decimal)
                .unwrap_or(Value::Null);
            Ok(value)
        }),
        "UUID" => (DataType::Uuid, |mysql_value| {
            let value = mysql_value
                .try_decode::<Option<uuid::Uuid>>()?
                .map(Value::Uuid)
                .unwrap_or(Value::Null);
            Ok(value)
        }),
        name => unimplemented!("unsupported mysql data type: {:?}", name),
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

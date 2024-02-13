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
    postgres::{
        types::{PgMoney, PgTimeTz},
        PgConnectOptions, PgRow, PgValue,
    },
    types::{Json, JsonRawValue},
    Column, ConnectOptions, Row, TypeInfo, Value as _, ValueRef,
};
use std::sync::Arc;
use std::time::Duration;
use std::{pin::pin, str::FromStr};

use crate::{PostgresColumn, PostgresMessage, PostgresPayload};
use tokio::sync::mpsc::Sender;

#[derive(Debug)]
pub struct Postgres {
    url: String,
    origin: Arc<str>,
    query: String,
    poll_interval: Duration,
}

impl Postgres {
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
        let mut connection = PgConnectOptions::from_str(self.url.as_str())?
            .extra_float_digits(2)
            .connect()
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
                        .fetch(&mut connection);

                    // check if row stream contains any result
                    let mut row_stream = match row_stream.next().await {
                        Some(res) => futures::stream::once(async { res }).chain(row_stream),
                        None => continue
                    };

                    let mut row_stream = pin!(row_stream);
                    let mut buf = Vec::with_capacity(256);
                    let (mut tx, rx) = tokio::sync::mpsc::channel(1);
                    let message = PostgresMessage::new(Arc::clone(&self.origin), rx);
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
        buf: &mut Vec<PgRow>,
    ) -> Result<(), SectionError> {
        if !buf.is_empty() {
            let chunk = Chunk::DataFrame(Box::new(self.build_payload(buf.as_slice())?));
            tx.send(Ok(chunk)).await.map_err(|_| "send error")?;
            buf.truncate(0);
        }
        Ok(())
    }

    fn build_payload(&self, rows: &[PgRow]) -> Result<PostgresPayload, SectionError> {
        if rows.is_empty() {
            Err("no rows")?
        }
        let raw_columns = rows[0].columns();
        let (columns, parse_funcs) =
            raw_columns
                .iter()
                .fold((vec![], vec![]), |(mut columns, mut funcs), col| {
                    let (dt, func) = from_pg_type_name(col.type_info().name());
                    let column = PostgresColumn::new(col.name(), dt);
                    columns.push(column);
                    funcs.push(func);
                    (columns, funcs)
                });
        let mut values: Vec<Vec<Value>> = vec![Vec::with_capacity(rows[0].len()); columns.len()];
        for row in rows.iter() {
            for (col, parse_func) in raw_columns.iter().zip(parse_funcs.iter()) {
                let pos = col.ordinal();
                let raw_value = row.try_get_raw(pos)?;
                let pg_value: PgValue = ValueRef::to_owned(&raw_value);
                let value = parse_func(pg_value)?;
                values[pos].push(value)
            }
        }
        Ok(PostgresPayload { columns, values })
    }
}

#[allow(clippy::type_complexity)]
pub(crate) fn from_pg_type_name(
    type_name: &str,
) -> (DataType, fn(PgValue) -> Result<Value, SectionError>) {
    match type_name {
        "INT2" => (DataType::I16, |pg_value| {
            Ok(pg_value
                .try_decode::<Option<i16>>()?
                .map(Value::I16)
                .unwrap_or(Value::Null))
        }),
        "INT4" => (DataType::I32, |pg_value| {
            Ok(pg_value
                .try_decode::<Option<i32>>()?
                .map(Value::I32)
                .unwrap_or(Value::Null))
        }),
        "INT8" => (DataType::I64, |pg_value| {
            Ok(pg_value
                .try_decode::<Option<i64>>()?
                .map(Value::I64)
                .unwrap_or(Value::Null))
        }),
        "BYTEA" => (DataType::Bin, |pg_value| {
            Ok(pg_value
                .try_decode::<Option<Vec<u8>>>()?
                .map(Value::from)
                .unwrap_or(Value::Null))
        }),
        "CHAR" | "VARCHAR" | "TEXT" => (DataType::Str, |pg_value| {
            Ok(pg_value
                .try_decode::<Option<String>>()?
                .map(Value::from)
                .unwrap_or(Value::Null))
        }),
        "DATE" => (DataType::Date(TimeUnit::Second), |pg_value| {
            let value = pg_value.try_decode::<Option<NaiveDate>>()?.map(|v| {
                Value::Date(
                    TimeUnit::Second,
                    v.and_hms_opt(0, 0, 0).unwrap().timestamp(),
                )
            });
            Ok(value.unwrap_or(Value::Null))
        }),
        "TIME" => (DataType::Time(TimeUnit::Microsecond), |pg_value| {
            let value = pg_value.try_decode::<Option<NaiveTime>>()?.map(|v| {
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
        "TIMETZ" => (DataType::Str, |pg_value| {
            let value = pg_value
                .try_decode::<Option<PgTimeTz>>()?
                .map(|v| format!("{} {}", v.time, v.offset).into());
            Ok(value.unwrap_or(Value::Null))
        }),
        "TIMESTAMP" => (DataType::TimeStamp(TimeUnit::Microsecond), |pg_value| {
            let value = pg_value
                .try_decode::<Option<NaiveDateTime>>()?
                .map(|v| Value::TimeStamp(TimeUnit::Microsecond, v.timestamp_micros()));
            Ok(value.unwrap_or(Value::Null))
        }),
        "TIMESTAMPTZ" => (DataType::TimeStampUTC(TimeUnit::Microsecond), |pg_value| {
            let value = pg_value
                .try_decode::<Option<DateTime<Utc>>>()?
                .map(|v| Value::TimeStampUTC(TimeUnit::Microsecond, v.timestamp_micros()));
            Ok(value.unwrap_or(Value::Null))
        }),
        "FLOAT4" => (DataType::F32, |pg_value| {
            let value = pg_value
                .try_decode::<Option<f32>>()?
                .map(Value::F32)
                .unwrap_or(Value::Null);
            Ok(value)
        }),
        "FLOAT8" => (DataType::F64, |pg_value| {
            let value = pg_value
                .try_decode::<Option<f64>>()?
                .map(Value::F64)
                .unwrap_or(Value::Null);
            Ok(value)
        }),
        "JSON" => (DataType::RawJson, |pg_value| {
            let value = pg_value
                .try_decode::<Option<Json<Box<JsonRawValue>>>>()?
                .map(|v| Value::Str(v.0.into()))
                .unwrap_or(Value::Null);
            Ok(value)
        }),
        "JSONB" => (DataType::RawJson, |pg_value| {
            let value = pg_value
                .try_decode::<Option<Json<Box<JsonRawValue>>>>()?
                .map(|v| Value::Str(v.0.into()))
                .unwrap_or(Value::Null);
            Ok(value)
        }),
        "MONEY" => (DataType::Decimal, |pg_value| {
            let value = pg_value
                .try_decode::<Option<PgMoney>>()?
                .map(|v| Value::Decimal(v.to_decimal(2)))
                .unwrap_or(Value::Null);
            Ok(value)
        }),
        "NUMERIC" => (DataType::Decimal, |pg_value| {
            let value = pg_value
                .try_decode::<Option<decimal::Decimal>>()?
                .map(Value::Decimal)
                .unwrap_or(Value::Null);
            Ok(value)
        }),
        "UUID" => (DataType::Uuid, |pg_value| {
            let value = pg_value
                .try_decode::<Option<uuid::Uuid>>()?
                .map(Value::Uuid)
                .unwrap_or(Value::Null);
            Ok(value)
        }),
        name => unimplemented!("unsupported postgres data type: {}", name),
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

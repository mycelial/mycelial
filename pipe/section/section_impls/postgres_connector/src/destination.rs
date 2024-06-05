use chrono::DateTime;
use section::{
    command_channel::{Command, SectionChannel},
    futures::{self, FutureExt, Sink, Stream, StreamExt},
    message::{Chunk, DataType, TimeUnit, ValueView},
    section::Section,
    SectionError, SectionFuture, SectionMessage,
};
use std::{collections::HashSet, pin::pin};

use crate::{escape, generate_schema};
use sqlx::{
    postgres::PgConnectOptions, types::chrono::NaiveDateTime, ConnectOptions, Connection,
    QueryBuilder,
};
use std::str::FromStr;

#[derive(Debug)]
pub struct Postgres {
    url: String,
    schema: String,
    truncate: bool,
}

impl Postgres {
    pub fn new(url: impl Into<String>, schema: impl Into<String>, truncate: bool) -> Self {
        Self {
            url: url.into(),
            schema: escape(schema.into()),
            truncate,
        }
    }

    async fn enter_loop<Input, Output, SectionChan>(
        self,
        input: Input,
        output: Output,
        mut section_chan: SectionChan,
    ) -> Result<(), SectionError>
    where
        Input: Stream<Item = SectionMessage> + Send + 'static,
        Output: Sink<SectionMessage, Error = SectionError> + Send + 'static,
        SectionChan: SectionChannel + Send + 'static,
    {
        let mut input = pin!(input.fuse());
        let mut _output = pin!(output);

        let connection = &mut PgConnectOptions::from_str(self.url.as_str())?
            .extra_float_digits(2)
            .connect()
            .await?;

        let mut tables = HashSet::<String>::new();

        loop {
            futures::select! {
                cmd = section_chan.recv().fuse() => {
                    if let Command::Stop = cmd? { return Ok(()) }
                },
                message = input.next() => {
                    let mut message = match message {
                        None => Err("input closed")?,
                        Some(message) => message,
                    };
                    let name = escape(message.origin());
                    let mut transaction = connection.begin().await?;
                    let mut initialized = false;
                    let mut insert_query = String::new();
                    while let Some(chunk) = message.next().await? {
                        let df = match chunk{
                            Chunk::DataFrame(df) => df,
                            _ => Err("expected dataframe chunk".to_string())?
                        };
                        let mut columns = df.columns();
                        if !initialized {
                            initialized = true;
                            if !tables.contains(name.as_str()) {
                                let schema = generate_schema(self.schema.as_str(), name.as_str(), df.as_ref())?;
                                sqlx::query(&schema).execute(&mut *transaction).await?;
                                tables.insert(name.clone());
                            };
                            if self.truncate {
                                sqlx::query(&format!("TRUNCATE \"{}\".\"{name}\"", self.schema))
                                    .execute(&mut *transaction)
                                    .await?;
                            }
                            let column_names = columns.iter().map(|col| escape(col.name())).collect::<Vec<_>>().join(",");
                            insert_query = format!(
                                "INSERT INTO \"{}\".\"{name}\"({column_names}) VALUES",
                                 self.schema
                            );
                        }
                        let mut query = QueryBuilder::new(&insert_query);
                        let mut count = 0;
                        while let Some(row) = columns.iter_mut().map(|col| col.next()).collect::<Option<Vec<_>>>() {
                            if count + row.len() >= 65535 {
                                count = 0;
                                query.build().execute(&mut *transaction).await?;
                                query = QueryBuilder::new(&insert_query);
                            }
                            if count != 0 {
                                query.push(",");
                            }
                            count += row.len();
                            query.push("(");
                            for (pos, value) in row.into_iter().enumerate() {
                                if pos != 0 {
                                    query.push(",");
                                }
                                match value {
                                    ValueView::I8(i) => query.push_bind(i),
                                    ValueView::I16(i) => query.push_bind(i),
                                    ValueView::I32(i) => query.push_bind(i),
                                    ValueView::I64(i) => query.push_bind(i),
                                    ValueView::F32(f) => query.push_bind(f),
                                    ValueView::F64(f) => query.push_bind(f),
                                    ValueView::Str(s) => query.push_bind(s),
                                    ValueView::Bin(b) => query.push_bind(b),
                                    ValueView::Bool(b) => query.push_bind(b),
                                    ValueView::Time(tu, t) => {
                                        let ts = to_naive_date(tu, t);
                                        query.push_bind(ts.unwrap().time())
                                    },
                                    ValueView::Date(tu, t) => {
                                        let ts = to_naive_date(tu, t);
                                        query.push_bind(ts.unwrap().date())
                                    },
                                    ValueView::TimeStamp(tu, t) => {
                                        let ts = to_naive_date(tu, t);
                                        query.push_bind(ts.unwrap())
                                    },
                                    ValueView::TimeStampUTC(tu, t) => {
                                        let ts = to_naive_date(tu, t).map(|ts| ts.and_utc());
                                        query.push_bind(ts.unwrap())
                                    }
                                    ValueView::Decimal(d) => query.push_bind(d),
                                    ValueView::Uuid(u) => query.push_bind(u),
                                    ValueView::Null => query.push_bind(Option::<&str>::None),
                                    unimplemented => unimplemented!("unimplemented value: {:?}", unimplemented),
                                };
                                if value.data_type() == DataType::RawJson {
                                    query.push("::json");
                                }
                            };
                            query.push(")");
                        };
                        if count > 0 {
                            query.build().execute(&mut *transaction).await?;
                        }
                    }
                    transaction.commit().await?;
                    message.ack().await;
                }
            }
        }
    }
}

// FIXME: move this function to lib
fn to_naive_date(tu: TimeUnit, t: i64) -> Option<NaiveDateTime> {
    match tu {
        TimeUnit::Second => DateTime::from_timestamp(t, 0),
        TimeUnit::Millisecond => DateTime::from_timestamp_micros(t * 1000),
        TimeUnit::Microsecond => DateTime::from_timestamp_micros(t),
        TimeUnit::Nanosecond => {
            DateTime::from_timestamp(t / 1_000_000_000, (t % 1_000_000_000) as _)
        }
    }
    .map(|d| d.naive_utc())
}

impl<Input, Output, SectionChan> Section<Input, Output, SectionChan> for Postgres
where
    Input: Stream<Item = SectionMessage> + Send + 'static,
    Output: Sink<SectionMessage, Error = SectionError> + Send + 'static,
    SectionChan: SectionChannel + Send + 'static,
{
    type Error = SectionError;
    type Future = SectionFuture;

    fn start(self, input: Input, output: Output, command: SectionChan) -> Self::Future {
        Box::pin(async move { self.enter_loop(input, output, command).await })
    }
}

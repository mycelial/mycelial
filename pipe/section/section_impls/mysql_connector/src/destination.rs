use chrono::NaiveDateTime;
use section::{
    command_channel::{Command, SectionChannel},
    futures::{self, FutureExt, Sink, Stream, StreamExt},
    message::{Chunk, TimeUnit, ValueView},
    section::Section,
    SectionError, SectionFuture, SectionMessage,
};
use std::pin::pin;

use crate::{escape_table_name, generate_schema};
use sqlx::Connection;
use sqlx::{mysql::MySqlConnectOptions, ConnectOptions};
use std::str::FromStr;

#[derive(Debug)]
pub struct Mysql {
    url: String,
    truncate: bool,
}

impl Mysql {
    pub fn new(url: impl Into<String>, truncate: bool) -> Self {
        Self {
            url: url.into(),
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

        let connection = &mut MySqlConnectOptions::from_str(self.url.as_str())?
            .connect()
            .await?;

        // set connection session timezone to UTC so mysql will treat incoming timestamps as UTC
        sqlx::query("SET time_zone = \"+00:00\"")
            .execute(&mut *connection)
            .await?;

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
                    let name = escape_table_name(message.origin());
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
                            let schema = generate_schema(name.as_str(), df.as_ref())?;
                            sqlx::query(&schema).execute(&mut *transaction).await?;
                            if self.truncate {
                                sqlx::query(&format!("TRUNCATE `{name}`"))
                                    .execute(&mut *transaction).await?;
                            }
                            let values_placeholder = columns.iter().map(|_| "?").collect::<Vec<_>>().join(",");
                            insert_query = format!("INSERT INTO `{name}` VALUES({values_placeholder})");
                        }
                        'outer: loop {
                            let mut query = sqlx::query(&insert_query);
                            for col in columns.iter_mut() {
                                let next = col.next();
                                if next.is_none() {
                                    break 'outer;
                                }
                                let n = next.unwrap();
                                query = match n {
                                    ValueView::U8(u) => query.bind(u),
                                    ValueView::U16(u) => query.bind(u),
                                    ValueView::U32(u) => query.bind(u),
                                    ValueView::U64(u) => query.bind(u),
                                    ValueView::I8(i) => query.bind(i as i64),
                                    ValueView::I16(i) => query.bind(i as i64),
                                    ValueView::I32(i) => query.bind(i as i64),
                                    ValueView::I64(i) => query.bind(i),
                                    ValueView::F32(f) => query.bind(f),
                                    ValueView::F64(f) => query.bind(f),
                                    ValueView::Str(s) => query.bind(s),
                                    ValueView::Bin(b) => query.bind(b),
                                    ValueView::Bool(b) => query.bind(b),
                                    ValueView::Time(tu, t) => {
                                        let ts = to_naive_date(tu, t);
                                        query.bind(ts.unwrap().time())
                                    },
                                    ValueView::Date(tu, t) => {
                                        let ts = to_naive_date(tu, t);
                                        query.bind(ts.unwrap().date())
                                    },
                                    ValueView::TimeStamp(tu, t) => {
                                        let ts = to_naive_date(tu, t);
                                        query.bind(ts.unwrap())
                                    },
                                    ValueView::TimeStampUTC(tu, t) => {
                                        let ts = to_naive_date(tu, t).map(|ts| ts.and_utc());
                                        query.bind(ts.unwrap())
                                    },
                                    ValueView::Decimal(d) => query.bind(d),
                                    ValueView::Uuid(u) => query.bind(u),
                                    ValueView::Null => query.bind(Option::<&str>::None),
                                    unimplemented => unimplemented!("unimplemented value: {:?}", unimplemented),
                                }
                            }
                            query.execute(&mut *transaction).await?;
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
        TimeUnit::Second => NaiveDateTime::from_timestamp_opt(t, 0),
        TimeUnit::Millisecond => NaiveDateTime::from_timestamp_micros(t * 1000),
        TimeUnit::Microsecond => NaiveDateTime::from_timestamp_micros(t),
        TimeUnit::Nanosecond => {
            NaiveDateTime::from_timestamp_opt(t / 1_000_000_000, (t % 1_000_000_000) as _)
        }
    }
}

impl<Input, Output, SectionChan> Section<Input, Output, SectionChan> for Mysql
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

use section::{
    command_channel::{Command, SectionChannel},
    futures::{self, FutureExt, Sink, Stream, StreamExt},
    message::{Chunk, TimeUnit, ValueView},
    section::Section,
    SectionError, SectionMessage,
};
use std::pin::{pin, Pin};

use crate::{escape_table_name, generate_column_names, generate_schema};
use sqlx::types::chrono::NaiveDateTime;
use sqlx::{sqlite::SqliteConnectOptions, ConnectOptions};
use std::future::Future;
use std::str::FromStr;

#[derive(Debug)]
pub struct Sqlite {
    path: String,
    truncate: bool,
}

impl Sqlite {
    pub fn new(path: impl Into<String>, truncate: bool) -> Self {
        Self {
            path: path.into(),
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

        let connection = &mut SqliteConnectOptions::from_str(self.path.as_str())?
            .create_if_missing(true)
            .connect()
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
                    // Manually start transaction.
                    // Immediate transaction will acquire database exclusive lock at the beginning
                    // of transaction, instead of when transaction will want to write something.
                    // This approach prevents 'database locked' errors, which happen on transaction
                    // commit.
                    // Sqlx doesn't provide support immediate transactions, so instead we just
                    // maintain transaction by ourself
                    sqlx::query("BEGIN IMMEDIATE").execute(&mut *connection).await?;
                    let name = escape_table_name(message.origin());
                    let mut initialized = false;
                    let mut insert_query = String::new();
                    loop {
                        futures::select! {
                            chunk = message.next().fuse() => {
                                let df = match chunk? {
                                    None => break,
                                    Some(Chunk::DataFrame(df)) => df,
                                    Some(ch) => Err(format!("unexpected chunk type: {:?}", ch))?,
                                };
                                let columns = &mut df.columns();
                                // generate schema, maybe truncate and prepare insert query
                                if !initialized {
                                    initialized = true;
                                    let schema = generate_schema(&name, df.as_ref());
                                    sqlx::query(&schema).execute(&mut *connection).await?;
                                    if self.truncate {
                                        sqlx::query(&format!("DELETE FROM \"{name}\""))
                                            .execute(&mut *connection)
                                            .await?;
                                    }
                                    let values_placeholder = (0..columns.len()).map(|_| "?").collect::<Vec<_>>().join(",");
                                    let columns = generate_column_names(df.as_ref());
                                    insert_query = format!("INSERT OR IGNORE INTO \"{name}\" ({columns}) VALUES({values_placeholder})");
                                }
                                'outer: loop {
                                    let mut query = sqlx::query(&insert_query);
                                    for col in columns.iter_mut() {
                                        let next = col.next();
                                        if next.is_none() {
                                            break 'outer;
                                        }
                                        query = match next.unwrap() {
                                            ValueView::I8(i) => query.bind(i),
                                            ValueView::I16(i) => query.bind(i),
                                            ValueView::I32(i) => query.bind(i),
                                            ValueView::I64(i) => query.bind(i),
                                            ValueView::U8(i) => query.bind(i),
                                            ValueView::U16(i) => query.bind(i),
                                            ValueView::U32(i) => query.bind(i),
                                            ValueView::U64(i) => query.bind(i as i64),
                                            ValueView::F32(f) => query.bind(f),
                                            ValueView::F64(f) => query.bind(f),
                                            ValueView::Str(s) => query.bind(s),
                                            ValueView::Bin(b) => query.bind(b),
                                            ValueView::Bool(b) => query.bind(b),
                                            ValueView::Time(tu, t) => {
                                                let ts = to_naive_date(tu, t).unwrap();
                                                query.bind(ts.to_string())
                                            },
                                            ValueView::Date(tu, t) => {
                                                let ts = to_naive_date(tu, t).unwrap();
                                                query.bind(ts.to_string())}
                                            ,
                                            ValueView::TimeStamp(tu, t) => {
                                                let ts = to_naive_date(tu, t).unwrap();
                                                query.bind(ts.to_string())
                                            },
                                            ValueView::TimeStampUTC(tu, t) => {
                                                let ts = to_naive_date(tu, t).map(|ts| ts.and_utc()).unwrap();
                                                query.bind(ts.to_string())
                                            },
                                            ValueView::Decimal(d) => query.bind(d.to_string()),
                                            ValueView::Uuid(u) => query.bind(u.to_string()),
                                            ValueView::Null => query.bind(Option::<&str>::None),
                                            unimplemented => unimplemented!("unimplemented value: {:?}", unimplemented),
                                        };
                                    }
                                    // FIXME: add batch support?
                                    query.execute(&mut *connection).await.map_err(|_| "failed to finish transaction")?;
                                }
                            },
                            cmd = section_chan.recv().fuse() => {
                                if let Command::Stop = cmd? { return Ok(()) }
                            },
                        }
                    }
                    sqlx::query("COMMIT").execute(&mut *connection).await?;
                    message.ack().await;
                }
            }
        }
    }
}

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

impl<Input, Output, SectionChan> Section<Input, Output, SectionChan> for Sqlite
where
    Input: Stream<Item = SectionMessage> + Send + 'static,
    Output: Sink<SectionMessage, Error = SectionError> + Send + 'static,
    SectionChan: SectionChannel + Send + 'static,
{
    type Error = SectionError;
    type Future = Pin<Box<dyn Future<Output = Result<(), Self::Error>> + Send + 'static>>;

    fn start(self, input: Input, output: Output, command: SectionChan) -> Self::Future {
        Box::pin(async move { self.enter_loop(input, output, command).await })
    }
}

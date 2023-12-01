use section::{
    command_channel::{Command, SectionChannel},
    futures::{self, FutureExt, Sink, Stream, StreamExt},
    message::{Chunk, ValueView},
    pretty_print::pretty_print,
    section::Section,
    SectionError, SectionMessage,
};
use std::pin::{pin, Pin};

use crate::{escape_table_name, generate_schema};
use sqlx::Connection;
use sqlx::{sqlite::SqliteConnectOptions, ConnectOptions};
use std::future::Future;
use std::str::FromStr;

#[derive(Debug)]
pub struct Sqlite {
    path: String,
}

impl Sqlite {
    pub fn new(path: impl Into<String>) -> Self {
        Self { path: path.into() }
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
                    let name = escape_table_name(message.origin());
                    let mut transaction = connection.begin().await?;
                    loop {
                        futures::select! {
                            chunk = message.next().fuse() => {
                                let df = match chunk? {
                                    None => break,
                                    Some(Chunk::DataFrame(df)) => df,
                                    Some(ch) => Err(format!("unexpected chunk type: {:?}", ch))?,
                                };
                                //println!("{}\n{}", name, pretty_print(&*df));
                                let schema = generate_schema(&name, df.as_ref());
                                sqlx::query(&schema).execute(&mut *transaction).await?;
                                let columns = &mut df.columns();
                                let values_placeholder = (0..columns.len()).map(|_| "?").collect::<Vec<_>>().join(",");
                                let insert = format!("INSERT OR IGNORE INTO \"{name}\" VALUES({values_placeholder})");
                                'outer: loop {
                                    let mut query = sqlx::query(&insert);
                                    for col in columns.iter_mut() {
                                        let next = col.next();
                                        if next.is_none() {
                                            break 'outer;
                                        }
                                        query = match next.unwrap() {
                                            ValueView::I8(i) => query.bind(i as i64),
                                            ValueView::I16(i) => query.bind(i as i64),
                                            ValueView::I32(i) => query.bind(i as i64),
                                            ValueView::I64(i) => query.bind(i),
                                            ValueView::U8(i) => query.bind(i as i64),
                                            ValueView::U16(i) => query.bind(i as i64),
                                            ValueView::U32(i) => query.bind(i as i64),
                                            ValueView::U64(i) => query.bind(i as i64),
                                            ValueView::F32(f) => query.bind(f),
                                            ValueView::F64(f) => query.bind(f),
                                            ValueView::Str(s) => query.bind(s),
                                            ValueView::Bin(b) => query.bind(b),
                                            ValueView::Bool(b) => query.bind(b),
                                            ValueView::Null => query.bind(Option::<&str>::None),
                                            unimplemented => unimplemented!("unimplemented value: {:?}", unimplemented),
                                        };
                                    }
                                    query.execute(&mut *transaction).await?;
                                }
                            },
                            cmd = section_chan.recv().fuse() => {
                                if let Command::Stop = cmd? { return Ok(()) }
                            },
                        }
                    }
                    transaction.commit().await?;
                    message.ack().await;
                }
            }
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

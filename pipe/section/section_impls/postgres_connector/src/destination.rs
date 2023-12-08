use section::{
    command_channel::{Command, SectionChannel},
    section::Section,
    message::{Chunk, ValueView, DataType},
    futures::{self, FutureExt, Sink, Stream, StreamExt},
    SectionError,
    SectionMessage,
    SectionFuture,
};
use std::pin::pin;

use crate::{escape_table_name, generate_schema};
use sqlx::{
    Connection,
    postgres::PgConnectOptions,
    ConnectOptions,
    types::{Json, JsonRawValue},
};
use std::str::FromStr;

#[derive(Debug)]
pub struct Postgres {
    url: String,
}

impl Postgres {
    pub fn new(url: impl Into<String>) -> Self {
        Self { url: url.into() }
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
                    while let Some(chunk) = message.next().await? {
                        let df = match chunk{
                            Chunk::DataFrame(df) => df,
                            _ => Err(format!("expected dataframe chunk"))?
                        };
                        let schema = generate_schema(name.as_str(), df.as_ref())?;
                        let mut columns = df.columns();
                        sqlx::query(&schema).execute(&mut *transaction).await?;
                        let values_placeholder = columns.iter().enumerate()
                            .map(|(pos, col)|  {
                                let suffix = match col.data_type() {
                                    DataType::RawJson => "::json",
                                    DataType::RawJsonB => "::jsonb",
                                    _ => "",
                                };
                                format!("${}{}", pos + 1, suffix)
                            })
                            .collect::<Vec<_>>()
                            .join(",");
                        let insert = format!("INSERT INTO \"{name}\" VALUES({values_placeholder})");
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
                                    ValueView::F32(f) => query.bind(f),
                                    ValueView::F64(f) => query.bind(f),
                                    ValueView::Str(s) => query.bind(s),
                                    ValueView::Bin(b) => query.bind(b),
                                    ValueView::Bool(b) => query.bind(b),
                                    ValueView::Time(t) => query.bind(t),
                                    ValueView::Date(t) => query.bind(t),
                                    ValueView::TimeStamp(t) => query.bind(t),
                                    ValueView::TimeStampTz(t) => query.bind(t),
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

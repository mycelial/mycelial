use futures::{FutureExt, Sink, Stream, StreamExt};
use section::{Command, Section, SectionChannel};
use std::pin::{pin, Pin};

use crate::{escape_table_name, generate_schema, Message, StdError, Value};
use sqlx::Connection;
use sqlx::{postgres::PgConnectOptions, ConnectOptions};
use std::future::Future;
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
    ) -> Result<(), StdError>
    where
        Input: Stream<Item = Message> + Send + 'static,
        Output: Sink<Message, Error = StdError> + Send + 'static,
        SectionChan: SectionChannel + Send + 'static,
    {
        let mut input = pin!(input.fuse());
        let mut _output = pin!(output);

        let connection = &mut PgConnectOptions::from_str(self.url.as_str())?
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
                    let payload = &message.payload;
                    let name = escape_table_name(&message.origin);
                    let schema = generate_schema(&message);
                    sqlx::query(&schema).execute(&mut *connection).await?;
                    let values_placeholder = (0..payload.values.len()).map(|x| {
                        let x = x + 1;
                        format!("${x}")
                    }).collect::<Vec<_>>().join(",");
                    let query = format!("INSERT INTO \"{name}\" VALUES({values_placeholder})");
                    let mut transaction = connection.begin().await?;
                    for row in 0..payload.values[0].len() {
                        let mut q = sqlx::query(&query);
                        for col in 0..payload.values.len() {
                            q = match &payload.values[col][row] {
                                Value::I16(i) => q.bind(i),
                                Value::I32(i) => q.bind(i),
                                Value::I64(i) => q.bind(i),
                                Value::F32(f) => q.bind(f),
                                Value::F64(f) => q.bind(f),
                                Value::Text(t) => q.bind(t),
                                Value::Blob(b) => q.bind(b),
                                Value::Bool(b) => q.bind(b),
                                // FIXME: oof, to insert NULL we need to bind None
                                Value::Null => q.bind(Option::<i64>::None)
                            };
                        };
                        q.execute(&mut *transaction).await?;
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
    Input: Stream<Item = Message> + Send + 'static,
    Output: Sink<Message, Error = StdError> + Send + 'static,
    SectionChan: SectionChannel + Send + 'static,
{
    type Error = StdError;
    type Future = Pin<Box<dyn Future<Output = Result<(), Self::Error>> + Send + 'static>>;

    fn start(self, input: Input, output: Output, command: SectionChan) -> Self::Future {
        Box::pin(async move { self.enter_loop(input, output, command).await })
    }
}

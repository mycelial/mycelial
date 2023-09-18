use std::{pin::{pin, Pin},  sync::Arc};
use futures::{Stream, StreamExt, Sink, FutureExt};
use section::{SectionChannel, Section, Command};

use std::future::Future;
use std::str::FromStr;
use sqlx::Connection;
use sqlx::{
    ConnectOptions,
    sqlite::SqliteConnectOptions
};
use crate::{escape_table_name, Message, StdError, generate_schema, Value};

#[derive(Debug)]
pub struct Sqlite{
    path: String,
}


impl Sqlite {
    pub fn new(path: impl Into<String>) -> Self {
        Self{ path: path.into() }
    }

    async fn enter_loop<Input, Output, SectionChan>(
        self, 
        input: Input,
        output: Output,
        mut section_chan: SectionChan,
    ) -> Result<(), StdError>
        where Input: Stream<Item=Message> + Send + 'static,
              Output: Sink<Message, Error=StdError> + Send + 'static,
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
                    match cmd? {
                        Command::Stop => return Ok(()),
                        _ => {}
                    }
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
                    let values_placeholder = (0..payload.values.len()).map(|_| "?").collect::<Vec<_>>().join(",");
                    let query = format!("INSERT OR IGNORE INTO \"{name}\" VALUES({values_placeholder})");
                    let mut transaction = connection.begin().await?;
                    for row in 0..payload.values[0].len() {
                        let mut q = sqlx::query(&query);
                        for col in 0..payload.values.len() {
                            q = match &payload.values[col][row] {
                                Value::Int(i) => q.bind(i),
                                Value::Real(f) => q.bind(f),
                                Value::Text(t) => q.bind(t),
                                Value::Blob(b) => q.bind(b),
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
        };
    }
}


impl<Input, Output, SectionChan> Section<Input, Output, SectionChan> for Sqlite
    where Input: Stream<Item=Message> + Send + 'static,
          Output: Sink<Message, Error=StdError> + Send + 'static,
          SectionChan: SectionChannel + Send + 'static,
{
    type Error = StdError;
    type Future = Pin<Box<dyn Future<Output = Result<(), Self::Error>> + Send + 'static>>;

    fn start(self, input: Input, output: Output, command: SectionChan) -> Self::Future {
        Box::pin(async move { self.enter_loop(input, output, command).await })
    }
}

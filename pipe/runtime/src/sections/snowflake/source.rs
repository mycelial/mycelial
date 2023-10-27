use futures::{Sink, SinkExt, Stream, StreamExt, FutureExt};
use std::pin::pin;
use std::time::Duration;
use snowflake_api::{SnowflakeApi, QueryResult};
use section::{Section, SectionChannel, Command};
use crate::{
    config::Map,
    message::Message,
    types::{DynSection, SectionError, SectionFuture},
};

pub struct SnowflakeSource {
    username: String,
    password: String,
    role: String,
    account_identifier: String,
    warehouse: String,
    database: String,
    schema: String,
    query: String,
    delay: Duration,
}

impl SnowflakeSource {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        username: impl Into<String>,
        password: impl Into<String>,
        role: impl Into<String>,
        account_identifier: impl Into<String>,
        warehouse: impl Into<String>,
        database: impl Into<String>,
        schema: impl Into<String>,
        query: impl Into<String>,
        delay: Duration,
    ) -> Self {
        Self {
            username: username.into(),
            password: password.into(),
            role: role.into(),
            account_identifier: account_identifier.into(),
            warehouse: warehouse.into(),
            database: database.into(),
            schema: schema.into(),
            query: query.into(),
            delay,
        }
    }

    async fn enter_loop<Input, Output, SectionChan>(
        self,
        input: Input,
        mut output: Output,
        mut section_chan: SectionChan,
    ) -> Result<(), SectionError>
        where
            Input: Stream<Item = Message> + Send + 'static,
            Output: Sink<Message, Error = SectionError> + Send + 'static,
            SectionChan: SectionChannel + Send + 'static,
    {
        // todo: move this before the loop to make it fail early? or verify configuration somehow
        let mut api = SnowflakeApi::with_password_auth(
            &self.account_identifier,
            &self.warehouse,
            Some(&self.database),
            Some(&self.schema),
            &self.username,
            Some(&self.role),
            &self.password,
        )?;

        let mut _input = pin!(input.fuse());
        let mut output = pin!(output);
        let mut tick = pin!(tokio::time::interval(self.delay));
        loop {
            futures::select! {
                cmd = section_chan.recv().fuse() => {
                    if let Command::Stop = cmd? {
                        return Ok(())
                    }
                },
                _ = tick.tick().fuse() => {
                    let query_result = api.exec(&self.query).await?;
                    match query_result {
                        QueryResult::Arrow(batches) => {
                            for batch in batches {
                                let message = Message::new("snowflake_src", batch, None);
                                output.send(message).await?;
                            }
                        }
                        QueryResult::Json(_) => {
                            Err("unexpected payload, expected arrow, got json")?
                        }
                        QueryResult::Empty => {}
                    }
                }
            }
        }
    }
}

impl<Input, Output, SectionChan> Section<Input, Output, SectionChan> for SnowflakeSource
    where
        Input: Stream<Item = Message> + Send + 'static,
        Output: Sink<Message, Error = SectionError> + Send + 'static,
        SectionChan: SectionChannel + Send + 'static,
{
    type Error = SectionError;
    type Future = SectionFuture;

    fn start(self: Self, input: Input, output: Output, section_chan: SectionChan) -> Self::Future {
        Box::pin(async move { self.enter_loop(input, output, section_chan).await })
    }
}


pub fn constructor<S: SectionChannel>(config: &Map) -> Result<Box<dyn DynSection<S>>, SectionError> {
    let username = config
        .get("username")
        .ok_or("username required")?
        .as_str()
        .ok_or("'username' should be a string")?;
    let password = config
        .get("password")
        .ok_or("password required")?
        .as_str()
        .ok_or("'password' should be a string")?;
    let role = config
        .get("role")
        .ok_or("role required")?
        .as_str()
        .ok_or("'role' should be a string")?;
    let account_identifier = config
        .get("account_identifier")
        .ok_or("account_identifier required")?
        .as_str()
        .ok_or("'account_identifier' should be a string")?;
    let warehouse = config
        .get("warehouse")
        .ok_or("warehouse required")?
        .as_str()
        .ok_or("'warehouse' should be a string")?;
    let database = config
        .get("database")
        .ok_or("database required")?
        .as_str()
        .ok_or("'database' should be a string")?;
    let schema = config
        .get("schema")
        .ok_or("schema required")?
        .as_str()
        .ok_or("'schema' should be a string")?;
    let query = config
        .get("query")
        .ok_or("query required")?
        .as_str()
        .ok_or("'query' should be a string")?;
    let delay = config
        .get("delay")
        .ok_or("hello world section requires 'delay'")?
        .as_int()
        .ok_or("'delay' should be an int")?;
    Ok(Box::new(SnowflakeSource::new(
            username,
            password,
            role,
            account_identifier,
            warehouse,
            database,
            schema,
            query,
            Duration::from_secs(delay as u64),
        )
    ))
}

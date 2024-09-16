use arrow_msg::ArrowMsg;
use section::{
    command_channel::{Command, SectionChannel},
    futures::{self, FutureExt, Sink, SinkExt, Stream, StreamExt},
    section::Section,
    SectionError, SectionFuture, SectionMessage,
};
use snowflake_api::{QueryResult, SnowflakeApi};
use std::pin::pin;
use std::time::Duration;

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
        Input: Stream<Item = SectionMessage> + Send + 'static,
        Output: Sink<SectionMessage, Error = SectionError> + Send + 'static,
        SectionChan: SectionChannel + Send + 'static,
    {
        // todo: move this before the loop to make it fail early? or verify configuration somehow
        let api = SnowflakeApi::with_password_auth(
            &self.account_identifier,
            Some(&self.warehouse),
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
                            let batches = batches
                                .into_iter()
                                .map(|batch| Some(batch.into()))
                                .collect::<Vec<_>>();
                            let message = ArrowMsg::new("snowflake_src", batches, None);
                            output.send(Box::new(message)).await?;
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
    Input: Stream<Item = SectionMessage> + Send + 'static,
    Output: Sink<SectionMessage, Error = SectionError> + Send + 'static,
    SectionChan: SectionChannel,
{
    type Error = SectionError;
    type Future = SectionFuture;

    fn start(self, input: Input, output: Output, section_chan: SectionChan) -> Self::Future {
        Box::pin(async move { self.enter_loop(input, output, section_chan).await })
    }
}

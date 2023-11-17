use std::time::Duration;


use crate::message::{Message, RecordBatch};
use futures::SinkExt;
use postgres_connector::source::Postgres;
use section::Section;
use section::SectionChannel;

use crate::types::SectionFuture;
use crate::{
    config::Map,
    types::{DynSection, DynSink, DynStream, SectionError},
};

#[allow(dead_code)]
pub struct PostgresAdapter {
    inner: Postgres,
}

impl<SectionChan: SectionChannel + Send + 'static> Section<DynStream, DynSink, SectionChan>
    for PostgresAdapter
{
    type Future = SectionFuture;
    type Error = SectionError;

    fn start(
        self,
        input: DynStream,
        output: DynSink,
        section_channel: SectionChan,
    ) -> Self::Future {
        Box::pin(async move {
            let output = output.with(|message: postgres_connector::Message| async {
                let payload: RecordBatch = message.payload.try_into()?;
                let message = Message::new(message.origin, payload, message.ack);
                Ok(message)
            });
            self.inner.start(input, output, section_channel).await
        })
    }
}

/// constructor for postgres source
///
/// # Config example:
/// ```toml
/// [[section]]
/// url = "postgres://user:password@host:port/database
/// schema = "public"
/// tables = "foo,bar,baz"
/// poll_interval = 5
/// ```
pub fn constructor<S: SectionChannel>(
    config: &Map,
) -> Result<Box<dyn DynSection<S>>, SectionError> {
    let url = config
        .get("url")
        .ok_or("postgres section requires 'url'")?
        .as_str()
        .ok_or("url should be string")?;
    let schema = config
        .get("schema")
        .ok_or("postgres source requires schema")?
        .as_str()
        .ok_or("schema should be string")?;
    let tables = config
        .get("tables")
        .ok_or("postgres  section requires 'tables'")?
        .as_str()
        .ok_or("'tables' should be string")?
        .split(',')
        .map(|x| x.trim())
        .filter(|x| !x.is_empty())
        .collect::<Vec<&str>>();
    let poll_interval = config
        .get("poll_interval")
        .ok_or("postgres source requires poll interval")?
        .as_int()
        .ok_or("poll interval should be integer")? as u64;

    Ok(Box::new(PostgresAdapter {
        inner: Postgres::new(
            url,
            schema,
            tables.as_slice(),
            Duration::from_secs(poll_interval),
        ),
    }))
}

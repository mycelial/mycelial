use crate::message::Message;
use futures::StreamExt;
use section::Section;
use postgres_connector::destination::Postgres;
use stub::Stub;

use crate::types::SectionFuture;
use crate::{
    config::Map,
    types::{DynSection, DynSink, DynStream, SectionError},
};

use section::SectionChannel;
use super::PostgresPayloadNewType;

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
        _output: DynSink,
        section_channel: SectionChan,
    ) -> Self::Future {
        Box::pin(async move {
            let input = input.map(|message: Message| {
                let sqlite_payload: PostgresPayloadNewType = (&message.payload).into();
                postgres_connector::Message::new(message.origin, sqlite_payload.0, message.ack)
            });
            let output = Stub::<postgres_connector::Message, SectionError>::new();
            self.inner.start(input, output, section_channel).await
        })
    }
}

/// constructor for sqlite destination
///
/// # Config example:
/// ```toml
/// [[section]]
/// url = "postgres://user:password@host:port/database
/// ```
pub fn constructor<S: SectionChannel>(
    config: &Map,
) -> Result<Box<dyn DynSection<S>>, SectionError> {
    let url = config
        .get("url")
        .ok_or("postgres destination section requires 'url'")?
        .as_str()
        .ok_or("path should be string")?;
    Ok(Box::new(PostgresAdapter {
        inner: Postgres::new(url),
    }))
}

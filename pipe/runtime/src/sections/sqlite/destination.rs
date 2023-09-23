use crate::command_channel::SectionChannel;
use crate::message::Message;
use futures::StreamExt;
use section::{Section, State};
use sqlite::destination::Sqlite;
use stub::Stub;

use crate::types::SectionFuture;
use crate::{
    config::Map,
    types::{DynSection, DynSink, DynStream, SectionError},
};

use super::SqlitePayloadNewType;

#[allow(dead_code)]
pub struct SqliteAdapter {
    inner: Sqlite,
}

impl<S: State> Section<DynStream, DynSink, SectionChannel<S>> for SqliteAdapter {
    type Future = SectionFuture;
    type Error = SectionError;

    fn start(
        self,
        input: DynStream,
        _output: DynSink,
        section_channel: SectionChannel<S>,
    ) -> Self::Future {
        Box::pin(async move {
            let input = input.map(|message: Message| {
                let sqlite_payload: SqlitePayloadNewType = (&message.payload).into();
                sqlite::Message::new(message.origin, sqlite_payload.0, message.ack)
            });
            let output = Stub::<sqlite::Message, SectionError>::new();
            self.inner.start(
                input,
                output, 
                section_channel
            ).await
        })
    }
}

/// constructor for sqlite destination
///
/// # Config example:
/// ```toml
/// [[section]]
/// name = "sqlite_destination"
/// path = ":memory:"
/// ```
pub fn constructor<S: State>(config: &Map) -> Result<Box<dyn DynSection<S>>, SectionError> {
    let path = config
        .get("path")
        .ok_or("sqlite section requires 'path'")?
        .as_str()
        .ok_or("path should be string")?;
    Ok(Box::new(SqliteAdapter {
        inner: Sqlite::new(path),
    }))
}

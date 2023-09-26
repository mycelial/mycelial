use crate::command_channel::SectionChannel;
use crate::message::{Message, RecordBatch};
use futures::SinkExt;
use section::{Section, State};
use sqlite_connector::source::Sqlite;

use crate::types::SectionFuture;
use crate::{
    config::Map,
    types::{DynSection, DynSink, DynStream, SectionError},
};

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
        output: DynSink,
        section_channel: SectionChannel<S>,
    ) -> Self::Future {
        Box::pin(async move {
            let output = output.with(|message: sqlite_connector::Message| async {
                let payload: RecordBatch = message.payload.try_into()?;
                let message = Message::new(message.origin, payload, message.ack);
                Ok(message)
            });
            self.inner.start(input, output, section_channel).await
        })
    }
}

/// constructor for sqlite
///
/// # Config example:
/// ```toml
/// [[section]]
/// name = "sqlite"
/// path = ":memory:"
/// tables = "foo,bar,baz"
/// ```
pub fn constructor<S: State>(config: &Map) -> Result<Box<dyn DynSection<S>>, SectionError> {
    let tables = config
        .get("tables")
        .ok_or("sqlite section requires 'tables'")?
        .as_str()
        .ok_or("'tables' should be string")?;
    let path = config
        .get("path")
        .ok_or("sqlite section requires 'path'")?
        .as_str()
        .ok_or("path should be string")?;
    let tables = tables
        .split(',')
        .map(|x| x.trim())
        .filter(|x| !x.is_empty())
        .collect::<Vec<&str>>();
    Ok(Box::new(SqliteAdapter {
        inner: Sqlite::new(path, tables.as_slice()),
    }))
}

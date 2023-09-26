use crate::command_channel::SectionChannel;
use crate::{
    config::Map,
    message,
    types::{DynSection, DynSink, DynStream, SectionError, SectionFuture},
};
use futures::{SinkExt, StreamExt};
use sqlite_physical_replication::source::Source;
use section::{Section, State};

pub struct SourceAdapter {
    inner: Source,
}

impl<S: State> Section<DynStream, DynSink, SectionChannel<S>> for SourceAdapter {
    type Future = SectionFuture;
    type Error = SectionError;

    fn start(
        self,
        input: DynStream,
        output: DynSink,
        command_channel: SectionChannel<S>,
    ) -> Self::Future {
        Box::pin(async move {
            let input = input.map(|msg| sqlite_physical_replication::Message::new(msg.origin, msg.payload.0, msg.ack));
            let output = output.with(|msg: sqlite_physical_replication::Message| async move {
                Ok(message::Message::new(msg.origin, msg.payload, msg.ack))
            });
            self.inner.start(input, output, command_channel).await
        })
    }
}

/// constructor for sqlite_physical_replication 
///
/// # Config example:
/// ```toml
/// [[section]]
/// name = "sqlite_physical_replication_source"
/// journal_path = "/tmp/path_to_journal"
/// ```
pub fn constructor<S: State>(config: &Map) -> Result<Box<dyn DynSection<S>>, SectionError> {
    let path = config
        .get("journal_path")
        .ok_or("sqlite_physical_replication journal path is required")?
        .as_str()
        .ok_or("path should be string")?;
    Ok(Box::new(SourceAdapter {
        inner: Source::new(path),
    }))
}

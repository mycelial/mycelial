use crate::{
    config::Map,
    message,
    types::{DynSection, DynSink, DynStream, SectionError, SectionFuture},
};
use futures::{SinkExt, StreamExt};
use section::{Section, SectionChannel};
use sqlite_physical_replication::destination::Destination;

pub struct DestinationAdapter {
    inner: Destination,
}

impl<S: SectionChannel> Section<DynStream, DynSink, S> for DestinationAdapter {
    type Future = SectionFuture;
    type Error = SectionError;

    fn start(self, input: DynStream, output: DynSink, command_channel: S) -> Self::Future {
        Box::pin(async move {
            // adapt incoming message to sqlite_physical_replication message
            let input = input.map(|msg| {
                sqlite_physical_replication::Message::new(msg.origin, msg.payload.0, msg.ack)
            });
            // adapt outgoing message to pipe message
            let output = output.with(|msg: sqlite_physical_replication::Message| async move {
                Ok(message::Message::new(msg.origin, msg.payload, msg.ack))
            });
            self.inner.start(input, output, command_channel).await
        })
    }
}

/// constructor for sqlite_physical_replication journal destination
///
/// # Config example:
/// ```toml
/// [[section]]
/// name = "sqlite_physical_replication_destination"
/// journal_path = "/tmp/path_to_journal"
/// database_path = "/tmp/path_to_database"
/// ```
pub fn constructor<S: SectionChannel>(
    config: &Map,
) -> Result<Box<dyn DynSection<S>>, SectionError> {
    let path = config
        .get("journal_path")
        .ok_or("sqlite_physical_replication journal path is required")?
        .as_str()
        .ok_or("path should be string")?;
    let database_path = config.get("database_path").map_or(
        Result::<Option<String>, SectionError>::Ok(None),
        |v| {
            v.as_str()
                .ok_or("database path should be string".into())
                .map(|s| {
                    if s.is_empty() {
                        None
                    } else {
                        Some(s.to_string())
                    }
                })
        },
    )?;
    Ok(Box::new(DestinationAdapter {
        inner: Destination::new(path, database_path),
    }))
}

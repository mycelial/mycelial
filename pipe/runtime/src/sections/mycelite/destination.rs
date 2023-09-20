use crate::{
    types::{DynSection, SectionError, SectionFuture, DynStream, DynSink, DynSectionState},
    config::Map,
    message,
};
use futures::{StreamExt, SinkExt};
use mycelite::destination::Mycelite;
use section::Section;
use crate::command_channel::SectionChannel;

pub struct MyceliteAdapter {
    inner: Mycelite
}

impl Section<DynStream, DynSink, SectionChannel<DynSectionState>> for MyceliteAdapter {
    type Future = SectionFuture;
    type Error = SectionError;

    fn start(self, input: DynStream, output: DynSink, command_channel: SectionChannel<DynSectionState>) -> Self::Future {
        Box::pin(async move { 
            // adapt incoming message to mycelite message
            let input = input.map(|msg| {
                mycelite::Message::new(msg.origin, msg.payload.0, msg.ack)
            });
            // adapt outgoing message to pipe message
            let output = output.with(|msg: mycelite::Message| {
                async move { Ok(message::Message::new(msg.origin, msg.payload, msg.ack)) }
            });
            self.inner.start(input, output, command_channel).await
        })
    }
}

/// constructor for mycelite journal destination
///
/// # Config example:
/// ```toml
/// [[section]]
/// name = "mycelite_destination"
/// journal_path = "/tmp/path_to_journal"
/// database_path = "/tmp/path_to_database"
/// ```
pub fn constructor(config: &Map) -> Result<Box<dyn DynSection>, SectionError> {
    let path = config
        .get("journal_path")
        .ok_or("mycelite journal path is required")?
        .as_str()
        .ok_or("path should be string")?;
    let database_path = config
        .get("database_path")
        .map_or(
            Result::<Option<String>, SectionError>::Ok(None),
            |v| v.as_str().ok_or("database path should be string".into())
                .map(|s| if s.is_empty() { None } else { Some(s.to_string()) })
        )?;
    Ok(Box::new(MyceliteAdapter{ inner: Mycelite::new(path, database_path) }))
}

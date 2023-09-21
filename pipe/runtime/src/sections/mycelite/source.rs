use crate::command_channel::SectionChannel;
use crate::{
    config::Map,
    message,
    types::{DynSection, DynSink, DynStream, SectionError, SectionFuture},
};
use futures::{SinkExt, StreamExt};
use mycelite::source::Mycelite;
use section::{Section, State};

pub struct MyceliteAdapter {
    inner: Mycelite,
}

impl<S: State> Section<DynStream, DynSink, SectionChannel<S>> for MyceliteAdapter {
    type Future = SectionFuture;
    type Error = SectionError;

    fn start(
        self,
        input: DynStream,
        output: DynSink,
        command_channel: SectionChannel<S>,
    ) -> Self::Future {
        Box::pin(async move {
            // adapt incoming message to mycelite message
            let input = input.map(|msg| mycelite::Message::new(msg.origin, msg.payload.0, msg.ack));
            // adapt outgoing message to pipe message
            let output = output.with(|msg: mycelite::Message| async move {
                Ok(message::Message::new(msg.origin, msg.payload, msg.ack))
            });
            self.inner.start(input, output, command_channel).await
        })
    }
}

/// constructor for mycelite journal source
///
/// # Config example:
/// ```toml
/// [[section]]
/// name = "mycelite_source"
/// journal_path = "/tmp/path_to_journal"
/// ```
pub fn constructor<S: State>(config: &Map) -> Result<Box<dyn DynSection<S>>, SectionError> {
    let path = config
        .get("journal_path")
        .ok_or("mycelite journal path is required")?
        .as_str()
        .ok_or("path should be string")?;
    Ok(Box::new(MyceliteAdapter {
        inner: Mycelite::new(path),
    }))
}

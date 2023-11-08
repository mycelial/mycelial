use crate::message::Message;
use futures::StreamExt;
use mysql_connector::destination::Mysql;
use section::Section;
use stub::Stub;

use crate::types::SectionFuture;
use crate::{
    config::Map,
    types::{DynSection, DynSink, DynStream, SectionError},
};

use super::MysqlPayloadNewType;
use section::SectionChannel;

#[allow(dead_code)]
pub struct MysqlAdapter {
    inner: Mysql,
}

impl<SectionChan: SectionChannel + Send + 'static> Section<DynStream, DynSink, SectionChan>
    for MysqlAdapter
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
                let mysql_payload: MysqlPayloadNewType = (&message.payload).into();
                mysql_connector::Message::new(message.origin, mysql_payload.0, message.ack)
            });
            let output = Stub::<mysql_connector::Message, SectionError>::new();
            self.inner.start(input, output, section_channel).await
        })
    }
}

/// constructor for mysql destination
///
/// # Config example:
/// ```toml
/// [[section]]
/// url = "mysql://user:password@host:port/database
/// ```
pub fn constructor<S: SectionChannel>(
    config: &Map,
) -> Result<Box<dyn DynSection<S>>, SectionError> {
    let url = config
        .get("url")
        .ok_or("mysql destination section requires 'url'")?
        .as_str()
        .ok_or("path should be string")?;
    Ok(Box::new(MysqlAdapter {
        inner: Mysql::new(url),
    }))
}

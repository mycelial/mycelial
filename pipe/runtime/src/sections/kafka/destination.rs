use crate::message::Message;
use futures::StreamExt;
use kafka::destination::Kafka;
use section::Section;
use stub::Stub;

use crate::types::SectionFuture;
use crate::{
    config::Map,
    types::{DynSection, DynSink, DynStream, SectionError},
};

use section::SectionChannel;

use super::OwnedMessageNewType;

#[allow(dead_code)]
pub struct KafkaAdapter {
    inner: Kafka,
}

impl<SectionChan: SectionChannel + Send + 'static> Section<DynStream, DynSink, SectionChan>
    for KafkaAdapter
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
                let kafka_payload: OwnedMessageNewType = (&message.payload).into();
                kafka::Message::new(message.origin, kafka_payload.0, message.ack)
            });
            let output = Stub::<kafka::Message, SectionError>::new();
            self.inner.start(input, output, section_channel).await
        })
    }
}

/// constructor for kafka destination
pub fn constructor<S: SectionChannel>(
    config: &Map,
) -> Result<Box<dyn DynSection<S>>, SectionError> {
    let brokers = config
        .get("brokers")
        .ok_or("kafka destination section requires 'brokers'")?
        .as_str()
        .ok_or("brokers should be string")?;
    let topic = config
        .get("topic")
        .ok_or("kafka destination section requires 'topic'")?
        .as_str()
        .ok_or("topic should be string")?;
    Ok(Box::new(KafkaAdapter {
        inner: Kafka::new(brokers, topic)?,
    }))
}

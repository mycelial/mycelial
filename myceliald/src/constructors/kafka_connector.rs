use pipe::{config::Map, types::DynSection};
use section::{command_channel::SectionChannel, SectionError};

/// constructor for kafka destination
pub fn destination_ctor<S: SectionChannel>(
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
    Ok(Box::new(kafka_connector::destination::Kafka::new(
        brokers,
        topic,
    )))
}
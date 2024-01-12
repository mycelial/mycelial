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
    // todo: Add an option to represent how the data should be serialized (e.g. json, avro, etc.)
    Ok(Box::new(kafka_connector::destination::Kafka::new(
        brokers, topic,
    )))
}

#[cfg(test)]
mod test {
    use std::collections::HashMap;

    use common::KafkaDestinationConfig;
    use section::dummy::DummySectionChannel;
    use serde_json::Value;

    use super::*;

    // #[test]
    // fn test_source_ctor_matches_config() {
    //     let source_config = KafkaConfig::default();
    //     let mut c: HashMap<String, Value> =
    //         serde_json::from_str(&serde_json::to_string(&source_config).unwrap()).unwrap();

    //     let config: Map = c.drain().map(|(k, v)| (k, v.try_into().unwrap())).collect();

    //     let _section = source_ctor::<DummySectionChannel>(&config).unwrap();
    // }

    #[test]
    fn test_destination_ctor_matches_config() {
        let destination_config = KafkaDestinationConfig::default();
        let mut c: HashMap<String, Value> =
            serde_json::from_str(&serde_json::to_string(&destination_config).unwrap()).unwrap();

        let config: Map = c.drain().map(|(k, v)| (k, v.try_into().unwrap())).collect();

        let _section = destination_ctor::<DummySectionChannel>(&config).unwrap();
    }
}

use pipe::{config::Map, types::DynSection};
use section::{command_channel::SectionChannel, SectionError};

/// # Config example:
/// ```toml
/// [[section]]
/// name = "mycelial_net"
/// endpoint = "http://localhost:7777/ingestion"
/// token = "token"
/// topic = "some_topic"
/// ```
pub fn source_ctor<S: SectionChannel>(
    config: &Map,
) -> Result<Box<dyn DynSection<S>>, SectionError> {
    let endpoint = config
        .get("endpoint")
        .ok_or("mycelial net section requires 'endpoint' url")?
        .as_str()
        .ok_or("endpoint should be string")?;
    let token = config
        .get("token")
        .ok_or("mycelian net section requires 'token'")?
        .as_str()
        .ok_or("token should be string")?;
    // FIXME: validate topic is not empty
    let topic = config
        .get("topic")
        .ok_or("mycelian net section requires 'topic'")?
        .as_str()
        .ok_or("token should be string")?;
    Ok(Box::new(mycelial_server::source::Mycelial::new(
        endpoint, token, topic,
    )))
}

/// # Config example:
/// ```toml
/// [[section]]
/// name = "mycelial_net"
/// endpoint = "http://localhost:7777/ingestion"
/// token = "token"
/// ```
pub fn destination_ctor<S: SectionChannel>(
    config: &Map,
) -> Result<Box<dyn DynSection<S>>, SectionError> {
    let endpoint = config
        .get("endpoint")
        .ok_or("mycelial net section requires 'endpoint' url")?
        .as_str()
        .ok_or("endpoint should be string")?;
    let token = config
        .get("token")
        .ok_or("mycelial net section requires 'token'")?
        .as_str()
        .ok_or("token should be string")?;
    let topic = config
        .get("topic")
        .ok_or("mycelial net section requires 'topic'")?
        .as_str()
        .ok_or("topic should be string")?;
    Ok(Box::new(mycelial_server::destination::Mycelial::new(
        endpoint, token, topic,
    )))
}

// #[cfg(test)]
// mod test {
//     use std::collections::HashMap;

//     use common::SqliteConnectorConfig;
//     use section::dummy::DummySectionChannel;
//     use serde_json::Value;

//     use super::*;

//     #[test]
//     fn test_source_ctor_matches_config() {
//         let source_config = SqliteConnectorConfig::default();
//         let mut c: HashMap<String, Value> =
//             serde_json::from_str(&serde_json::to_string(&source_config).unwrap()).unwrap();

//         let config: Map = c.drain().map(|(k, v)| (k, v.try_into().unwrap())).collect();

//         let _section = source_ctor::<DummySectionChannel>(&config).unwrap();
//     }

//     #[test]
//     fn test_destination_ctor_matches_config() {
//         let destination_config = SqliteConnectorConfig::default();
//         let mut c: HashMap<String, Value> =
//             serde_json::from_str(&serde_json::to_string(&destination_config).unwrap()).unwrap();

//         let config: Map = c.drain().map(|(k, v)| (k, v.try_into().unwrap())).collect();

//         let _section = destination_ctor::<DummySectionChannel>(&config).unwrap();
//     }
// }

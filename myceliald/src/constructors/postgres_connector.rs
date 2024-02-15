use std::time::Duration;

use pipe::{
    config::{Map, Value},
    types::DynSection,
};
use section::{command_channel::SectionChannel, SectionError};

pub fn source_ctor<S: SectionChannel>(
    config: &Map,
) -> Result<Box<dyn DynSection<S>>, SectionError> {
    let url = config
        .get("url")
        .ok_or("postgres section requires 'url'")?
        .as_str()
        .ok_or("url should be string")?;
    let origin = config
        .get("origin")
        .ok_or("postgres section requires 'origin'")?
        .as_str()
        .ok_or("'origin' should be string")?;
    let query = config
        .get("query")
        .ok_or("postgres section requires 'query'")?
        .as_str()
        .ok_or("'query' should be string")?;
    let poll_interval = match config
        .get("poll_interval")
        .ok_or("postgres source requires poll interval")?
    {
        Value::String(v) => v.parse()?,
        Value::Int(i) => (*i) as _,
        _ => Err("poll_interval should be integer")?,
    };
    Ok(Box::new(postgres_connector::source::Postgres::new(
        url,
        origin,
        query,
        Duration::from_secs(poll_interval),
    )))
}

pub fn destination_ctor<S: SectionChannel>(
    config: &Map,
) -> Result<Box<dyn DynSection<S>>, SectionError> {
    let url = config
        .get("url")
        .ok_or("postgres destination section requires 'url'")?
        .as_str()
        .ok_or("path should be string")?;
    let schema = config
        .get("schema")
        .ok_or("postgres source requires schema")?
        .as_str()
        .ok_or("schema should be string")?;
    let truncate = config
        .get("truncate")
        .ok_or("sqlite destination section requires 'truncate'")?;
    let truncate = match truncate {
        Value::Bool(b) => *b,
        Value::String(s) => s.to_lowercase() == "true",
        _ => Err("truncate should be either bool or bool string")?,
    };
    Ok(Box::new(postgres_connector::destination::Postgres::new(
        url, schema, truncate,
    )))
}

#[cfg(test)]
mod test {
    use std::collections::HashMap;

    use common::{PostgresConnectorConfig, PostgresConnectorDestinationConfig};
    use section::dummy::DummySectionChannel;
    use serde_json::Value;

    use super::*;

    #[test]
    fn test_source_ctor_matches_config() {
        let source_config = PostgresConnectorConfig::default();
        let mut c: HashMap<String, Value> =
            serde_json::from_str(&serde_json::to_string(&source_config).unwrap()).unwrap();

        let config: Map = c.drain().map(|(k, v)| (k, v.try_into().unwrap())).collect();

        let _section = source_ctor::<DummySectionChannel>(&config).unwrap();
    }

    #[test]
    fn test_destination_ctor_matches_config() {
        let destination_config = PostgresConnectorDestinationConfig::default();
        let mut c: HashMap<String, Value> =
            serde_json::from_str(&serde_json::to_string(&destination_config).unwrap()).unwrap();

        let config: Map = c.drain().map(|(k, v)| (k, v.try_into().unwrap())).collect();

        let _section = destination_ctor::<DummySectionChannel>(&config).unwrap();
    }
}

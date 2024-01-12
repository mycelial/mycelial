use std::time::Duration;

use pipe::{
    config::{Map, Value},
    types::DynSection,
};
use section::{command_channel::SectionChannel, SectionError};

/// constructor for postgres source
///
/// # Config example:
/// ```toml
/// [[section]]
/// url = "postgres://user:password@host:port/database
/// schema = "public"
/// tables = "foo,bar,baz"
/// poll_interval = 5
/// ```
pub fn source_ctor<S: SectionChannel>(
    config: &Map,
) -> Result<Box<dyn DynSection<S>>, SectionError> {
    let url = config
        .get("url")
        .ok_or("postgres section requires 'url'")?
        .as_str()
        .ok_or("url should be string")?;
    let schema = config
        .get("schema")
        .ok_or("postgres source requires schema")?
        .as_str()
        .ok_or("schema should be string")?;
    let tables = config
        .get("tables")
        .ok_or("postgres  section requires 'tables'")?
        .as_str()
        .ok_or("'tables' should be string")?
        .split(',')
        .map(|x| x.trim())
        .filter(|x| !x.is_empty())
        .collect::<Vec<&str>>();
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
        schema,
        tables.as_slice(),
        Duration::from_secs(poll_interval),
    )))
}

/// constructor for postgres destination
///
/// # Config example:
/// ```toml
/// [[section]]
/// url = "postgres://user:password@host:port/database
/// ```
pub fn destination_ctor<S: SectionChannel>(
    config: &Map,
) -> Result<Box<dyn DynSection<S>>, SectionError> {
    let url = config
        .get("url")
        .ok_or("postgres destination section requires 'url'")?
        .as_str()
        .ok_or("path should be string")?;
    Ok(Box::new(postgres_connector::destination::Postgres::new(
        url,
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

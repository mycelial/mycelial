use pipe::{config::Map, types::DynSection};
use section::{command_channel::SectionChannel, SectionError};

/// constructor for sqlite
///
/// # Config example:
/// ```toml
/// [[section]]
/// name = "sqlite"
/// path = ":memory:"
/// tables = "foo,bar,baz"
/// ```
pub fn source_ctor<S: SectionChannel>(
    config: &Map,
) -> Result<Box<dyn DynSection<S>>, SectionError> {
    let tables = config
        .get("tables")
        .ok_or("sqlite section requires 'tables'")?
        .as_str()
        .ok_or("'tables' should be string")?;
    let path = config
        .get("path")
        .ok_or("sqlite section requires 'path'")?
        .as_str()
        .ok_or("path should be string")?;
    let tables = tables
        .split(',')
        .map(|x| x.trim())
        .filter(|x| !x.is_empty())
        .collect::<Vec<&str>>();
    Ok(Box::new(sqlite_connector::source::Sqlite::new(
        path,
        tables.as_slice(),
    )))
}

pub fn destination_ctor<S: SectionChannel>(
    config: &Map,
) -> Result<Box<dyn DynSection<S>>, SectionError> {
    let path = config
        .get("path")
        .ok_or("sqlite section requires 'path'")?
        .as_str()
        .ok_or("path should be string")?;
    Ok(Box::new(sqlite_connector::destination::Sqlite::new(path)))
}


#[cfg(test)]
mod test {
    use std::collections::HashMap;

    use common::SqliteConnectorConfig;
    use section::dummy::DummySectionChannel;
    use serde_json::Value;

    use super::*;

    #[test]
    fn test_source_ctor_matches_config() {
        let source_config = SqliteConnectorConfig::default();
        let mut c: HashMap<String, Value> =
            serde_json::from_str(&serde_json::to_string(&source_config).unwrap()).unwrap();

        let config: Map = c.drain().map(|(k, v)| (k, v.try_into().unwrap())).collect();

        let _section = source_ctor::<DummySectionChannel>(&config).unwrap();
    }

    #[test]
    fn test_destination_ctor_matches_config() {
        let destination_config = SqliteConnectorConfig::default();
        let mut c: HashMap<String, Value> =
            serde_json::from_str(&serde_json::to_string(&destination_config).unwrap()).unwrap();

        let config: Map = c.drain().map(|(k, v)| (k, v.try_into().unwrap())).collect();

        let _section = destination_ctor::<DummySectionChannel>(&config).unwrap();
    }
}
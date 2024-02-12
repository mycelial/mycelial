use pipe::{
    config::{Map, Value},
    types::DynSection,
};
use section::{command_channel::SectionChannel, SectionError};

pub fn source_ctor<S: SectionChannel>(
    config: &Map,
) -> Result<Box<dyn DynSection<S>>, SectionError> {
    let path = config
        .get("path")
        .ok_or("sqlite section requires 'path'")?
        .as_str()
        .ok_or("path should be string")?;
    let origin = config
        .get("origin")
        .ok_or("sqlite section requires 'origin'")?
        .as_str()
        .ok_or("'origin' should be string")?;
    let query = config
        .get("query")
        .ok_or("sqlite section requires 'query'")?
        .as_str()
        .ok_or("'query' should be string")?;
    Ok(Box::new(sqlite_connector::source::Sqlite::new(
        path, origin, query,
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
    let truncate = config
        .get("truncate")
        .ok_or("sqlite destination section requires 'truncate'")?;
    let truncate = match truncate {
        Value::Bool(b) => *b,
        Value::String(s) => s.to_lowercase() == "true",
        _ => Err("truncate should be either bool or bool string")?,
    };
    Ok(Box::new(sqlite_connector::destination::Sqlite::new(
        path, truncate,
    )))
}

#[cfg(test)]
mod test {
    use std::collections::HashMap;

    use common::{SqliteDestinationConfig, SqliteSourceConfig};
    use section::dummy::DummySectionChannel;
    use serde_json::Value;

    use super::*;

    #[test]
    fn test_source_ctor_matches_config() {
        let source_config = SqliteSourceConfig::default();
        let mut c: HashMap<String, Value> =
            serde_json::from_str(&serde_json::to_string(&source_config).unwrap()).unwrap();

        let config: Map = c.drain().map(|(k, v)| (k, v.try_into().unwrap())).collect();

        assert!(source_ctor::<DummySectionChannel>(&config).is_ok())
    }

    #[test]
    fn test_destination_ctor_matches_config() {
        let destination_config = SqliteDestinationConfig::default();
        let mut c: HashMap<String, Value> =
            serde_json::from_str(&serde_json::to_string(&destination_config).unwrap()).unwrap();

        let config: Map = c.drain().map(|(k, v)| (k, v.try_into().unwrap())).collect();

        assert!(destination_ctor::<DummySectionChannel>(&config).is_ok())
    }
}

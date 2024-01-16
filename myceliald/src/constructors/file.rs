use pipe::{config::Map, types::DynSection};
use section::{command_channel::SectionChannel, SectionError};

pub fn source_ctor<S: SectionChannel>(
    config: &Map,
) -> Result<Box<dyn DynSection<S>>, SectionError> {
    let path = config
        .get("path")
        .ok_or("file section requires 'path'")?
        .as_str()
        .ok_or("'tables' should be string")?;
    Ok(Box::new(file::source::FileSource::new(path)))
}

pub fn destination_ctor<S: SectionChannel>(
    config: &Map,
) -> Result<Box<dyn DynSection<S>>, SectionError> {
    let path = config
        .get("path")
        .ok_or("file section requires 'path'")?
        .as_str()
        .ok_or("path should be string")?;
    Ok(Box::new(file::destination::FileDestination::new(path)))
}

#[cfg(test)]
mod test {
    use std::collections::HashMap;

    use common::{FileDestinationConfig, FileSourceConfig};
    use section::dummy::DummySectionChannel;
    use serde_json::Value;

    use super::*;

    #[test]
    fn test_source_ctor_matches_config() {
        let source_config = FileSourceConfig::default();
        let mut c: HashMap<String, Value> =
            serde_json::from_str(&serde_json::to_string(&source_config).unwrap()).unwrap();

        let config: Map = c.drain().map(|(k, v)| (k, v.try_into().unwrap())).collect();

        assert!(source_ctor::<DummySectionChannel>(&config).is_ok());
    }

    #[test]
    fn test_destination_ctor_matches_config() {
        let destination_config = FileDestinationConfig::default();
        let mut c: HashMap<String, Value> =
            serde_json::from_str(&serde_json::to_string(&destination_config).unwrap()).unwrap();

        let config: Map = c.drain().map(|(k, v)| (k, v.try_into().unwrap())).collect();
        assert!(destination_ctor::<DummySectionChannel>(&config).is_ok());
    }
}

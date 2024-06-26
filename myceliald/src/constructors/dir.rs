use std::time::Duration;

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
        .ok_or("dir section requires 'path'")?
        .as_str()
        .ok_or("'tables' should be string")?;
    let pattern = match config.get("pattern") {
        Some(Value::String(s)) => {
            if s.is_empty() {
                None
            } else {
                Some(s.clone())
            }
        }
        Some(_other) => Err("pattern should be string")?,
        None => None,
    };
    let start_after = match config.get("start_after") {
        Some(Value::String(s)) => {
            if s.is_empty() {
                None
            } else {
                Some(s.clone())
            }
        }
        Some(_) => Err("pattern should be string")?,
        None => None,
    };
    let interval = match config
        .get("interval")
        .ok_or("dir source requires interval")?
    {
        Value::String(v) => v.parse()?,
        Value::Int(i) => (*i) as _,
        _ => Err("poll_interval should be integer")?,
    };
    let stream_binary = match config.get("stream_binary").unwrap_or(&Value::Bool(false)) {
        Value::String(maybe_bool) => maybe_bool.to_lowercase() == "true",
        Value::Bool(b) => *b,
        _ => Err("'stream_binary' must be a bool")?,
    };
    Ok(Box::new(dir::source::DirSource::new(
        path.into(),
        pattern,
        start_after,
        Duration::from_secs(interval),
        stream_binary,
    )?))
}

#[cfg(test)]
mod test {
    use std::collections::HashMap;

    use common::DirSourceConfig;
    use section::dummy::DummySectionChannel;
    use serde_json::Value;

    use super::*;

    #[test]
    fn test_source_ctor_matches_config() {
        let source_config = DirSourceConfig::default();
        let mut c: HashMap<String, Value> =
            serde_json::from_str(&serde_json::to_string(&source_config).unwrap()).unwrap();
        println!("c: {c:?}");

        let config: Map = c.drain().map(|(k, v)| (k, v.try_into().unwrap())).collect();

        assert!(source_ctor::<DummySectionChannel>(&config).is_ok());
    }
}

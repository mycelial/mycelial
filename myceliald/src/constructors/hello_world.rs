use pipe::{config::Map, types::DynSection};
use section::{command_channel::SectionChannel, SectionError};

pub fn source_ctor<S: SectionChannel>(
    config: &Map,
) -> Result<Box<dyn DynSection<S>>, SectionError> {
    let message = config
        .get("message")
        .ok_or("hello world section requires 'message'")?
        .as_str()
        .ok_or("'message' should be a string")?;
    let interval_milis = config
        .get("interval_milis")
        .ok_or("hello world section requires 'interval_milis'")?
        .as_int()
        .ok_or("'interval_milis' should be an int")?;
    Ok(Box::new(hello_world::source::HelloWorld::new(
        message,
        interval_milis,
    )))
}

pub fn destination_ctor<S: SectionChannel>(
    _: &Map,
) -> Result<Box<dyn DynSection<S>>, SectionError> {
    Ok(Box::new(hello_world::destination::HelloWorld::new()))
}

#[cfg(test)]
mod test {
    use std::collections::HashMap;

    use common::HelloWorldSourceConfig;
    use section::dummy::DummySectionChannel;
    use serde_json::Value;

    use super::*;

    #[test]
    fn test_source_ctor() {
        let source_config = HelloWorldSourceConfig::default();
        let mut c: HashMap<String, Value> =
            serde_json::from_str(&serde_json::to_string(&source_config).unwrap()).unwrap();

        let config: Map = c.drain().map(|(k, v)| (k, v.try_into().unwrap())).collect();

        let _section = source_ctor::<DummySectionChannel>(&config).unwrap();
    }
}

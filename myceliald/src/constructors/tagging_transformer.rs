use pipe::{config::Map, types::DynSection};
use section::{command_channel::SectionChannel, SectionError};

pub fn transform_ctor<S: SectionChannel>(
    config: &Map,
) -> Result<Box<dyn DynSection<S>>, SectionError> {
    let column = config
        .get("column")
        .ok_or("tagging requires a 'column' name")?
        .as_str()
        .ok_or("tagging 'column' name must be a string")?;
    let text = config
        .get("text")
        .ok_or("tagging requires a 'text'")?
        .as_str()
        .ok_or("tagging 'text' must be a string")?;
    Ok(Box::new(tagging_transformer::TaggingTransformer::new(
        column, text,
    )))
}


#[cfg(test)]
mod test {
    use std::collections::HashMap;

    use common::TaggingTransformerConfig;
    use section::dummy::DummySectionChannel;
    use serde_json::Value;

    use super::*;

    #[test]
    fn test_ctor_matches_config() {
        let source_config = TaggingTransformerConfig::default();
        let mut c: HashMap<String, Value> =
            serde_json::from_str(&serde_json::to_string(&source_config).unwrap()).unwrap();

        let config: Map = c.drain().map(|(k, v)| (k, v.try_into().unwrap())).collect();

        let _section = transform_ctor::<DummySectionChannel>(&config).unwrap();
    }
}
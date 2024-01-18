use pipe::{config::Map, types::DynSection};
use section::{command_channel::SectionChannel, SectionError};

pub fn transformer<S: SectionChannel>(
    config: &Map,
) -> Result<Box<dyn DynSection<S>>, SectionError> {
    let column = config
        .get("column")
        .ok_or("tagging requires a 'column' name")?
        .as_str()
        .ok_or("tagging 'column' name must be a string")?;
    let target_type = match config
        .get("target_type")
        .ok_or("typecasting requires a 'target_type'")?
        .as_str()
        .ok_or("tagging 'text' must be set")?
    {
        "string" => sqlite_typecast::Type::String,
        "int" => sqlite_typecast::Type::Int,
        "float" => sqlite_typecast::Type::Float,
        _ => return Err("target type must be string, int, or float")?,
    };
    Ok(Box::new(sqlite_typecast::SqliteTypecast::new(
        target_type,
        column,
    )))
}

#[cfg(test)]
mod test {
    use std::collections::HashMap;

    use common::SqliteTypeCast;
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

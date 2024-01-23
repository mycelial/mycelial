use pipe::{config::Map, types::DynSection};
use section::{command_channel::SectionChannel, SectionError};

pub fn source_ctor<S: SectionChannel>(
    config: &Map,
) -> Result<Box<dyn DynSection<S>>, SectionError> {
    let sheets = config
        .get("sheets")
        .ok_or("excel section requires 'sheets'")?
        .as_str()
        .ok_or("'sheets' should be string")?;
    let sheets = sheets
        .split(',')
        .map(|x| x.trim())
        .filter(|x| !x.is_empty())
        .collect::<Vec<&str>>();
    let path = config
        .get("path")
        .ok_or("excel section requires 'path'")?
        .as_str()
        .ok_or("path should be string")?;
    // FIXME: naming
    // If excel is strict - DataType::Any will be used, otherwise each cell value will be converted
    // to string
    let stringify = config.get("strict")
        .ok_or("excel section requires 'strict' flag")?
        .as_str()
        .ok_or("strict flag should be string")?;

    Ok(Box::new(excel_connector::source::Excel::new(
        path,
        sheets.as_slice(),
        stringify.to_lowercase() != "true",
    )))
}

#[cfg(test)]
mod test {
    use std::collections::HashMap;

    use common::ExcelConfig;
    use section::dummy::DummySectionChannel;
    use serde_json::Value;

    use super::*;

    #[test]
    fn test_source_ctor_matches_config() {
        let source_config: ExcelConfig = ExcelConfig::default();
        let mut c: HashMap<String, Value> =
            serde_json::from_str(&serde_json::to_string(&source_config).unwrap()).unwrap();

        let config: Map = c.drain().map(|(k, v)| (k, v.try_into().unwrap())).collect();

        let _section = source_ctor::<DummySectionChannel>(&config).unwrap();
    }
}

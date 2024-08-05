use pipe::{
    config::{Map, Value},
    types::DynSection,
};
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
    let stringify = match config
        .get("strict")
        .ok_or("excel section requires 'strict' flag")?
    {
        Value::String(v) => Ok(v.to_lowercase() != "true"),
        Value::Bool(b) => Ok(!(*b)),
        _ => Err("strict should be boolean-ish"),
    }?;

    Ok(Box::new(excel_connector::source::Excel::new(
        path,
        sheets.as_slice(),
        stringify,
    )))
}

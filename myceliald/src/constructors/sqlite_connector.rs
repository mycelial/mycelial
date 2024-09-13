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

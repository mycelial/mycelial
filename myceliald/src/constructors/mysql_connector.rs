use std::time::Duration;

use pipe::{
    config::{Map, Value},
    types::DynSection,
};
use section::{command_channel::SectionChannel, SectionError};

pub fn source_ctor<S: SectionChannel>(
    config: &Map,
) -> Result<Box<dyn DynSection<S>>, SectionError> {
    let url = config
        .get("url")
        .ok_or("mysql section requires 'url'")?
        .as_str()
        .ok_or("url should be string")?;
    let origin = config
        .get("origin")
        .ok_or("mysql source requires 'origin'")?
        .as_str()
        .ok_or("'origin' should be string")?;
    let query = config
        .get("query")
        .ok_or("mysql  section requires 'query'")?
        .as_str()
        .ok_or("'query' should be string")?;
    let poll_interval = match config
        .get("poll_interval")
        .ok_or("mysql source requires poll interval")?
    {
        Value::String(v) => v.parse()?,
        Value::Int(i) => (*i) as _,
        _ => Err("poll_interval should be integer")?,
    };
    Ok(Box::new(mysql_connector::source::Mysql::new(
        url,
        origin,
        query,
        Duration::from_secs(poll_interval),
    )))
}

pub fn destination_ctor<S: SectionChannel>(
    config: &Map,
) -> Result<Box<dyn DynSection<S>>, SectionError> {
    let url = config
        .get("url")
        .ok_or("mysql destination section requires 'url'")?
        .as_str()
        .ok_or("path should be string")?;
    let truncate = config
        .get("truncate")
        .ok_or("mysql destination section requires 'truncate'")?;
    let truncate = match truncate {
        Value::Bool(b) => *b,
        Value::String(s) => s.to_lowercase() == "true",
        _ => Err("truncate should be either bool or bool string")?,
    };
    Ok(Box::new(mysql_connector::destination::Mysql::new(
        url, truncate,
    )))
}

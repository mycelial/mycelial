use std::time::Duration;

use pipe::{
    config::{Map, Value},
    types::DynSection,
};
use section::{command_channel::SectionChannel, SectionError};

/// constructor for mysql source
///
/// # Config example:
/// ```toml
/// [[section]]
/// url = "mysql://user:password@host:port/database
/// schema = "public"
/// tables = "foo,bar,baz"
/// poll_interval = 5
/// ```
pub fn source_ctor<S: SectionChannel>(
    config: &Map,
) -> Result<Box<dyn DynSection<S>>, SectionError> {
    let url = config
        .get("url")
        .ok_or("mysql section requires 'url'")?
        .as_str()
        .ok_or("url should be string")?;
    let schema = config
        .get("schema")
        .ok_or("mysql source requires schema")?
        .as_str()
        .ok_or("schema should be string")?;
    let tables = config
        .get("tables")
        .ok_or("mysql  section requires 'tables'")?
        .as_str()
        .ok_or("'tables' should be string")?
        .split(',')
        .map(|x| x.trim())
        .filter(|x| !x.is_empty())
        .collect::<Vec<&str>>();
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
        schema,
        tables.as_slice(),
        Duration::from_secs(poll_interval),
    )))
}

/// constructor for mysql destination
///
/// # Config example:
/// ```toml
/// [[section]]
/// url = "mysql://user:password@host:port/database
/// ```
pub fn destination_ctor<S: SectionChannel>(
    config: &Map,
) -> Result<Box<dyn DynSection<S>>, SectionError> {
    let url = config
        .get("url")
        .ok_or("mysql destination section requires 'url'")?
        .as_str()
        .ok_or("path should be string")?;
    Ok(Box::new(mysql_connector::destination::Mysql::new(
        url,
    )))
}

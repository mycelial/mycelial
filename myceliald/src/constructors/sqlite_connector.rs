use pipe::{config::Map, types::DynSection};
use section::{command_channel::SectionChannel, SectionError};

/// constructor for sqlite
///
/// # Config example:
/// ```toml
/// [[section]]
/// name = "sqlite"
/// path = ":memory:"
/// tables = "foo,bar,baz"
/// ```
pub fn source_ctor<S: SectionChannel>(
    config: &Map,
) -> Result<Box<dyn DynSection<S>>, SectionError> {
    let tables = config
        .get("tables")
        .ok_or("sqlite section requires 'tables'")?
        .as_str()
        .ok_or("'tables' should be string")?;
    let path = config
        .get("path")
        .ok_or("sqlite section requires 'path'")?
        .as_str()
        .ok_or("path should be string")?;
    let tables = tables
        .split(',')
        .map(|x| x.trim())
        .filter(|x| !x.is_empty())
        .collect::<Vec<&str>>();
    Ok(Box::new(sqlite_connector::source::Sqlite::new(
        path,
        tables.as_slice(),
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
    Ok(Box::new(sqlite_connector::destination::Sqlite::new(path)))
}

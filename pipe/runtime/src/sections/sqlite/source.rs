use section::{Section, State};
use sqlite::source::Sqlite;
use crate::command_channel::SectionChannel;

use crate::types::SectionFuture;
use crate::{
    config::Map,
    types::{SectionError, DynSection, DynStream, DynSink},
};

pub struct SqliteAdapter {
    inner: Sqlite,
}

impl<S: State> Section<DynStream, DynSink, SectionChannel<S>> for SqliteAdapter {
    type Future = SectionFuture;
    type Error = SectionError;

    fn start(self, _input: DynStream, _output: DynSink, _section_channel: SectionChannel<S>) -> Self::Future {
        unimplemented!()
    }
}


/// constructor for sqlite
///
/// # Config example:
/// ```toml
/// [[section]]
/// name = "sqlite"
/// path = ":memory:"
/// tables = "foo,bar,baz"
/// ```
pub fn constructor<S: State>(config: &Map) -> Result<Box<dyn DynSection<S>>, SectionError> {
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
    let tables = tables.split(',').map(|x| x.trim()).filter(|x| !x.is_empty()).collect::<Vec<&str>>();
    Ok(Box::new(
        SqliteAdapter{inner: Sqlite::new(path, tables.as_slice())}
    ))
}

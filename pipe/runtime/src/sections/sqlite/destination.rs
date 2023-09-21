use section::{Section, State};
use sqlite::destination::Sqlite;
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

/// constructor for sqlite destination
///
/// # Config example:
/// ```toml
/// [[section]]
/// name = "sqlite_destination"
/// path = ":memory:"
/// ```
pub fn constructor<S: State>(config: &Map) -> Result<Box<dyn DynSection<S>>, SectionError> {
    let path = config
        .get("path")
        .ok_or("sqlite section requires 'path'")?
        .as_str()
        .ok_or("path should be string")?;
    Ok(Box::new(SqliteAdapter{inner: Sqlite::new(path)}))
}

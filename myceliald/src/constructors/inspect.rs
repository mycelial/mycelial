use pipe::{config::Map, types::DynSection};
use section::prelude::*;

pub fn inspect_ctor<S: SectionChannel>(
    _config: &Map,
) -> Result<Box<dyn DynSection<S>>, SectionError> {
    Ok(Box::new(inspect::Inspect::default()))
}

use pipe::{config::Map, types::DynSection};
use section::{command_channel::SectionChannel, SectionError};

pub fn source_ctor<S: SectionChannel>(
    config: &Map,
) -> Result<Box<dyn DynSection<S>>, SectionError> {
    let path = config
        .get("path")
        .ok_or("file section requires 'path'")?
        .as_str()
        .ok_or("'tables' should be string")?;
    Ok(Box::new(file::source::FileSource::new(path)))
}

pub fn destination_ctor<S: SectionChannel>(
    config: &Map,
) -> Result<Box<dyn DynSection<S>>, SectionError> {
    let dir_path = config
        .get("dir_path")
        .ok_or("file section requires 'dir_path'")?
        .as_str()
        .ok_or("dir_path should be string")?;
    Ok(Box::new(file::destination::FileDestination::new(dir_path)))
}

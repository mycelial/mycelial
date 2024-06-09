use pipe::{config::Map, types::DynSection};
use section::{command_channel::SectionChannel, SectionError};

pub fn source_ctor<S: SectionChannel>(
    config: &Map,
) -> Result<Box<dyn DynSection<S>>, SectionError> {
    let batch_size = config
        .get("batch_size")
        .ok_or("csv_transform section requires 'batch_size'")?
        .as_int()
        .unwrap_or(512);
    Ok(Box::new(csv_transform::source::FromCsv::new(
        batch_size.try_into()?,
    )))
}

pub fn destination_ctor<S: SectionChannel>(
    _config: &Map,
) -> Result<Box<dyn DynSection<S>>, SectionError> {
    Ok(Box::new(csv_transform::destination::ToCsv::new(4096)))
}

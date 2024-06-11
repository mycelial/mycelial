use pipe::{config::Map, types::DynSection};
use section::{command_channel::SectionChannel, SectionError};

pub fn regex_ctor<S: SectionChannel>(config: &Map) -> Result<Box<dyn DynSection<S>>, SectionError> {
    let regex = config
        .get("regex")
        .ok_or("origin transform section requires 'regex'")?
        .as_str()
        .ok_or("'regex' must be string")?;
    let replacement = config
        .get("replacement")
        .ok_or("origin transform section requires 'replacement'")?
        .as_str()
        .ok_or("'replacement' must be string")?;
    Ok(Box::new(origin_transform::regex::OriginTransform::new(
        regex,
        replacement,
    )?))
}

pub fn time_nanos_ctor<S: SectionChannel>(
    config: &Map,
) -> Result<Box<dyn DynSection<S>>, SectionError> {
    let regex = config
        .get("regex")
        .ok_or("origin transform section requires 'regex'")?
        .as_str()
        .ok_or("'regex' must be string")?;
    Ok(Box::new(
        origin_transform::time_nanos::OriginTransform::new(regex)?,
    ))
}

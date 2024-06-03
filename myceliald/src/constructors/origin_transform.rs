use pipe::{config::Map, types::DynSection};
use section::{command_channel::SectionChannel, SectionError};

pub fn ctor<S: SectionChannel>(config: &Map) -> Result<Box<dyn DynSection<S>>, SectionError> {
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
    Ok(Box::new(origin_transform::OriginTransform::new(
        regex,
        replacement,
    )?))
}

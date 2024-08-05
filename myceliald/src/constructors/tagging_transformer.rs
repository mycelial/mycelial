use pipe::{config::Map, types::DynSection};
use section::{command_channel::SectionChannel, SectionError};

pub fn transform_ctor<S: SectionChannel>(
    config: &Map,
) -> Result<Box<dyn DynSection<S>>, SectionError> {
    let column = config
        .get("column")
        .ok_or("tagging requires a 'column' name")?
        .as_str()
        .ok_or("tagging 'column' name must be a string")?;
    let text = config
        .get("text")
        .ok_or("tagging requires a 'text'")?
        .as_str()
        .ok_or("tagging 'text' must be a string")?;
    Ok(Box::new(tagging_transformer::TaggingTransformer::new(
        column, text,
    )))
}

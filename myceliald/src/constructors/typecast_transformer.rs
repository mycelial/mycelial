use pipe::{config::Map, types::DynSection};
use section::{command_channel::SectionChannel, SectionError};

pub fn transformer<S: SectionChannel>(
    config: &Map,
) -> Result<Box<dyn DynSection<S>>, SectionError> {
    let target_column = config
        .get("column")
        .ok_or("typecasting requires a 'column' name")?
        .as_str()
        .ok_or("typecasted 'column' name must be a string")?;
    let target_type = match config
        .get("target_type")
        .ok_or("typecasting requires a 'target_type'")?
        .as_str()
        .ok_or("typecasting 'target_type' must be set")?
    {
        "string" => "string",
        "int" => "int",
        "real" => "real",
        _ => return Err("target type must be string, int, or real")?,
    };
    let section = typecast_transformer::TypecastTransformer::new(target_type, target_column)?;
    Ok(Box::new(section))
}

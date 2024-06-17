use pipe::{
    config::{Map, Value},
    types::DynSection,
};
use section::{command_channel::SectionChannel, SectionError};

pub fn ctor<S: SectionChannel>(config: &Map) -> Result<Box<dyn DynSection<S>>, SectionError> {
    let database_url = config
        .get("database_url")
        .ok_or("redshift loader requires 'database_url'")?
        .as_str()
        .ok_or("'database_url' must be string")?;
    let iam_role = config
        .get("iam_role")
        .ok_or("redshift loader requires 'iam_role'")?
        .as_str()
        .ok_or("'iam_role' must be string")?;
    let region = config
        .get("region")
        .ok_or("redshift loader requires 'region'")?
        .as_str()
        .ok_or("'region' must be string")?;
    let data_format = config
        .get("data_format")
        .ok_or("redshift loader requires 'data_format'")?
        .as_str()
        .ok_or("'data_format' must be string")?;
    let ignore_header = match config.get("ignore_header").unwrap_or(&Value::Bool(false)) {
        Value::String(maybe_bool) => maybe_bool.to_lowercase() == "true",
        Value::Bool(b) => *b,
        _ => Err("'stream_binary' must be a bool")?,
    };
    Ok(Box::new(redshift_loader::RedshiftLoader::new(
        database_url,
        iam_role,
        region,
        data_format,
        ignore_header,
    )))
}

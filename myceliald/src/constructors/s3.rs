use std::time::Duration;

use pipe::{config::{Map, Value}, types::DynSection};
use section::{command_channel::SectionChannel, SectionError};


pub fn source_ctor<S: SectionChannel>(
    config: &Map,
) -> Result<Box<dyn DynSection<S>>, SectionError> {
    let region = config
        .get("region")
        .ok_or("s3 connector source requires 'region'")?
        .as_str()
        .ok_or("'region' must be string")?;
    let bucket = config
        .get("bucket")
        .ok_or("s3 connector source requires 'bucket'")?
        .as_str()
        .ok_or("'bucket' must be string")?;
    let access_key_id = config
        .get("access_key_id")
        .ok_or("s3 connector source requires 'access_key_id'")?
        .as_str()
        .ok_or("'access_key_id' must be string")?;
    let secret_key = config
        .get("secret_key")
        .ok_or("s3 connector source requires 'secret_key'")?
        .as_str()
        .ok_or("'secret_key' must be string")?;
    let stream_binary = match config.get("stream_binary").unwrap_or(&Value::Bool(false)) {
        Value::String(maybe_bool) => maybe_bool.to_lowercase() == "true",
        Value::Bool(b) => *b,
        _ => Err("'stream_binary' must be a bool")?,
    };
    let interval = match config
        .get("interval")
        .ok_or("s3 source requires interval")?
    {
        Value::String(v) => v.parse()?,
        Value::Int(i) => (*i) as _,
        _ => Err("interval should be integer")?,
    };
    let interval = Duration::from_secs(interval);
    let start_after = match config.get("start_after") {
        Some(Value::String(s)) => {
            if s.is_empty() {
                None
            } else {
                Some(s.clone())
            }
        }
        Some(_) => Err("pattern should be string")?,
        None => None,
    };
    Ok(Box::new(s3::source::S3Source::new(
        bucket,
        region,
        access_key_id,
        secret_key,
        stream_binary,
        start_after,
        interval,
    )?))
}

pub fn destination_ctor<S: SectionChannel>(
    config: &Map,
) -> Result<Box<dyn DynSection<S>>, SectionError> {
    let region = config
        .get("region")
        .ok_or("s3 connector source requires 'region'")?
        .as_str()
        .ok_or("'region' must be string")?;
    let bucket = config
        .get("bucket")
        .ok_or("s3 connector source requires 'bucket'")?
        .as_str()
        .ok_or("'bucket' must be string")?;
    let access_key_id = config
        .get("access_key_id")
        .ok_or("s3 connector source requires 'access_key_id'")?
        .as_str()
        .ok_or("'access_key_id' must be string")?;
    let secret_key = config
        .get("secret_key")
        .ok_or("s3 connector source requires 'secret_key'")?
        .as_str()
        .ok_or("'secret_key' must be string")?;
    Ok(Box::new(s3::destination::S3Destination::new(
        bucket,
        region,
        access_key_id,
        secret_key,
    )?))
}

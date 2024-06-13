use pipe::{config::Map, types::DynSection};
use section::{command_channel::SectionChannel, SectionError};

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

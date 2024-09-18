#[cfg(feature = "section")]
pub mod destination;
#[cfg(feature = "section")]
pub mod source;
#[cfg(feature = "section")]
pub(crate) mod static_credentials_provider;

#[cfg(feature = "section")]
pub(crate) type Result<T, E = section::SectionError> = std::result::Result<T, E>;

#[derive(Debug, Clone, config::Configuration)]
#[section(output=bin_or_dataframe)]
pub struct S3Source {
    endpoint: String,
    bucket: String,
    region: String,
    access_key_id: String,
    #[field_type(password)]
    secret_key: String,
    stream_binary: bool,
    start_after: String,
    interval: u64,
}

impl S3Source {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        endpoint: impl Into<String>,
        bucket: impl Into<String>,
        region: impl Into<String>,
        access_key_id: impl Into<String>,
        secret_key: impl Into<String>,
        stream_binary: bool,
        start_after: impl Into<String>,
        interval: u64,
    ) -> Self {
        Self {
            endpoint: endpoint.into(),
            bucket: bucket.into(),
            region: region.into(),
            access_key_id: access_key_id.into(),
            secret_key: secret_key.into(),
            stream_binary,
            start_after: start_after.into(),
            interval,
        }
    }
}

impl Default for S3Source {
    fn default() -> Self {
        Self::new(
            "",
            "s3://some_bucket/",
            "us-east-1",
            "access_key_id",
            "",
            false,
            "",
            30,
        )
    }
}

#[derive(Debug, Clone, config::Configuration)]
#[section(input=bin)]
pub struct S3Destination {
    endpoint: String,
    bucket: String,
    region: String,
    access_key_id: String,
    secret_key: String,
    max_upload_part_size: usize,
}

impl S3Destination {
    pub fn new(
        endpoint: impl Into<String>,
        bucket: impl Into<String>,
        region: impl Into<String>,
        access_key_id: impl Into<String>,
        secret_key: impl Into<String>,
        max_upload_part_size: usize,
    ) -> Self {
        Self {
            endpoint: endpoint.into(),
            bucket: bucket.into(),
            region: region.into(),
            access_key_id: access_key_id.into(),
            secret_key: secret_key.into(),
            max_upload_part_size,
        }
    }
}

impl Default for S3Destination {
    fn default() -> Self {
        Self::new(
            "",
            "s3://some-bucket/",
            "us-east-1",
            "access-key-id",
            "",
            1 << 23, // 8 MB
        )
    }
}

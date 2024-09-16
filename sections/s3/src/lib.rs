use aws_credential_types::{
    provider::future::ProvideCredentials as ProvideCredentialsFuture, Credentials,
};
use aws_sdk_s3::config::ProvideCredentials;
use section::SectionError;

pub mod destination;
pub mod source;

pub(crate) type Result<T, E = SectionError> = std::result::Result<T, E>;

#[derive(Debug)]
pub(crate) struct StaticCredentialsProvider {
    pub access_key_id: String,
    pub secret_key: String,
}

impl StaticCredentialsProvider {
    pub fn new(access_key_id: String, secret_key: String) -> Self {
        Self {
            access_key_id,
            secret_key,
        }
    }
}

impl ProvideCredentials for StaticCredentialsProvider {
    fn provide_credentials<'a>(&'a self) -> ProvideCredentialsFuture<'a>
    where
        Self: 'a,
    {
        let credentials = Credentials::new(
            &self.access_key_id,
            &self.secret_key,
            None,
            None,
            "StaticCredentials",
        );
        ProvideCredentialsFuture::new(async move { Ok(credentials) })
    }
}

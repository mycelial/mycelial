//! list or stream files from s3 bucket
use std::{pin::pin, sync::Arc, time::Duration};

use crate::{static_credentials_provider::StaticCredentialsProvider, Result, S3Source};
use aws_config::{BehaviorVersion, Region, SdkConfig};
use aws_sdk_s3::{config::SharedCredentialsProvider, Client};
use section::prelude::*;

#[derive(Debug)]
pub struct S3SourceInner {
    endpoint: Option<url::Url>,
    bucket: url::Url,
    region: String,
    access_key_id: String,
    secret_key: String,
    stream_binary: bool,
    start_after: String,
    interval: Duration,
}

impl TryFrom<S3Source> for S3SourceInner {
    type Error = SectionError;

    fn try_from(value: S3Source) -> std::result::Result<Self, Self::Error> {
        Self::new(
            value.endpoint.as_str(),
            value.bucket.as_str(),
            value.region,
            value.access_key_id,
            value.secret_key,
            value.stream_binary,
            value.start_after,
            Duration::from_secs(value.interval),
        )
    }
}

impl S3SourceInner {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        endpoint: impl AsRef<str>,
        bucket: impl AsRef<str>,
        region: impl Into<String>,
        access_key_id: impl Into<String>,
        secret_key: impl Into<String>,
        stream_binary: bool,
        start_after: impl Into<String>,
        interval: Duration,
    ) -> Result<Self> {
        let endpoint = match endpoint.as_ref() {
            "" => None,
            endpoint => Some(endpoint.parse()?),
        };
        let bucket = url::Url::try_from(bucket.as_ref())?;
        if bucket.scheme() != "s3" {
            Err("expected url with 's3' schema")?
        };
        let region = region.into();
        let start_after = start_after.into();
        let access_key_id = access_key_id.into();
        let secret_key = secret_key.into();
        Ok(Self {
            endpoint,
            bucket,
            region,
            stream_binary,
            start_after,
            interval,
            access_key_id,
            secret_key,
        })
    }

    async fn handle_command<S: SectionChannel>(
        &self,
        cmd: Command,
        state: &mut S::State,
        section_channel: &mut S,
    ) -> Result<HandleCommandResult> {
        match cmd {
            Command::Stop => Ok(HandleCommandResult::Stop),
            Command::Ack(ack) => match ack.downcast::<String>() {
                Ok(path) => {
                    tracing::debug!("setting start after to {path}");
                    state.set(START_AFTER_KEY, path.to_string())?;
                    section_channel.store_state(state.clone()).await?;
                    Ok(HandleCommandResult::Ok)
                }
                Err(_) => Err("failed to downcast Ack message")?,
            },
            _ => Ok(HandleCommandResult::Ok),
        }
    }
}

enum HandleCommandResult {
    Ok,
    Stop,
}

#[derive(Debug)]
struct S3Object {
    // full path to s3 object
    path: Arc<str>,
}

impl S3Object {
    fn new(path: Arc<str>) -> Self {
        Self { path }
    }
}

impl DataFrame for S3Object {
    fn columns(&self) -> Vec<Column<'_>> {
        vec![Column::new(
            "path",
            DataType::Str,
            Box::new(std::iter::once(ValueView::Str(&self.path))),
        )]
    }
}

struct S3Message {
    origin: Arc<str>,
    inner: Option<Box<dyn DataFrame>>,
    ack: Option<Ack>,
}

impl S3Message {
    fn new(origin: Arc<str>, inner: Box<dyn DataFrame>, ack: Ack) -> Self {
        Self {
            origin,
            inner: Some(inner),
            ack: Some(ack),
        }
    }
}

impl std::fmt::Debug for S3Message {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("S3Message")
            .field("origin", &self.origin)
            .field("inner", &self.inner)
            .finish()
    }
}

impl Message for S3Message {
    fn ack(&mut self) -> Ack {
        self.ack.take().unwrap_or(Box::pin(async {}))
    }

    fn origin(&self) -> &str {
        &self.origin
    }

    fn next(&mut self) -> Next<'_> {
        let payload = self.inner.take().map(Chunk::DataFrame);
        Box::pin(async move { Ok(payload) })
    }
}

const START_AFTER_KEY: &str = "start_after";

impl<Input, Output, SectionChan> Section<Input, Output, SectionChan> for S3Source
where
    Input: Stream<Item = SectionMessage> + Send + 'static,
    Output: Sink<SectionMessage, Error = SectionError> + Send + 'static,
    SectionChan: SectionChannel,
{
    type Error = SectionError;
    type Future = SectionFuture;

    fn start(
        self,
        _input: Input,
        output: Output,
        mut section_channel: SectionChan,
    ) -> Self::Future {
        Box::pin(async move {
            let mut inner: S3SourceInner = self.try_into()?;
            if inner.stream_binary {
                Err("binary streaming not yet implemented")?
            }
            let mut output = pin!(output);
            let bucket = inner
                .bucket
                .host()
                .ok_or("bucket url doesn't contain host")?
                .to_string();
            let config = SdkConfig::builder()
                .credentials_provider(SharedCredentialsProvider::new(
                    StaticCredentialsProvider::new(
                        inner.access_key_id.clone(),
                        inner.secret_key.clone(),
                    ),
                ))
                .behavior_version(BehaviorVersion::latest())
                .region(Region::new(inner.region.clone()));
            let config = match inner.endpoint.take() {
                None => config,
                Some(endpoint) => {
                    tracing::info!("using custom endpoint: {}", endpoint);
                    config
                        // FIXME: region is ignored for custom endpoint
                        .region(Some(Region::new("localhost")))
                        .endpoint_url(endpoint)
                }
            }
            .build();

            let client = Client::new(&config);
            let prefix = inner
                .bucket
                .path()
                .strip_prefix('/')
                .unwrap_or("")
                .to_string();
            inner.bucket.set_path("");

            let mut state = section_channel
                .retrieve_state()
                .await?
                .unwrap_or(State::new());
            let mut start_after = state.get::<String>(START_AFTER_KEY)?.unwrap_or("".into());
            start_after = inner.start_after.clone().max(start_after);
            let mut interval = tokio::time::interval(inner.interval);
            tracing::info!("start after: {start_after}");
            loop {
                futures::select! {
                    cmd = section_channel.recv().fuse() => {
                        if let HandleCommandResult::Stop = inner.handle_command(cmd?, &mut state, &mut section_channel).await? {
                            return Ok(())
                        }
                    }
                    _ = interval.tick().fuse() => {
                        let mut response = client
                            .list_objects_v2()
                            .bucket(&bucket)
                            .max_keys(1000)
                            .prefix(&prefix)
                            .start_after(&start_after)
                            .into_paginator()
                            .send();
                        if let Some(result) = response.next().await {
                            for object in result?.contents() {
                                let key = object.key().ok_or("object without name")?.to_string();
                                start_after = key.to_string();
                                let path: Arc<str> = Arc::from(inner.bucket.join(&key)?.to_string());
                                let weak_chan = section_channel.weak_chan();

                                let ack = Box::pin(async move { weak_chan.ack(Box::new(key)).await; });

                                let msg = Box::new(S3Message::new(
                                    Arc::clone(&path),
                                    Box::new(S3Object::new(path)),
                                    ack,
                                ));

                                let mut future = pin!(output.send(msg));
                                loop {
                                    futures::select!{
                                        cmd = section_channel.recv().fuse() => {
                                            if let HandleCommandResult::Stop = inner.handle_command(cmd?, &mut state, &mut section_channel).await? {
                                                return Ok(())
                                            }
                                        }
                                        msg = (&mut future).fuse() => {
                                            msg?;
                                            break
                                        },
                                    }
                                }
                            }
                        }
                        interval.reset();
                    }
                }
            }
        })
    }
}

//! list or stream files from s3 bucket
use std::{pin::pin, sync::Arc, time::Duration};

use crate::{Result, StaticCredentialsProvider};
use aws_config::{BehaviorVersion, Region, SdkConfig};
use aws_sdk_s3::{config::SharedCredentialsProvider, Client};
use section::prelude::*;

#[derive(Debug)]
pub struct S3Source {
    bucket: url::Url,
    region: String,
    stream_binary: bool,
    start_after: String,
    interval: Duration,
    access_key_id: String,
    secret_key: String,
}

impl S3Source {
    pub fn new(
        bucket: impl AsRef<str>,
        region: impl Into<String>,
        access_key_id: impl Into<String>,
        secret_key: impl Into<String>,
        stream_binary: bool,
        start_after: Option<impl AsRef<str>>,
        interval: Duration,
    ) -> Result<Self> {
        let bucket = url::Url::try_from(bucket.as_ref())?;
        if bucket.scheme() != "s3" {
            Err("expected url with 's3' schema")?
        };
        if stream_binary {
            Err("binary streaming not yet implemented")?
        }
        let region = region.into();
        let start_after = start_after.as_ref().map(AsRef::as_ref).unwrap_or("").into();
        let access_key_id = access_key_id.into();
        let secret_key = secret_key.into();
        Ok(Self {
            bucket,
            region,
            stream_binary,
            start_after,
            interval,
            access_key_id,
            secret_key,
        })
    }
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
        mut self,
        _input: Input,
        output: Output,
        mut section_channel: SectionChan,
    ) -> Self::Future {
        Box::pin(async move {
            let mut output = pin!(output);
            let bucket = self
                .bucket
                .host()
                .ok_or("bucket url doesn't contain host")?;
            let config = SdkConfig::builder()
                .credentials_provider(SharedCredentialsProvider::new(
                    StaticCredentialsProvider::new(
                        self.access_key_id.clone(),
                        self.secret_key.clone(),
                    ),
                ))
                .behavior_version(BehaviorVersion::latest())
                .region(Region::new(self.region.clone()))
                .build();
            let client = Client::new(&config);
            let bucket = self.bucket.host().unwrap().to_string();
            let prefix = self
                .bucket
                .path()
                .strip_prefix('/')
                .unwrap_or("")
                .to_string();
            self.bucket.set_path("");

            let mut state = section_channel
                .retrieve_state()
                .await?
                .unwrap_or(State::new());
            let mut start_after = state.get::<String>(START_AFTER_KEY)?.unwrap_or("".into());
            start_after = self.start_after.max(start_after);
            let mut interval = tokio::time::interval(self.interval);
            loop {
                futures::select! {
                    cmd = section_channel.recv().fuse() => {
                        match cmd? {
                            Command::Stop => return Ok(()),
                            Command::Ack(ack) => {
                                match ack.downcast::<Arc<str>>() {
                                    Ok(path) => {
                                        state.set(START_AFTER_KEY, path.to_string())?;
                                        section_channel.store_state(state.clone()).await?;
                                    }
                                    Err(_) => Err("failed to downcast Ack message")?
                                };
                            },
                            _ => (),
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
                                let key = object.key().ok_or("object without name")?;
                                start_after = key.to_string();
                                let path: Arc<str> = Arc::from(self.bucket.join(key)?.to_string());
                                let weak_chan = section_channel.weak_chan();

                                let ack = {
                                    let path = Arc::clone(&path);
                                    Box::pin(async move { weak_chan.ack(Box::new(path)).await; })
                                };

                                let msg = Box::new(S3Message::new(
                                    Arc::clone(&path),
                                    Box::new(S3Object::new(path)),
                                    ack,
                                ));

                                output.send(msg).await?;
                            }
                        }
                        interval.reset();
                    }
                }
            }
        })
    }
}

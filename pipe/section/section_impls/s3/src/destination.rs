use std::{
    pin::{pin, Pin},
    task::{Context, Poll},
};

use crate::{Result, StaticCredentialsProvider};
use aws_config::{BehaviorVersion, Region, SdkConfig};
use aws_sdk_s3::{
    config::SharedCredentialsProvider,
    primitives::SdkBody,
    types::{CompletedMultipartUpload, CompletedPart},
    Client,
};
use bytes::Bytes;
use http_body::{Body, Frame, SizeHint};
use section::prelude::*;

#[derive(Debug)]
pub struct S3Destination {
    bucket: url::Url,
    region: String,
    access_key_id: String,
    secret_access_key: String,
    max_upload_part_size: usize,
}

impl S3Destination {
    pub fn new(
        bucket: impl AsRef<str>,
        region: impl Into<String>,
        access_key_id: impl Into<String>,
        secret_access_key: impl Into<String>,
    ) -> Result<Self> {
        let url = url::Url::try_from(bucket.as_ref())?;
        let scheme = url.scheme();
        if scheme != "s3" {
            Err("bad url scheme: {scheme}")?
        };
        if url.host().is_none() {
            Err("s3 url host missing")?
        }
        Ok(Self {
            bucket: url::Url::try_from(bucket.as_ref())?,
            region: region.into(),
            access_key_id: access_key_id.into(),
            secret_access_key: secret_access_key.into(),
            max_upload_part_size: 1 << 23, // 8 MB
        })
    }

    #[allow(clippy::too_many_arguments)]
    async fn maybe_upload_part(
        &self,
        client: &Client,
        upload_id: &str,
        bucket: &str,
        key: &str,
        buf: &mut ChunkBuffer,
        completed_parts: &mut Vec<CompletedPart>,
        force: bool,
    ) -> Result<()> {
        if !buf.needs_flush() && !force {
            return Ok(());
        }
        let stream = buf.flush();
        if stream.is_empty() {
            return Ok(());
        }
        let body = SdkBody::from_body_1_x(stream);
        let part_number = (completed_parts.len() + 1) as i32;
        let upload_part_res = client
            .upload_part()
            .key(key)
            .bucket(bucket)
            .upload_id(upload_id)
            .body(body.into())
            .part_number(part_number)
            .send()
            .await?;
        completed_parts.push(
            CompletedPart::builder()
                .e_tag(upload_part_res.e_tag.unwrap_or_default())
                .part_number(part_number)
                .build(),
        );
        Ok(())
    }
}

struct ChunkBuffer {
    buffer: Vec<Bytes>,
    len: usize,
    limit: usize,
}

impl ChunkBuffer {
    fn new(limit: usize) -> Self {
        Self {
            buffer: vec![],
            len: 0,
            limit,
        }
    }

    fn append(&mut self, mut chunk: Bytes) -> Option<Bytes> {
        let rest = match self.len + chunk.len() > self.limit {
            true => Some(chunk.split_off(self.limit - self.len)),
            false => None,
        };
        self.len += chunk.len();
        self.buffer.push(chunk);
        rest
    }

    fn flush(&mut self) -> VecByteStream {
        let mut buf = vec![];
        let len = self.len;
        self.len = 0;
        std::mem::swap(&mut self.buffer, &mut buf);
        buf.reverse();
        VecByteStream::new(buf, len)
    }

    fn needs_flush(&mut self) -> bool {
        self.len >= self.limit
    }
}

struct VecByteStream {
    buf: Vec<Bytes>,
    size: usize,
}

impl VecByteStream {
    fn new(buf: Vec<Bytes>, size: usize) -> Self {
        Self { buf, size }
    }

    fn is_empty(&self) -> bool {
        self.buf.is_empty()
    }
}

impl Body for VecByteStream {
    type Data = Bytes;
    type Error = SectionError;

    fn poll_frame(
        mut self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
        match self.buf.pop() {
            Some(chunk) => Poll::Ready(Some(Ok(Frame::data(chunk)))),
            None => Poll::Ready(None),
        }
    }

    fn is_end_stream(&self) -> bool {
        self.buf.is_empty()
    }

    fn size_hint(&self) -> SizeHint {
        SizeHint::with_exact(self.size as u64)
    }
}

impl<Input, Output, SectionChan> Section<Input, Output, SectionChan> for S3Destination
where
    Input: Stream<Item = SectionMessage> + Send + 'static,
    Output: Sink<SectionMessage, Error = SectionError> + Send + 'static,
    SectionChan: SectionChannel,
{
    type Error = SectionError;
    type Future = SectionFuture;

    fn start(
        self,
        input: Input,
        _output: Output,
        mut section_channel: SectionChan,
    ) -> Self::Future {
        Box::pin(async move {
            let mut input = pin!(input);
            let config = SdkConfig::builder()
                .credentials_provider(SharedCredentialsProvider::new(
                    StaticCredentialsProvider::new(
                        self.access_key_id.clone(),
                        self.secret_access_key.clone(),
                    ),
                ))
                .behavior_version(BehaviorVersion::latest())
                .region(Region::new(self.region.clone()))
                .build();
            let client = Client::new(&config);
            let bucket = self.bucket.host().unwrap().to_string();
            loop {
                futures::select! {
                    cmd = section_channel.recv().fuse() => {
                        if let Command::Stop = cmd? {
                            return Ok(())
                        }
                    },
                    // FIXME: select against command channel
                    msg = input.next().fuse() => {
                        let mut msg = match msg {
                            None => Err("input closed")?,
                            Some(msg) => msg,
                        };
                        let key = self.bucket.join(msg.origin())?;
                        let key = key.path().strip_prefix('/').ok_or("bad object path")?;
                        let multipart_upload = client
                            .create_multipart_upload()
                            .bucket(&bucket)
                            .key(key)
                            .send()
                            .await?;
                        let upload_id = multipart_upload.upload_id().ok_or("upload id missing")?;
                        let mut completed_parts = Vec::<CompletedPart>::new();
                        let mut buf = ChunkBuffer::new(self.max_upload_part_size);
                        while let Some(chunk) = msg.next().await? {
                            let mut chunk = match chunk {
                                Chunk::Byte(chunk) => Bytes::from(chunk),
                                _ => Err("expected binary stream")?
                            };
                            while let Some(rest) = buf.append(chunk) {
                                chunk = rest;
                                self.maybe_upload_part(
                                    &client,
                                    upload_id,
                                    &bucket,
                                    key,
                                    &mut buf,
                                    &mut completed_parts,
                                    false
                                ).await?;
                            }
                        }
                        self.maybe_upload_part(
                            &client,
                            upload_id,
                            &bucket,
                            key,
                            &mut buf,
                            &mut completed_parts,
                            true
                        ).await?;
                        let completed_multipart_upload = CompletedMultipartUpload::builder()
                            .set_parts(Some(completed_parts))
                            .build();
                        client
                            .complete_multipart_upload()
                            .bucket(&bucket)
                            .key(key)
                            .multipart_upload(completed_multipart_upload)
                            .upload_id(upload_id)
                            .send()
                            .await?;
                    }
                }
            }
        })
    }
}

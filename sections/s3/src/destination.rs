use std::{
    pin::{pin, Pin},
    task::{Context, Poll},
};

use crate::{static_credentials_provider::StaticCredentialsProvider, Result, S3Destination};
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
pub struct S3DestinationInner {
    bucket: url::Url,
    region: String,
    access_key_id: String,
    secret_key: String,
    max_upload_part_size: usize,
}

impl TryFrom<S3Destination> for S3DestinationInner {
    type Error = SectionError;

    fn try_from(value: S3Destination) -> std::result::Result<Self, Self::Error> {
        Self::new(
            value.bucket.as_str(),
            value.region,
            value.access_key_id,
            value.secret_key,
            value.max_upload_part_size,
        )
    }
}

impl S3DestinationInner {
    pub fn new(
        bucket: impl AsRef<str>,
        region: impl Into<String>,
        access_key_id: impl Into<String>,
        secret_key: impl Into<String>,
        max_upload_part_size: usize,
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
            secret_key: secret_key.into(),
            max_upload_part_size,
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
            let inner: S3DestinationInner = self.try_into()?;
            let mut input = pin!(input);
            let config = SdkConfig::builder()
                .credentials_provider(SharedCredentialsProvider::new(
                    StaticCredentialsProvider::new(
                        inner.access_key_id.clone(),
                        inner.secret_key.clone(),
                    ),
                ))
                .behavior_version(BehaviorVersion::latest())
                .region(Region::new(inner.region.clone()))
                .build();
            let client = Client::new(&config);
            let bucket = inner.bucket.host().unwrap().to_string();
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
                        let key = inner.bucket.join(msg.origin())?;
                        let key = key.path().strip_prefix('/').ok_or("bad object path")?;
                        let multipart_upload = client
                            .create_multipart_upload()
                            .bucket(&bucket)
                            .key(key)
                            .send()
                            .await?;
                        let upload_id = multipart_upload.upload_id().ok_or("upload id missing")?;
                        let mut completed_parts = Vec::<CompletedPart>::new();
                        let mut buf = ChunkBuffer::new(inner.max_upload_part_size);
                        let ack = msg.ack();
                        while let Some(chunk) = msg.next().await? {
                            let mut chunk = match chunk {
                                Chunk::Byte(chunk) => Bytes::from(chunk),
                                _ => Err("expected binary stream")?
                            };
                            while let Some(rest) = buf.append(chunk) {
                                chunk = rest;
                                inner.maybe_upload_part(
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
                        inner.maybe_upload_part(
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
                        tracing::debug!("uploaded {key}");
                        ack.await;
                    }
                }
            }
        })
    }
}

#[cfg(test)]
mod test {
    use std::{
        ptr,
        task::{RawWaker, RawWakerVTable, Waker},
    };

    use super::*;
    use quickcheck::TestResult;

    fn noop_raw_waker() -> RawWaker {
        const VTABLE: RawWakerVTable =
            RawWakerVTable::new(|_| noop_raw_waker(), |_| {}, |_| {}, |_| {});
        RawWaker::new(ptr::null(), &VTABLE)
    }

    fn noop_waker() -> Waker {
        unsafe { Waker::from_raw(noop_raw_waker()) }
    }

    fn consume_stream(result: &mut Vec<u8>, mut stream: VecByteStream) {
        let waker = noop_waker();
        let mut cx = Context::from_waker(&waker);
        while let Poll::Ready(Some(Ok(chunk))) = pin!(&mut stream).poll_frame(&mut cx) {
            let data = chunk.data_ref().unwrap();
            result.extend(data)
        }
    }

    #[test]
    fn test_chunk_buffer() {
        // for any given input/limit
        // chunk buffer should produce output which equals input
        let check = |limit: u8, chunks: Vec<Vec<u8>>| -> TestResult {
            let mut buffer = ChunkBuffer::new((limit as usize).max(1));
            let mut result = Vec::<u8>::new();
            for chunk in chunks.iter() {
                let mut chunk = Bytes::from(chunk.clone());
                while let Some(rest) = buffer.append(chunk) {
                    consume_stream(&mut result, buffer.flush());
                    chunk = rest;
                }
            }
            consume_stream(&mut result, buffer.flush());

            let flattened = chunks.into_iter().flatten().collect::<Vec<_>>();
            assert_eq!(flattened, result);
            TestResult::from_bool(true)
        };
        quickcheck::quickcheck(check as fn(u8, Vec<Vec<u8>>) -> TestResult);
    }
}

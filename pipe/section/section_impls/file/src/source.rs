//! Streams specified file once
//! Possible improvements:
//! - file watching
//! - dir support
//! - store state to remember what was sent?
//! - ???

use section::{
    command_channel::SectionChannel,
    futures::{Sink, SinkExt, Stream},
    message::{Chunk, Message},
    section::Section,
    {SectionError, SectionFuture, SectionMessage},
};
use std::pin::pin;
use tokio::{
    fs::{File, OpenOptions},
    io::AsyncReadExt,
};
#[derive(Debug)]
pub struct FileSource {
    path: String,
}

impl FileSource {
    pub fn new(path: impl Into<String>) -> Self {
        Self { path: path.into() }
    }
}

#[derive(Debug)]
struct FileStream {
    origin: String,
    fd: File,
}

impl Message for FileStream {
    fn origin(&self) -> &str {
        self.origin.as_str()
    }

    fn next(&mut self) -> section::message::Next<'_> {
        Box::pin(async {
            let mut buf = vec![0; 16384];
            match self.fd.read(buf.as_mut_slice()).await {
                Ok(0) => Ok(None),
                Ok(read) => {
                    buf.truncate(read);
                    Ok(Some(Chunk::Byte(buf)))
                }
                Err(e) => Err(e.into()),
            }
        })
    }

    fn ack(&mut self) -> section::message::Ack {
        Box::pin(async {})
    }
}

impl<Input, Output, SectionChan> Section<Input, Output, SectionChan> for FileSource
where
    Input: Stream<Item = SectionMessage> + Send + 'static,
    Output: Sink<SectionMessage, Error = SectionError> + Send + 'static,
    SectionChan: SectionChannel + Send + 'static,
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
            let mut output = pin!(output);
            let fd = OpenOptions::new().read(true).open(&self.path).await?;
            let msg = FileStream {
                origin: self.path.clone(),
                fd,
            };
            output.send(Box::new(msg)).await?;
            section_channel.recv().await?;
            Ok(())
        })
    }
}

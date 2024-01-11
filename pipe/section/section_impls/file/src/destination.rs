use section::{
    command_channel::{Command, SectionChannel},
    futures::{self, FutureExt, Sink, SinkExt, Stream, StreamExt},
    message::Chunk,
    section::Section,
    {SectionError, SectionFuture, SectionMessage},
};
use std::pin::pin;
use tokio::{fs::OpenOptions, io::AsyncWriteExt};

use crate::FileMessage;

#[derive(Debug)]
pub struct FileDestination {
    path: String,
}

impl FileDestination {
    pub fn new(path: impl Into<String>) -> Self {
        Self { path: path.into() }
    }
}

impl<Input, Output, SectionChan> Section<Input, Output, SectionChan> for FileDestination
where
    Input: Stream<Item = SectionMessage> + Send + Sync + 'static,
    Output: Sink<SectionMessage, Error = SectionError> + Send + Sync + 'static,
    SectionChan: SectionChannel + Send + Sync + 'static,
{
    type Error = SectionError;
    type Future = SectionFuture;

    fn start(self, input: Input, output: Output, mut section_channel: SectionChan) -> Self::Future {
        Box::pin(async move {
            let mut input = pin!(input);
            let mut output = pin!(output);

            loop {
                futures::select! {
                    cmd = section_channel.recv().fuse() => {
                        if let Command::Stop = cmd? {
                            return Ok(())
                        }
                    },
                    msg = input.next().fuse() => {
                        if msg.is_none() {
                            Err("input closed")?;
                        }
                        let mut msg = msg.unwrap();
                        let tmp_file = tokio::task::spawn_blocking(|| tempfile::NamedTempFile::new())
                            .await??;
                        let mut fd = OpenOptions::new()
                            .create(true)
                            .write(true)
                            .open(&tmp_file)
                            .await?;
                        loop {
                            match msg.next().await {
                                Ok(Some(Chunk::Byte(chunk))) => {
                                    fd.write_all(chunk.as_slice()).await?;
                                }
                                Ok(Some(chunk)) => {
                                    Err(format!("expected byte chunk, got: {:?}", chunk))?;
                                }
                                Ok(None) => {
                                    fd.flush().await?;
                                    drop(fd);
                                    tokio::fs::rename(tmp_file, self.path.as_str()).await?;
                                    msg.ack().await;
                                    let msg = Box::new(FileMessage::new(self.path.as_str()));
                                    output.send(msg).await?;
                                    break;
                                }
                                Err(e) => {
                                    Err(format!("tailed to receive message chunk: {:?}", e))?;
                                }
                            }
                        }
                    }
                }
            }
        })
    }
}

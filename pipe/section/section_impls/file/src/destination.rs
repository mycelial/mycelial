use section::{
    command_channel::{Command, SectionChannel},
    futures::{self, FutureExt, Sink, Stream, StreamExt},
    message::Chunk,
    section::Section,
    {SectionError, SectionFuture, SectionMessage},
};
use std::{path::PathBuf, pin::pin};
use tokio::{fs::OpenOptions, io::AsyncWriteExt};

#[derive(Debug)]
pub struct FileDestination {
    dir_path: PathBuf,
}

impl FileDestination {
    pub fn new(dir_path: &str) -> Self {
        Self {
            dir_path: dir_path.strip_suffix("/").unwrap_or(dir_path).into(),
        }
    }
}

impl<Input, Output, SectionChan> Section<Input, Output, SectionChan> for FileDestination
where
    Input: Stream<Item = SectionMessage> + Send + 'static,
    Output: Sink<SectionMessage, Error = SectionError> + Send + 'static,
    SectionChan: SectionChannel + Send + 'static,
{
    type Error = SectionError;
    type Future = SectionFuture;

    fn start(self, input: Input, output: Output, mut section_channel: SectionChan) -> Self::Future {
        Box::pin(async move {
            let mut input = pin!(input);
            let mut _output = pin!(output);
            tokio::fs::create_dir_all(self.dir_path.as_path()).await?;

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
                        let tmp_file = tokio::task::spawn_blocking(tempfile::NamedTempFile::new)
                            .await??;
                        let mut fd = OpenOptions::new()
                            .create(true)
                            .truncate(true)
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
                                    tokio::fs::rename(tmp_file, self.dir_path.join(msg.origin())).await?;
                                    msg.ack().await;
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

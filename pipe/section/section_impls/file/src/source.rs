//! Streams specified file once
//! Possible improvements:
//! - dir support
//! - store state to remember what was sent?
//! - ???

use notify::{Event, RecursiveMode, Watcher};
use section::{
    command_channel::{Command, SectionChannel, WeakSectionChannel},
    futures::{self, FutureExt, Sink, SinkExt, Stream},
    message::{Ack, Chunk, Message},
    section::Section,
    state::State,
    {SectionError, SectionFuture, SectionMessage},
};
use std::{path::Path, pin::pin};
use tokio::{
    fs::{File, OpenOptions},
    io::AsyncReadExt,
    sync::mpsc::Sender,
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

struct FileStream {
    origin: String,
    fd: File,
    ack: Option<Ack>,
}

impl std::fmt::Debug for FileStream {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FileStream")
            .field("origin", &self.origin)
            .field("fd", &self.fd)
            .finish()
    }
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
        self.ack.take().unwrap_or(Box::pin(async {}))
    }
}

const LAST_MTIME: &str = "last_mtime";

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
            let mut state = section_channel
                .retrieve_state()
                .await?
                .unwrap_or(SectionChan::State::new());

            let (tx, mut rx) = tokio::sync::mpsc::channel::<()>(1);
            tx.send(()).await?;
            let mut last_mtime = state.get(LAST_MTIME)?.unwrap_or(0);

            let _watcher = watch_file(self.path.as_str(), tx);
            loop {
                futures::select! {
                    _ = rx.recv().fuse() => {
                        let fd = OpenOptions::new().read(true).open(&self.path).await?;
                        let metadata = fd.metadata().await?;
                        let mtime = metadata.modified()?.duration_since(std::time::UNIX_EPOCH)?
                            .as_micros() as i64;
                        if last_mtime == mtime {
                            continue
                        }
                        last_mtime = mtime;

                        let weak_chan = section_channel.weak_chan();
                        let msg = FileStream {
                            origin: self.path.clone(),
                            fd,
                            ack: Some(Box::pin(async move { weak_chan.ack(Box::new(mtime)).await } ))
                        };

                        output.send(Box::new(msg)).await?;
                    },
                    cmd = section_channel.recv().fuse() => {
                        match cmd? {
                            Command::Stop => {
                                return Ok(())
                            },
                            Command::Ack(any) => {
                                match any.downcast::<i64>() {
                                    Ok(last_mtime) => {
                                        state.set(LAST_MTIME, *last_mtime)?;
                                        section_channel.store_state(state.clone()).await?;
                                    },
                                    Err(_) => Err("Failed to downcast Ack to i64")?,
                                }
                            },
                            _ => (),
                        }

                    }
                }
            }
        })
    }
}

fn watch_file(path: &str, tx: Sender<()>) -> notify::Result<impl Watcher> {
    let mut watcher = notify::recommended_watcher(move |res: Result<Event, _>| match res {
        Ok(event) if event.kind.is_modify() || event.kind.is_create() => {
            tx.blocking_send(()).ok();
        }
        Ok(_) => (),
        Err(_e) => (),
    })?;
    watcher.watch(Path::new(path), RecursiveMode::NonRecursive)?;
    Ok(watcher)
}

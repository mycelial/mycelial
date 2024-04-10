#![allow(unused)]
use regex::Regex;
use section::{
    command_channel::{Command, SectionChannel, WeakSectionChannel},
    futures::{self, FutureExt, Sink, SinkExt, Stream, StreamExt},
    message::{Ack, Chunk, Column, DataFrame, DataType, Message, Next, ValueView},
    section::Section,
    state::State,
    SectionError, SectionFuture, SectionMessage,
};
use std::pin::pin;
use std::{
    any::Any,
    collections::VecDeque,
    path::{Path, PathBuf},
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
    time::Duration,
};
use tokio::fs::{read_dir, DirEntry};
use tokio_stream::wrappers::ReadDirStream;

pub type Result<T, E = SectionError> = core::result::Result<T, E>;

struct DirSourceMessage {
    origin: Arc<str>,
    path: Option<Arc<str>>,
    ack: Option<Ack>,
}

impl DirSourceMessage {
    fn new(origin: Arc<str>, ack: Ack) -> Self {
        let path = Arc::clone(&origin);
        Self {
            origin,
            path: Some(path),
            ack: Some(ack),
        }
    }
}

impl std::fmt::Debug for DirSourceMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DirSourceMessage")
            .field("origin", &self.origin.as_ref())
            .field("path", &self.path.as_ref())
            .field("ack", &self.ack.as_ref().map(|_| "Some").unwrap_or("None"))
            .finish()
    }
}

#[derive(Debug)]
struct PathDataFrame {
    path: Arc<str>,
}

impl DataFrame for PathDataFrame {
    fn columns(&self) -> Vec<Column<'_>> {
        vec![Column::new(
            "path",
            DataType::Str,
            Box::new(std::iter::once(ValueView::Str(self.path.as_ref()))),
        )]
    }
}

impl Message for DirSourceMessage {
    fn ack(&mut self) -> Ack {
        self.ack.take().unwrap_or(Box::pin(async {}))
    }

    fn next(&mut self) -> Next<'_> {
        let chunk = self
            .path
            .take()
            .map(|path| Chunk::DataFrame(Box::new(PathDataFrame { path })));
        Box::pin(async move { Ok(chunk) })
    }

    fn origin(&self) -> &str {
        self.origin.as_ref()
    }
}

#[derive(Debug)]
pub struct DirSource {
    path: PathBuf,
    pattern: Option<Regex>,
    start_after: Option<String>,
    walk_stack: Vec<DirEntry>,
    stream_mode: (),
    interval: Duration,
}

impl DirSource {
    pub fn new(
        path: PathBuf,
        pattern: Option<String>,
        start_after: Option<String>,
        interval: Duration,
    ) -> Result<Self> {
        let pattern = match pattern {
            Some(s) => Some(Regex::try_from(s)?),
            None => None,
        };
        Ok(Self {
            path,
            pattern,
            start_after,
            walk_stack: vec![],
            stream_mode: (),
            interval,
        })
    }

    // initiate walking stack
    async fn init_walk_stack(&mut self) -> Result<()> {
        let mut read_dir = tokio::fs::read_dir(self.path.as_path()).await?;
        while let Some(entry) = read_dir.next_entry().await.map_err(|e| {
            format!(
                "failed to initialize walking stack at {:?}: {e}",
                self.path.as_path()
            )
        })? {
            let full_path = entry.path();
            let rel_fname = full_path
                .strip_prefix(self.path.as_path())?
                .to_string_lossy()
                .to_string();
            if Some(&rel_fname) < self.start_after.as_ref() {
                tracing::debug!(
                    "{:?} filtered, less than current `start_after`: {:?}",
                    full_path,
                    self.start_after
                );
                continue;
            }
            self.walk_stack.push(entry);
        }
        // sort in reverse order
        self.walk_stack
            .sort_by_key(|right| std::cmp::Reverse(right.path()));
        Ok(())
    }

    // walk path
    // apply pattern/start_after filters
    // sort result, since `read_dir` is not guaranteed to be sorted
    async fn walk_path(&mut self) -> Result<Option<String>> {
        loop {
            let entry = match self.walk_stack.pop() {
                Some(entry) => entry,
                None => return Ok(None),
            };
            let entry_path = entry.path();
            let entry_path = entry_path.as_path();
            let entry_type = entry
                .file_type()
                .await
                .map_err(|e| format!("failed to read file type at {:?} : {e}", entry_path))?;
            match (entry_type.is_dir(), entry_type.is_file()) {
                (true, _) => {
                    let mut entries = vec![];
                    let mut entry_stream = ReadDirStream::new(
                        read_dir(entry_path)
                            .await
                            .map_err(|e| format!("failed to read dir at {:?}: {e}", entry_path))?,
                    );
                    while let Some(inner) = entry_stream.next().await {
                        inner.map(|inner| entries.push(inner)).map_err(|e| {
                            format!("failed to read directory '{:?}': {e}", entry_path)
                        })?;
                    }
                    // sort in reverse order to preserve proper order after pushing values to front of vec deque
                    entries.sort_by_key(|right| std::cmp::Reverse(right.file_name()));
                    entries
                        .into_iter()
                        .for_each(|entry| self.walk_stack.push(entry));
                }
                (_, true) => {
                    let rel_fname = entry_path
                        .strip_prefix(self.path.as_path())?
                        .to_string_lossy()
                        .to_string();
                    let fname = entry_path.to_string_lossy().to_string();
                    if self.start_after.as_ref() >= Some(&fname) {
                        tracing::debug!(
                            "'{fname}' filtered, less than value of `start_after` {:?}",
                            self.start_after
                        );
                        continue;
                    }
                    if self
                        .pattern
                        .as_ref()
                        .map(|pattern| !pattern.is_match(&rel_fname))
                        .unwrap_or(false)
                    {
                        tracing::debug!("'{fname}' filtered, doesn't match pattern");
                        continue;
                    }
                    // updating start_after filter here
                    self.start_after = Some(fname);
                    // FIXME: converting pathbuf to string, good enough for now, but incorrect in general
                    return Ok(Some(entry.path().to_string_lossy().to_string()));
                }
                _ => (),
            }
        }
    }
}

const START_AFTER_KEY: &str = "start_after";
const PATH_KEY: &str = "path";

impl<Input, Output, SectionChan> Section<Input, Output, SectionChan> for DirSource
where
    Input: Stream<Item = SectionMessage> + Send + 'static,
    Output: Sink<SectionMessage, Error = SectionError> + Send + 'static,
    SectionChan: SectionChannel + Send + 'static,
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
            let mut interval = tokio::time::interval(self.interval);
            let mut state = section_channel
                .retrieve_state()
                .await?
                .unwrap_or(State::new());

            // check if state needs to be reset
            // reset can happen if:
            // - section path was changed
            match state.get::<String>(PATH_KEY)? {
                Some(ref path) if PathBuf::from(path) != self.path => {
                    tracing::warn!("path changed, resetting state");
                    state = State::new();
                    section_channel.store_state(state.clone()).await?;
                }
                _ => (),
            };
            state.set(PATH_KEY, self.path.to_string_lossy().to_string())?;
            self.start_after = state.get(START_AFTER_KEY)?.unwrap_or(self.start_after);

            loop {
                futures::select! {
                    _ = interval.tick().fuse() => {
                        self.init_walk_stack().await?;
                        loop {
                            let file = match self.walk_path().await {
                                Err(e) => {
                                    tracing::error!("failed to traverse path: {}", e);
                                    break
                                },
                                Ok(None) => break,
                                Ok(Some(file)) => file,

                            };
                            let weak_chan = section_channel.weak_chan();
                            let file = Arc::from(file);
                            let file_clone: Box<dyn Any + Send> = Box::new(Arc::clone(&file));
                            let ack: Ack = Box::pin(async move { weak_chan.ack(file_clone).await; });
                            let message: SectionMessage = Box::new(DirSourceMessage::new(file, ack));
                            output.send(message).await?;
                        }
                    },
                    cmd = section_channel.recv().fuse() => {
                        match cmd? {
                            Command::Stop => return Ok(()),
                            Command::Ack(ack) => {
                                match ack.downcast_ref::<Arc<str>>() {
                                    Some(acked) => {
                                        tracing::debug!("ack for '{acked}' received");
                                        state.set(START_AFTER_KEY, acked.to_string());
                                        section_channel.store_state(state.clone()).await?;
                                    },
                                    None => Err("failed to downcast Ack message")?
                                };
                            },
                            _ => (),
                        }
                    }
                }
            }
        })
    }
}

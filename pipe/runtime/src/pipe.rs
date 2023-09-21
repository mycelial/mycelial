//! Pipe

use futures::{Sink, SinkExt, Stream};
use std::future::IntoFuture;
use tokio::task::JoinHandle;

use crate::channel::channel;
use crate::command_channel::RootChannel;
use crate::types::{DynSection, DynSink, DynStream, SectionError, SectionFuture};
use section::{Section, ReplyTo as _, SectionRequest, State};
use section::RootChannel as _;

use super::config::Config;
use super::message::Message;
use super::registry::Registry;
use stub::Stub;

#[allow(dead_code)]
pub struct Pipe<S: State> {
    id: u64,
    config: Config,
    sections: Option<Vec<Box<dyn DynSection<S>>>>,
}

impl<S: State> std::fmt::Debug for Pipe<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Pipe")
            .field("config", &self.config)
            .field(
                "section",
                &self
                    .sections
                    .iter()
                    .map(|_| "<Section>")
                    .collect::<Vec<&'static str>>(),
            )
            .finish()
    }
}

impl<S: State> Pipe<S> {
    pub fn new(id: u64, config: Config, sections: Vec<Box<dyn DynSection<S>>>) -> Self {
        Self {
            id,
            config,
            sections: Some(sections),
        }
    }
}

impl<S: State> IntoFuture for Pipe<S> {
    type Output = Result<(), SectionError>;
    type IntoFuture = SectionFuture;

    fn into_future(self) -> Self::IntoFuture {
        Box::new(self).start(
            Stub::<_, SectionError>::new(),
            Stub::<_, SectionError>::new(),
            (),
        )
    }
}

impl<S: State> TryFrom<(u64, Config, &'_ Registry<S>)> for Pipe<S> {
    type Error = SectionError;

    fn try_from(
        (id, config, registry): (u64, Config, &Registry<S>),
    ) -> Result<Self, Self::Error> {
        let sections = config
            .get_sections()
            .iter()
            .map(|section_cfg| -> Result<Box<dyn DynSection<S>>, SectionError> {
                let name: &str = section_cfg
                    .get("name")
                    .ok_or("section needs to have a name")?
                    .as_str()
                    .ok_or("section name should be string")?;
                let constructor = registry
                    .get_constructor(name)
                    .ok_or(format!("no constructor for '{name}' available"))?;
                constructor(section_cfg)
            })
            .collect::<Result<Vec<Box<dyn DynSection<S>>>, _>>()?;
        Ok(Pipe::new(id, config, sections))
    }
}

impl<Input, Output, S: State> Section<Input, Output, ()> for Pipe<S>
where
    Input: Stream<Item = Message> + Send + 'static,
    Output: Sink<Message, Error = SectionError> + Send + 'static,
{
    type Error = SectionError;
    type Future = SectionFuture;

    fn start(mut self, input: Input, output: Output, _command: ()) -> Self::Future {
        let len = self.sections.as_ref().unwrap().len();
        let input: DynStream = Box::pin(input);
        let output: DynSink = Box::pin(output);
        let mut root_channel = RootChannel::new();
        let (_, _, _, handles) = self
            .sections
            .take()
            .unwrap()
            .into_iter()
            .enumerate()
            .fold(
                (None, Some(input), Some(output), vec![]), |(prev, mut pipe_input, mut pipe_output, mut acc), (pos, section)| {
                    let input: DynStream = match prev {
                        None => pipe_input.take().unwrap(),
                        Some(rx) => rx,
                    };
                    let (next_input, output): (DynStream, DynSink) =
                        if pos == len - 1 {
                            // last element
                            let next_input = Box::pin(Stub::<_, SectionError>::new());
                            let output = pipe_output.take().unwrap();
                            (next_input, output)
                        } else {
                            let (tx, rx) = channel::<Message>(1);
                            let tx = tx.sink_map_err(|_| -> SectionError { "send error".into() });
                            (Box::pin(rx), Box::pin(tx))
                        };
                    let section_channel = root_channel.section_channel(pos as u64).unwrap();
                    let handle = tokio::spawn(section.dyn_start(input, output, section_channel));
                    acc.push(HandleWrap::new(handle));
                    (Some(next_input), pipe_input, pipe_output, acc)
                },
            );

        let future = async move {
            let mut handles = handles;
            while let Ok(msg) = root_channel.recv().await {
                match msg {
                    SectionRequest::StoreState{reply_to, ..} => {
                        // FIXME: unwrap
                        //let name = self.config.get_sections()[id as usize].get("name").unwrap();
                        //let future = self.storage.store_state(self.id, id, name.as_str().unwrap().to_string(), state);
                        //future.await?;
                        reply_to.reply(()).await?;
                    },
                    SectionRequest::RetrieveState { reply_to, .. } => {
                        reply_to.reply(None).await?;
                    },
                    SectionRequest::Log { id, message } => {
                        println!("log request from section with id: {id}, message: {message}");
                    },
                    SectionRequest::Stopped { id } => {
                        return match handles[id as usize].handle.take() {
                            Some(handle) => handle.await?,
                            None => Ok(())
                        }
                    },
                    _req => {
                        unreachable!()
                    }
                }
            }
            Ok(())
        };
        Box::pin(future)
    }
}

// Wrapper around tokio handle
//
// Implements custom Drop to abort spawned tasks
struct HandleWrap {
    handle: Option<JoinHandle<Result<(), SectionError>>>,
}

impl HandleWrap {
    fn new(handle: JoinHandle<Result<(), SectionError>>) -> Self {
        Self {
            handle: Some(handle),
        }
    }
}

impl Drop for HandleWrap {
    fn drop(&mut self) {
        if let Some(handle) = self.handle.take() {
            handle.abort()
        }
    }
}

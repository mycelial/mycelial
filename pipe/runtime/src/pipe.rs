//! Pipe

use tokio::task::JoinHandle;

use crate::{
    channel::channel,
    config::Config,
    registry::Registry,
    types::{DynSection, DynSink, DynStream},
};
use section::{
    command_channel::{Command, ReplyTo as _, RootChannel, SectionChannel, SectionRequest},
    futures::{self, FutureExt, Sink, SinkExt, Stream},
    section::Section,
    state::State,
    SectionError, SectionFuture, SectionMessage,
};
use stub::Stub;

#[allow(dead_code)]
pub struct Pipe<R: RootChannel + Send + 'static> {
    config: Config,
    sections: Option<Vec<Box<dyn DynSection<R::SectionChannel>>>>,
}

impl<R: RootChannel + Send + 'static> std::fmt::Debug for Pipe<R> {
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

impl<R: RootChannel + Send + 'static> Pipe<R> {
    pub fn new(config: Config, sections: Vec<Box<dyn DynSection<R::SectionChannel>>>) -> Self {
        Self {
            config,
            sections: Some(sections),
        }
    }
}

impl<R: RootChannel + Send + 'static> TryFrom<(&'_ Config, &'_ Registry<R::SectionChannel>)>
    for Pipe<R>
{
    type Error = SectionError;

    fn try_from(
        (config, registry): (&Config, &Registry<R::SectionChannel>),
    ) -> Result<Self, Self::Error> {
        let sections = config
            .get_sections()
            .iter()
            .map(
                |section_cfg: &std::collections::HashMap<String, crate::config::Value>| -> Result<Box<dyn DynSection<R::SectionChannel>>, SectionError> {
                    let name: &str = section_cfg
                        .get("name")
                        .ok_or("section needs to have a name")?
                        .as_str()
                        .ok_or("section name should be string")?;
                    let constructor = registry.get_constructor(name).ok_or(format!(
                        "the runtime's registry contains no constructor for '{name}' available"
                    ))?;
                    constructor(section_cfg)
                },
            )
            .collect::<Result<Vec<Box<dyn DynSection<R::SectionChannel>>>, _>>()?;
        Ok(Pipe::new(config.clone(), sections))
    }
}

impl<Input, Output, RootChan> Section<Input, Output, <RootChan as RootChannel>::SectionChannel>
    for Pipe<RootChan>
where
    Input: Stream<Item = SectionMessage> + Send + 'static,
    Output: Sink<SectionMessage, Error = SectionError> + Send + 'static,
    RootChan: RootChannel + Send + 'static,
{
    type Error = SectionError;
    type Future = SectionFuture;

    fn start(
        mut self,
        input: Input,
        output: Output,
        mut section_chan: RootChan::SectionChannel,
    ) -> Self::Future {
        let len = self.sections.as_ref().unwrap().len();
        let input: DynStream = Box::pin(input);
        let output: DynSink = Box::pin(output);
        let mut root_channel = <RootChan as RootChannel>::new();
        let (_, _, _, handles) = self.sections.take().unwrap().into_iter().enumerate().fold(
            (None, Some(input), Some(output), vec![]),
            |(prev, mut pipe_input, mut pipe_output, mut acc), (pos, section)| {
                let input: DynStream = match prev {
                    None => pipe_input.take().unwrap(),
                    Some(rx) => rx,
                };
                let (next_input, output): (DynStream, DynSink) = if pos == len - 1 {
                    // last element
                    let next_input = Box::pin(Stub::<_, SectionError>::new());
                    let output = pipe_output.take().unwrap();
                    (next_input, output)
                } else {
                    let (tx, rx) = channel::<SectionMessage>(1);
                    let tx = tx.sink_map_err(|_| -> SectionError { "send error".into() });
                    (Box::pin(rx), Box::pin(tx))
                };
                let section_channel = root_channel.add_section(pos as u64).unwrap();
                let handle = tokio::spawn(section.dyn_start(input, output, section_channel));
                acc.push(HandleWrap::new(handle));
                (Some(next_input), pipe_input, pipe_output, acc)
            },
        );

        let future = async move {
            let mut state = section_chan.retrieve_state().await?.unwrap_or(
                <<RootChan as RootChannel>::SectionChannel as SectionChannel>::State::new(),
            );
            let mut handles = handles;
            loop {
                futures::select! {
                    msg = root_channel.recv().fuse() => {
                        match msg? {
                            SectionRequest::StoreState { reply_to, id, state: section_state } => {
                                state.set(&format!("{id}"), section_state)?;
                                section_chan.store_state(state.clone()).await?;
                                reply_to.reply(()).await?;
                            }
                            SectionRequest::RetrieveState { id, reply_to } => {
                                let retrieved_state = state.get(&format!("{id}"))?;
                                reply_to.reply(retrieved_state).await?;
                            }
                            SectionRequest::Log { id, message } => {
                                section_chan.log(format!("section_id<id: {id}>: {message}")).await?;
                            }
                            SectionRequest::Stopped { id } => {
                                return match handles[id as usize].handle.take() {
                                    Some(handle) => handle.await?,
                                    None => Ok(()),
                                }
                            }
                            req => unreachable!("{:?}", req)
                        }
                    },
                    cmd = section_chan.recv().fuse() => {
                        if let Command::Stop = cmd? {
                            return Ok(())
                        }
                    }
                }
            }
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

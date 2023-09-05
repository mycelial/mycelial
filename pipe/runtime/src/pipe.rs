//! Pipe

use futures::{Sink, SinkExt, Stream};
use tokio::task::JoinHandle;
use std::future::IntoFuture;

use crate::command_channel::{Command, RootChannel};
use crate::channel::channel;
use section::Section;
use crate::types::{SectionError, SectionFuture, DynStream, DynSink, DynSection};

use super::config::Config;
use super::message::Message;
use super::registry::Registry;
use super::scheduler::Storage;
use stub::Stub;

pub struct Pipe<T> {
    id: u64,
    storage: T,
    config: Config,
    sections: Option<Vec<Box<dyn DynSection>>>,
}

impl<T: std::fmt::Debug> std::fmt::Debug for Pipe<T> {
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

impl<T> Pipe<T> {
    pub fn new(id: u64, config: Config, sections: Vec<Box<dyn DynSection>>, storage: T) -> Self {
        Self { id, storage, config, sections: Some(sections) }
    }
}

impl<T: Storage + Send + 'static> IntoFuture for Pipe<T> {
    type Output = Result<(), SectionError>;
    type IntoFuture = SectionFuture;

    fn into_future(self) -> Self::IntoFuture {
        Box::new(self).start(Stub::<_, SectionError>::new(), Stub::<_, SectionError>::new(), ())
    }
}

impl<T: Storage + Send + 'static> TryFrom<(u64, Config, &'_ Registry, T)> for Pipe<T> {
    type Error = SectionError;

    fn try_from((id, config, registry, storage): (u64, Config, &Registry, T)) -> Result<Self, Self::Error> {
        let sections = config
            .get_sections()
            .iter()
            .map(|section_cfg| -> Result<Box<dyn DynSection>, SectionError> {
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
            .collect::<Result<Vec<Box<dyn DynSection>>, _>>()?;
        Ok(Pipe::new(id, config, sections, storage))
    }
}

impl<Input, Output, T> Section<Input, Output, ()> for Pipe<T>
where
    Input: Stream<Item = Message> + Send + 'static,
    Output: Sink<Message, Error = SectionError> + Send + 'static,
    T: Storage + Send + 'static,
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
                    let (section_tx, section_channel) = root_channel.section_channel(pos as u64);
                    let handle = tokio::spawn(section.dyn_start(input, output, section_channel));
                    acc.push((section_tx, HandleWrap::new(handle)));
                    (Some(next_input), pipe_input, pipe_output, acc)
                },
            );

        let future = async move {
            let mut handles = handles;
            while let Some(msg) = root_channel.rx.recv().await {
                match msg {
                    Command::StoreState{id, state} => {
                        // FIXME: unwrap
                        let name = self.config.get_sections()[id as usize].get("name").unwrap();
                        let future = self.storage.store_state(self.id, id, name.as_str().unwrap().to_string(), state);
                        future.await?;
                    },
                    Command::RetrieveState{id, reply_to} => {
                        // FIXME: unwrap
                        let name = self.config.get_sections()[id as usize].get("name").unwrap();
                        let future = self.storage.retrieve_state(self.id, id, name.as_str().unwrap().to_string());
                        reply_to.send(future.await?).ok();
                    },
                    Command::Log{ .. } => {
                    },
                    Command::Stopped { id } => {
                        // FIXME: add timeout
                        handles[id as usize].1.handle.take().unwrap().await??;
                    }
                    Command::Stop => {
                        // pipe ignores stop from sections
                    },
                    Command::Ack(_) => {
                        // pipe can't ack messages
                    },
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
struct HandleWrap{
    handle: Option<JoinHandle<Result<(), SectionError>>>
}

impl HandleWrap {
    fn new(handle: JoinHandle<Result<(), SectionError>>) -> Self {
        Self { handle: Some(handle) }
    }
}

impl Drop for HandleWrap {
    fn drop(&mut self) {
        if let Some(handle) = self.handle.take() { handle.abort() }
    }
}

//#[cfg(test)]
//mod test {
//    use super::*;
//    use crate::dynamic_pipe::{
//        registry::Registry,
//        section_impls::{mycelial_net, sqlite}, section::State,
//    };
//    use std::future::Future;
//
//    #[derive(Debug, Clone)]
//    pub struct NoopStorage{}
//
//    impl Storage for NoopStorage {
//        fn store_state(
//            &self,
//            _id: u64,
//            _section_id: u64,
//            _section_name: String,
//            _state: State
//        ) -> Pin<Box<dyn Future<Output=Result<(), SectionError>> + Send + 'static>> {
//            Box::pin(async { Ok(()) })
//        }
//
//        fn retrieve_state(
//            &self,
//            _id: u64,
//            _section_id: u64,
//            _section_name: String,
//        ) -> Pin<Box<dyn Future<Output=Result<Option<State>, SectionError>> + Send + 'static>> {
//            Box::pin(async { Ok(None) })
//        }
//    }
//
//    #[tokio::test]
//    async fn test() {
//        // pipe configuration with 2 sections
//        let config = r#"
//[[section]]
//name = "sqlite"
//path = ":memory:"
//query = "SELECT * FROM sqlite_master"
//
//[[section]]
//name = "mycelial_net"
//endpoint = "http://localhost:8080/ingestion"
//token = "token"
//"#;
//        let cfg: Config = Config::try_from_toml(config).unwrap();
//        let mut registry = Registry::new();
//        registry.register_section("sqlite", sqlite::source::constructor);
//        registry.register_section("mycelial_net", mycelial_net::destination::constructor);
//
//        let pipe = Pipe::try_from((0, cfg, &registry, NoopStorage{} )).unwrap();
//        let _f = Box::new(pipe).start(Stub::<_, SectionError>::new(), Stub::<_, SectionError>::new(), ());
//    }
//}

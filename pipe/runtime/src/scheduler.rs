//! Pipe scheduler

use crate::storage::Storage;
use crate::{config::Config, pipe::Pipe, registry::Registry};

use section::{
    command_channel::{Command, ReplyTo, RootChannel, SectionChannel, SectionRequest},
    section::Section,
    SectionError,
};
use std::collections::HashMap;
use std::time::Duration;
use stub::Stub;
use tokio::sync::mpsc::WeakSender;
use tokio::task::JoinHandle;
use tokio::{
    sync::mpsc::{channel, Receiver, Sender},
    sync::oneshot::{channel as oneshot_channel, Sender as OneshotSender},
};

#[allow(dead_code)]
pub struct Scheduler<T: Storage<<R::SectionChannel as SectionChannel>::State>, R: RootChannel> {
    registry: Registry<<R as RootChannel>::SectionChannel>,
    storage: T,
    pipe_configs: HashMap<u64, Config>,
    pipes: HashMap<u64, Option<JoinHandle<Result<(), SectionError>>>>,
    root_chan: R,
}

#[derive(Debug)]
pub enum Message {
    /// Add new pipe to schedule
    AddPipe {
        id: u64,
        config: Config,
        reply_to: OneshotSender<Result<ScheduleResult, SectionError>>,
    },

    /// Remove pipe
    RemovePipe {
        id: u64,
        reply_to: OneshotSender<Result<(), SectionError>>,
    },

    /// Shutdown scheduler and all pipes
    Shutdown {
        reply_to: OneshotSender<Result<(), SectionError>>,
    },

    /// List pipe ids
    ListIds {
        reply_to: OneshotSender<Result<Vec<u64>, SectionError>>,
    },

    /// Reschedule pipe
    Reschedule { id: u64 },
}

#[derive(Debug)]
pub enum ScheduleResult {
    /// New pipe was scheduler
    New,

    /// Pipe was updated with newer config
    Updated,

    /// Pipe was re-added with same config
    Noop,
}

impl<T, R> Scheduler<T, R>
where
    R: RootChannel,
    T: Storage<<R::SectionChannel as SectionChannel>::State>
        + std::fmt::Debug
        + Clone
        + Send
        + 'static,
{
    pub fn new(registry: Registry<R::SectionChannel>, storage: T) -> Self {
        Self {
            registry,
            storage,
            pipe_configs: HashMap::new(),
            pipes: HashMap::new(),
            root_chan: RootChannel::new(),
        }
    }

    pub fn spawn(mut self) -> SchedulerHandle {
        let (tx, mut rx) = channel(8);
        let weak_tx = tx.clone().downgrade();
        tokio::spawn(async move { self.enter_loop(&mut rx, weak_tx).await });
        SchedulerHandle { tx }
    }

    async fn enter_loop(
        &mut self,
        rx: &mut Receiver<Message>,
        weak_tx: WeakSender<Message>,
    ) -> Result<(), SectionError> {
        loop {
            tokio::select! {
                message = rx.recv() => {
                    let message = match message {
                        Some(message) => message,
                        None => return Ok(()) // scheduler handle was dropped
                    };
                    match message {
                        Message::Shutdown { reply_to } => {
                            reply_to.send(Ok(())).ok();
                            return Ok(());
                        }
                        Message::AddPipe {
                            id,
                            config,
                            reply_to,
                        } => {
                            reply_to.send(self.add_pipe(id, config).await).ok();
                        }
                        Message::RemovePipe { id, reply_to } => {
                            self.remove_pipe(id).await;
                            reply_to.send(Ok(())).ok();
                        }
                        Message::ListIds { reply_to } => {
                            reply_to
                                .send(Ok(self.pipe_configs.keys().copied().collect()))
                                .ok();
                        }
                        Message::Reschedule{ id } => {
                            if !self.pipes.contains_key(&id) {
                                self.schedule(id).ok();
                            }
                        }
                    };
                },
                req = self.root_chan.recv() => {
                    match req? {
                        SectionRequest::RetrieveState { id, reply_to } => {
                            reply_to.reply(self.storage.retrieve_state(id).await?).await.ok();
                        },
                        SectionRequest::StoreState { id, state, reply_to } => {
                            self.storage.store_state(id, state).await?;
                            reply_to.reply(()).await.ok();
                        },
                        SectionRequest::Log { id, message } => {
                            // FIXME: use proper logger
                            log::info!("pipe<{id}>: {message}");
                        },
                        SectionRequest::Stopped{ id } => {
                            let finished = match self.pipes.get(&id) {
                                Some(Some(handle)) => handle.is_finished(),
                                _ => true,
                            };
                            if finished {
                                if let Err(err) = self.retrieve_pipe_error(id).await {
                                    log::error!("pipe with id: {id} stopped: {:?}", err);
                                };
                                self.unschedule(id).await;
                                self.reschedule(id, weak_tx.clone());
                            }
                        },
                        _ => {},
                    };
                }
            }
        }
    }

    async fn add_pipe(&mut self, id: u64, config: Config) -> Result<ScheduleResult, SectionError> {
        let schedule_result = match self.pipe_configs.get(&id) {
            Some(c) if c == &config => return Ok(ScheduleResult::Noop),
            Some(_) => {
                self.remove_pipe(id).await;
                ScheduleResult::Updated
            }
            None => ScheduleResult::New,
        };
        self.pipe_configs.insert(id, config);
        self.schedule(id).map(|_| schedule_result)
    }

    async fn remove_pipe(&mut self, id: u64) {
        self.pipe_configs.remove(&id);
        self.unschedule(id).await;
    }

    fn schedule(&mut self, id: u64) -> Result<(), SectionError> {
        if let Some(config) = self.pipe_configs.get(&id).cloned() {
            let pipe = Pipe::<R>::try_from((&config, &self.registry))?;
            let section_chan = self.root_chan.add_section(id)?;
            let pipe = pipe.start(
                Stub::<_, SectionError>::new(),
                Stub::<_, SectionError>::new(),
                section_chan,
            );
            let handle = Some(tokio::spawn(pipe));
            self.pipes.insert(id, handle);
        }
        Ok(())
    }

    /// Stop pipe by removing it from pipes list
    async fn unschedule(&mut self, pipe_id: u64) {
        self.root_chan.send(pipe_id, Command::Stop).await.ok();
        if let Some(Some(handle)) = self.pipes.remove(&pipe_id) {
            handle.abort();
        }
        self.root_chan.remove_section(pipe_id).ok();
    }

    /// reschedule failed pipe
    fn reschedule(&self, id: u64, weak_tx: WeakSender<Message>) {
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_secs(3)).await;
            if let Some(tx) = weak_tx.upgrade() {
                tx.send(Message::Reschedule { id }).await.ok();
            }
        });
    }

    /// retrieve pipe error, if any
    async fn retrieve_pipe_error(&mut self, pipe_id: u64) -> Result<(), SectionError> {
        match self.pipes.get_mut(&pipe_id) {
            Some(handle) if handle.is_some() => {
                if handle.as_mut().map(|handle| handle.is_finished()).unwrap() {
                    let handle = handle.take().unwrap();
                    handle.await?
                } else {
                    Ok(())
                }
            }
            _ => Ok(()),
        }
    }
}

#[allow(dead_code)] // fixme
#[derive(Debug, Clone)]
pub struct SchedulerHandle {
    tx: Sender<Message>,
}

// call macro:
// - crates new oneshot channel
// - assembles message, appends reply_to
// - sends message to and awaits response
macro_rules! call {
    ($self:ident, $ty:tt :: $arm:tt { $($field:tt: $value:expr),* $(,)?} ) => {
        {
            let (reply_to, rx) = oneshot_channel();
            $self.send($ty::$arm{
                $($field: $value,)*
                reply_to,
            }).await?;
            rx.await?
        }
    };
    // shortcut struct init
    ($self:ident, $ty:tt :: $arm:tt { $($field:tt),* $(,)?} ) => {
        {
            let (reply_to, rx) = oneshot_channel();
            $self.send($ty::$arm{
                $($field,)*
                reply_to,
            }).await?;
            rx.await?
        }
    }
}

#[allow(dead_code)] // fixme
impl SchedulerHandle {
    /// Schedule new pipe
    ///
    /// If pipe id already exists - scheduler will check configuration:
    /// * if configuration is equal - nothing will happen
    /// * if configuration differs - pipe with old config will be replaced with new pipe
    pub async fn add_pipe(&self, id: u64, config: Config) -> Result<ScheduleResult, SectionError> {
        call!(self, Message::AddPipe { id, config })
    }

    /// Remove pipe
    pub async fn remove_pipe(&self, id: u64) -> Result<(), SectionError> {
        call!(self, Message::RemovePipe { id })
    }

    /// List pipes ids
    pub async fn list_ids(&self) -> Result<Vec<u64>, SectionError> {
        call!(self, Message::ListIds {})
    }

    /// Shutdown scheduler
    pub async fn shutdown(self) -> Result<(), SectionError> {
        call!(self, Message::Shutdown {})
    }

    async fn send(&self, message: Message) -> Result<(), SectionError> {
        Ok(self.tx.send(message).await?)
    }
}

//! Pipe scheduler

use crate::state::State;
use crate::{
    types::SectionError,
    config::Config,
    registry::Registry,
    pipe::Pipe,
};
use std::future::{Future, IntoFuture};
use std::pin::Pin;
use std::{collections::HashMap, time::Duration};
use tokio::{
    sync::mpsc::{channel, Receiver, Sender},
    sync::{
        mpsc::WeakSender,
        oneshot::{channel as oneshot_channel, Sender as OneshotSender},
    },
};

pub trait Storage {
    fn store_state(
        &self,
        id: u64,
        section_id: u64,
        section_name: String,
        state: State
    ) -> Pin<Box<dyn Future<Output=Result<(), SectionError>> + Send + 'static>>;

    fn retrieve_state(
        &self,
        id: u64,
        section_id: u64,
        section_name: String,
    ) -> Pin<Box<dyn Future<Output=Result<Option<State>, SectionError>> + Send + 'static>>;
}

pub struct Scheduler<Storage> {
    registry: Registry,
    storage: Storage,
    pipe_configs: HashMap<u64, Config>,
    pipes: HashMap<u64, OneshotSender<()>>,
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

    /// Notify scheduler on pipe down
    PipeDown { id: u64 },

    /// List pipe ids
    ListIds {
        reply_to: OneshotSender<Result<Vec<u64>, SectionError>>,
    },
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

impl<T: Storage + Clone + Send + 'static> Scheduler<T> {
    pub fn new(registry: Registry, storage: T) -> Self {
        Self {
            registry,
            storage,
            pipe_configs: HashMap::new(),
            pipes: HashMap::new(),
        }
    }

    pub fn spawn(mut self) -> SchedulerHandle {
        let (tx, mut rx) = channel(8);
        let weak_tx = tx.clone().downgrade();
        tokio::spawn(async move {
            self.enter_loop(&mut rx, weak_tx).await;
        });
        SchedulerHandle { tx }
    }

    async fn enter_loop(&mut self, rx: &mut Receiver<Message>, tx: WeakSender<Message>) {
        while let Some(message) = rx.recv().await {
            match message {
                Message::Shutdown { reply_to } => {
                    reply_to.send(Ok(())).ok();
                    return;
                }
                Message::AddPipe {
                    id,
                    config,
                    reply_to,
                } => {
                    reply_to.send(self.add_pipe(id, config, tx.clone())).ok();
                }
                Message::RemovePipe { id, reply_to } => {
                    self.remove_pipe(id);
                    reply_to.send(Ok(())).ok();
                }
                Message::PipeDown { id } => {
                    self.unschedule(id);
                    self.schedule(id, tx.clone()).ok();
                }
                Message::ListIds { reply_to } => {
                    reply_to
                        .send(Ok(self.pipe_configs.keys().copied().collect()))
                        .ok();
                }
            };
        }
    }

    pub fn add_pipe(
        &mut self,
        id: u64,
        config: Config,
        tx: WeakSender<Message>,
    ) -> Result<ScheduleResult, SectionError> {
        let schedule_result = match self.pipe_configs.get(&id) {
            Some(c) if c == &config => return Ok(ScheduleResult::Noop),
            Some(_) => {
                self.remove_pipe(id);
                ScheduleResult::Updated
            }
            None => ScheduleResult::New,
        };
        self.pipe_configs.insert(id, config);
        self.schedule(id, tx).map(|_| schedule_result)
    }

    pub fn remove_pipe(&mut self, id: u64) {
        self.pipe_configs.remove(&id);
        self.unschedule(id);
    }

    fn schedule(&mut self, id: u64, tx: WeakSender<Message>) -> Result<(), SectionError> {
        // FIXME: error reporting
        if let Some(config) = self.pipe_configs.get(&id).cloned() {
            let pipe = Pipe::try_from((id, config.clone(), &self.registry, self.storage.clone()))?;
            let rx = spawn(id, tx, pipe.into_future());
            self.pipes.insert(id, rx);
        }
        Ok(())
    }

    /// Stop pipe by removing it from pipes list
    fn unschedule(&mut self, pipe_id: u64) {
        if let Some(handle) = self.pipes.remove(&pipe_id) {
            drop(handle)
        }
    }
}

/// spawn pipe
///
/// if future exited before rx channel - notify scheduler that pipe did die
fn spawn(
    id: u64,
    scheduler_tx: WeakSender<Message>,
    future: impl Future<Output = Result<(), SectionError>> + Send + 'static,
) -> OneshotSender<()> {
    let (tx, rx) = oneshot_channel();
    tokio::spawn(async move {
        println!("pipe with id {id} spawned");
        tokio::select! {
            _ = rx => {
                println!("pipe exit: join handle dropped")
            },
            res = future => {
                println!("pipe exit: pipe finished: {:?}", res);
                if let Some(scheduler_tx) = scheduler_tx.upgrade() {
                    // FIXME: delay added to disallow rescheduling spam
                    tokio::time::sleep(Duration::from_secs(3)).await;
                    scheduler_tx.send(Message::PipeDown{ id }).await.ok();
                }
            }
        };
    });
    tx
}

#[derive(Debug, Clone)]
pub struct SchedulerHandle {
    tx: Sender<Message>,
}

// call macro:
// - crates new oneshot channel
// - assembles message, appends reply_to
// - sends message to and awaits response
//
// reduces boilerplate a bit
// named after gen_server:call
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

impl SchedulerHandle {
    /// Schedule new pipe
    ///
    /// If pipe id already exists - scheduler will check configuration:
    /// * if configuration is equal - nothing will happen
    /// * if configuration differs - pipe with old config will be replaced with new pipe
    pub async fn add_pipe(
        &self,
        id: u64,
        config: Config,
    ) -> Result<ScheduleResult, SectionError> {
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

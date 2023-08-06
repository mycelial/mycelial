//! Runtime
//!
//! Pipe scheduling and peristance

use exp2::dynamic_pipe::{
    registry::{Constructor, Registry},
    scheduler::{Scheduler, SchedulerHandle, ScheduleResult},
    section_impls::{mycelial_net, sqlite, kafka, snowflake}, config::Config, section::{self, State},
};
use tokio::sync::{
    mpsc::{channel, Sender, Receiver},
    oneshot::{channel as oneshot_channel, Sender as OneshotSender},
};

use crate::{storage::StorageHandle, call};

/// Setup & populate registry
fn setup_registry() -> Registry {
    let arr: &[(&str, Constructor)] = &[
        ("sqlite_source", sqlite::source::constructor),
        ("sqlite_destination", sqlite::destination::constructor),
        ("mycelial_net_source", mycelial_net::source::constructor),
        ("mycelial_net_destination", mycelial_net::destination::constructor),
        ("kafka_source", kafka::source::constructor),
        ("snowflake_source", snowflake::source::constructor),
        ("snowflake_destination", snowflake::destination::constructor),
    ];
    arr.iter()
        .fold(Registry::new(), |mut acc, &(section_name, constructor)| {
            acc.register_section(section_name, constructor);
            acc
        })
}

pub struct Runtime {
    scheduler: SchedulerHandle,
    storage: StorageHandle,
}


impl Runtime {
    pub fn new(storage: StorageHandle) -> Self {
        Self {
            scheduler: Scheduler::new(setup_registry()).spawn(),
            storage,
        }
    }

    pub fn spawn(mut self) -> RuntimeHandle {
        let (tx, mut rx) = channel(1);
        tokio::spawn(async move { self.enter_loop(&mut rx).await });
        RuntimeHandle{ tx }
    }

    async fn enter_loop(&mut self, receiver: &mut Receiver<Message>) {
        while let Some(msg) = receiver.recv().await { 
            match msg {
                Message::ListIds { reply_to } => {
                    let handle = self.scheduler.clone();
                    tokio::spawn(async move { reply_to.send(handle.list_ids().await).ok(); });
                },
                Message::AddPipe { id, config, reply_to } => {
                    let handle = self.scheduler.clone();
                    tokio::spawn(async move { reply_to.send(handle.add_pipe(id, config).await).ok(); });
                },
                Message::RemovePipe { id, reply_to } => {
                    let handle = self.scheduler.clone();
                    tokio::spawn(async move { reply_to.send(handle.remove_pipe(id).await).ok(); });
                },
                Message::StoreState { id, section_id, section_name, state, reply_to } => {
                },
                Message::RetrieveState { id, section_id, section_name, reply_to } => {
                } 
            }
        }
    }
}

#[derive(Debug)]
pub struct RuntimeHandle {
    tx: Sender<Message>
}

#[derive(Debug)]
enum Message {
    ListIds {
        reply_to: OneshotSender<Result<Vec<u64>, section::Error>>,
    },
    AddPipe {
        id: u64,
        config: Config,
        reply_to: OneshotSender<Result<ScheduleResult, section::Error>>,
    },
    RemovePipe {
        id: u64,
        reply_to: OneshotSender<Result<(), section::Error>>,
    },
    StoreState {
        id: u64,
        section_id: u64,
        section_name: String,
        state: State,
        reply_to: OneshotSender<Result<(), section::Error>>,
    },
    RetrieveState {
        id: u64,
        section_id: u64,
        section_name: String,
        reply_to: OneshotSender<Result<Option<State>, section::Error>>
    }
}

impl RuntimeHandle {
    pub async fn list_ids(&self) -> Result<Vec<u64>, section::Error> {
        call!(self, Message::ListIds{})
    }

    pub async fn add_pipe(&self, id: u64, config: Config) -> Result<ScheduleResult, section::Error> {
        call!(self, Message::AddPipe{id, config})
    }

    pub async fn remove_pipe(&self, id: u64) -> Result<(), section::Error> {
        call!(self, Message::RemovePipe{id})
    }

    pub async fn store_state(&self, id: u64, section_id: u64, section_name: &str, state: State) -> Result<(), section::Error> {
        call!(self, Message::StoreState{
            id: id,
            section_id: section_id,
            section_name: section_name.into(),
            state: state
        })
    }

    pub async fn retrieve_state(&self, id: u64, section_id: u64, section_name: &str) -> Result<Option<State>, section::Error> {
        call!(self, Message::RetrieveState{
            id: id,
            section_id: section_id,
            section_name: section_name.into(),
        })
    }

    async fn send(&self, message: Message) -> Result<(), section::Error> {
        Ok(self.tx.send(message).await?)
    }
}

pub fn new(storage: StorageHandle) -> RuntimeHandle {
    Runtime::new(storage).spawn()
}

//! storage backend for client


use std::str::FromStr;

use exp2::dynamic_pipe::section::{self, State};
use sqlx::{sqlite::SqliteConnectOptions, ConnectOptions, SqliteConnection};
use tokio::sync::{
    mpsc::{channel, Sender, Receiver},
    oneshot::{channel as oneshot_channel, Sender as OneshotSender},
};

use crate::call;

pub struct Storage {
    path: String,
    connection: SqliteConnection,
}

impl Storage {
    pub async fn new(path: impl Into<String>) -> Result<Self, section::Error> {
        let path = path.into();
        let mut connection = SqliteConnectOptions::from_str(path.as_str())?
            .create_if_missing(true)
            .connect()
            .await?;
        sqlx::migrate!().run(&mut connection).await?;
        Ok(Self { path, connection })
    }

    pub fn spawn(mut self) -> StorageHandle {
        let (tx, mut rx) = channel::<Message>(1);
        tokio::spawn(async move { self.enter_loop(&mut rx).await });
        StorageHandle { tx }
    }

    async fn enter_loop(&mut self, rx: &mut Receiver<Message>) -> Result<(), section::Error> {
        while let Some(msg) = rx.recv().await {
            match msg {
                Message::StoreState { id, section_id, section_name, state, reply_to } => {
                    println!("storing state for pipe {id}/{section_id}/{section_name}: {state:?}");
                    reply_to.send(Ok(())).ok();
                },
                Message::RetrieveState { id, section_id, section_name, reply_to } => {
                    println!("retrieving state for pipe {id}/{section_id}/{section_name}");
                    reply_to.send(Ok(None)).ok();
                },
            }
        };
        Ok(())
    }
}

#[derive(Debug)]
pub enum Message {
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

pub struct StorageHandle {
    tx: Sender<Message>
}

impl StorageHandle {
    pub async fn store_state(&self, id: u64, section_id: u64, section_name: String, state: State) -> Result<(), section::Error> {
        call!(self, Message::StoreState{ id, section_id, section_name, state })
    }

    pub async fn retrieve_state(&self, id: u64, section_id: u64, section_name: String) -> Result<Option<State>, section::Error> {
        call!(self, Message::RetrieveState{ id, section_id, section_name })
    }


    async fn send(&self, message: Message) -> Result<(), section::Error> {
        Ok(self.tx.send(message).await?)
    }
}


pub async fn new(storage_path: String) -> Result<StorageHandle, section::Error> {
    Ok(Storage::new(storage_path).await?.spawn())
}

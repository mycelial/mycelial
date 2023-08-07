//! storage backend for client


use std::{str::FromStr, pin::Pin};
use std::future::Future;
use exp2::call;
use exp2::dynamic_pipe::{
    scheduler::Storage,
    section::{self, State},
};
use sqlx::{sqlite::SqliteConnectOptions, ConnectOptions, SqliteConnection, Row};
use tokio::sync::{
    mpsc::{channel, Sender, Receiver},
    oneshot::{channel as oneshot_channel, Sender as OneshotSender},
};

pub struct SqliteStorage {
    #[allow(unused)]
    path: String,
    connection: SqliteConnection,
}

impl SqliteStorage {
    pub async fn new(path: impl Into<String>) -> Result<Self, section::Error> {
        let path = path.into();
        let mut connection = SqliteConnectOptions::from_str(path.as_str())?
            .create_if_missing(true)
            .connect()
            .await?;
        sqlx::migrate!().run(&mut connection).await?;
        Ok(Self { path, connection })
    }

    pub fn spawn(mut self) -> SqliteStorageHandle {
        let (tx, mut rx) = channel::<Message>(1);
        tokio::spawn(async move { self.enter_loop(&mut rx).await });
        SqliteStorageHandle { tx }
    }

    async fn enter_loop(&mut self, rx: &mut Receiver<Message>) -> Result<(), section::Error> {
        while let Some(msg) = rx.recv().await {
            match msg {
                Message::StoreState { id, section_id, section_name, state, reply_to } => {
                    let result = sqlx::query(
                        "INSERT INTO state VALUES(?, ?, ?, ?) ON CONFLICT (id, section_id, section_name) DO UPDATE SET state = excluded.state"
                    )
                        .bind(id as i64)
                        .bind(section_id as i64)
                        .bind(section_name)
                        .bind(state.serialize().unwrap())
                        .execute(&mut self.connection)
                        .await
                        .map(|_| ())
                        .map_err(|e| e.into());
                    reply_to.send(result).ok();
                },
                Message::RetrieveState { id, section_id, section_name, reply_to } => {
                    let result = sqlx::query(
                        "SELECT state FROM state WHERE id = ? and section_id = ? and section_name = ?"
                    )
                        .bind(id as i64)
                        .bind(section_id as i64)
                        .bind(section_name)
                        .fetch_optional(&mut self.connection)
                        .await
                        .map(|row| row.map(|val| State::deserialize(&val.get::<String, _>("state")).unwrap() ))
                        .map_err(|e| e.into());
                    reply_to.send(result).ok();
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

#[derive(Clone)]
pub struct SqliteStorageHandle {
    tx: Sender<Message>
}

impl SqliteStorageHandle {
    async fn send(&self, message: Message) -> Result<(), section::Error> {
        Ok(self.tx.send(message).await?)
    }
}

impl Storage for SqliteStorageHandle {
    fn store_state(
        &self,
        id: u64,
        section_id: u64,
        section_name: String,
        state: State
    ) -> Pin<Box<dyn Future<Output=Result<(), section::Error>> + Send + 'static>> {
        let this = self.clone();
        Box::pin(async move { call!(this, Message::StoreState{ id, section_id, section_name, state }) })
    }

    fn retrieve_state(
        &self, 
        id: u64,
        section_id: u64,
        section_name: String
    ) -> Pin<Box<dyn Future<Output=Result<Option<State>, section::Error>> + Send + 'static>> {
        let this = self.clone();
        Box::pin(async move { call!(this, Message::RetrieveState{ id, section_id, section_name }) })
    }

}


pub async fn new(storage_path: String) -> Result<SqliteStorageHandle, section::Error> {
    Ok(SqliteStorage::new(storage_path).await?.spawn())
}

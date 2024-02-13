//! storage backend for sections

use pipe::storage::Storage;
use section::state::State;
use serde::{Deserialize, Serialize};
use sqlx::{sqlite::SqliteConnectOptions, ConnectOptions, Row, SqliteConnection};
use std::any::{type_name, Any, TypeId};
use std::future::Future;
use std::{pin::Pin, str::FromStr};
use tokio::sync::{
    mpsc::{channel, Receiver, Sender},
    oneshot::{channel as oneshot_channel, Sender as OneshotSender},
};

pub type StdError = Box<dyn std::error::Error + Send + Sync + 'static>;

pub struct SqliteStorage {
    #[allow(unused)]
    path: String,
    connection: SqliteConnection,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SqliteState {
    map: serde_json::Map<String, serde_json::Value>,
}

#[derive(Debug)]
pub enum SqliteStateError {
    UnsupportedType { id: TypeId, type_name: &'static str },
}

impl std::fmt::Display for SqliteStateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl std::error::Error for SqliteStateError {}

impl State for SqliteState {
    type Error = SqliteStateError;

    fn new() -> Self {
        Self {
            map: serde_json::Map::new(),
        }
    }

    fn get<T: Clone + Send + Sync + 'static>(&self, key: &str) -> Result<Option<T>, Self::Error> {
        let value = match self.map.get(key) {
            None => return Ok(None),
            Some(value) => value,
        };
        let type_id = TypeId::of::<T>();
        match value {
            serde_json::Value::String(s) if TypeId::of::<String>() == type_id => {
                let any = s as &dyn Any;
                Ok(any.downcast_ref().cloned())
            }
            serde_json::Value::Number(num) if TypeId::of::<u64>() == type_id => {
                let num = match num.as_u64() {
                    None => return Ok(None),
                    Some(num) => num,
                };
                let any = &num as &dyn Any;
                Ok(any.downcast_ref().cloned())
            }
            serde_json::Value::Number(num) if TypeId::of::<i64>() == type_id => {
                let num = match num.as_i64() {
                    None => return Ok(None),
                    Some(num) => num,
                };
                let any = &num as &dyn Any;
                Ok(any.downcast_ref().cloned())
            }
            serde_json::Value::Object(map) if TypeId::of::<Self>() == type_id => {
                let any = &SqliteState { map: map.clone() } as &dyn Any;
                Ok(any.downcast_ref().cloned())
            }
            _ => {
                // FIXME: this one points to foot
                Ok(None)
            }
        }
    }

    fn set<T: Clone + Send + Sync + 'static>(
        &mut self,
        key: &str,
        mut value: T,
    ) -> Result<(), Self::Error> {
        let any = &mut value as &mut dyn Any;
        let type_id = TypeId::of::<T>();
        let value = match type_id {
            t if t == TypeId::of::<String>() => {
                let string: &String = any.downcast_ref().unwrap();
                serde_json::Value::String(string.clone())
            }
            t if t == TypeId::of::<u64>() => {
                let num: &u64 = any.downcast_ref().unwrap();
                serde_json::Value::Number((*num).into())
            }
            t if t == TypeId::of::<i64>() => {
                let num: &i64 = any.downcast_ref().unwrap();
                serde_json::Value::Number((*num).into())
            }
            t if t == TypeId::of::<SqliteState>() => {
                let sqlite_state: &mut SqliteState = any.downcast_mut().unwrap();
                let map = std::mem::replace(&mut sqlite_state.map, serde_json::Map::new());
                serde_json::Value::Object(map)
            }
            _ => Err(SqliteStateError::UnsupportedType {
                id: type_id,
                type_name: type_name::<T>(),
            })?,
        };
        self.map.insert(key.to_string(), value);
        Ok(())
    }
}

impl SqliteStorage {
    pub async fn new(path: impl Into<String>) -> Result<Self, StdError> {
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

    async fn enter_loop(&mut self, rx: &mut Receiver<Message>) -> Result<(), StdError> {
        while let Some(msg) = rx.recv().await {
            match msg {
                Message::StoreState {
                    pipe_id,
                    state,
                    reply_to,
                } => {
                    let result = sqlx::query(
                        "INSERT INTO state VALUES(?, ?) ON CONFLICT (id) DO UPDATE SET state = excluded.state"
                    )
                        .bind(pipe_id as i64)
                        .bind(serde_json::to_string(&serde_json::Value::Object(state.map))?)
                        .execute(&mut self.connection)
                        .await
                        .map(|_| ())
                        .map_err(|e| e.into());
                    reply_to.send(result).ok();
                }

                Message::RetrieveState { pipe_id, reply_to } => {
                    let result = sqlx::query("SELECT state FROM state WHERE id = ?")
                        .bind(pipe_id as i64)
                        .fetch_optional(&mut self.connection)
                        .await
                        .map(|row| {
                            row.map(|val| {
                                match serde_json::from_str(&val.get::<String, _>("state")) {
                                    Ok(serde_json::Value::Object(map)) => SqliteState { map },
                                    _ => SqliteState::new(),
                                }
                            })
                        })
                        .map_err(|e| e.into());
                    reply_to.send(result).ok();
                }
            }
        }
        Ok(())
    }
}

#[derive(Debug)]
pub enum Message {
    StoreState {
        pipe_id: u64,
        state: SqliteState,
        reply_to: OneshotSender<Result<(), StdError>>,
    },
    RetrieveState {
        pipe_id: u64,
        reply_to: OneshotSender<Result<Option<SqliteState>, StdError>>,
    },
}

#[derive(Debug, Clone)]
pub struct SqliteStorageHandle {
    tx: Sender<Message>,
}

impl SqliteStorageHandle {
    async fn send(&self, message: Message) -> Result<(), StdError> {
        Ok(self.tx.send(message).await?)
    }
}

impl Storage<SqliteState> for SqliteStorageHandle {
    fn store_state(
        &self,
        pipe_id: u64,
        state: SqliteState,
    ) -> Pin<Box<dyn Future<Output = Result<(), StdError>> + Send + 'static>> {
        let this = self.clone();
        Box::pin(async move {
            let (reply_to, rx) = oneshot_channel();
            this.send(Message::StoreState {
                pipe_id,
                state,
                reply_to,
            })
            .await?;
            rx.await?
        })
    }

    fn retrieve_state(
        &self,
        pipe_id: u64,
    ) -> Pin<Box<dyn Future<Output = Result<Option<SqliteState>, StdError>> + Send + 'static>> {
        let this = self.clone();
        Box::pin(async move {
            let (reply_to, rx) = oneshot_channel();
            this.send(Message::RetrieveState { pipe_id, reply_to })
                .await?;
            rx.await?
        })
    }
}

pub async fn new(storage_path: String) -> Result<SqliteStorageHandle, StdError> {
    Ok(SqliteStorage::new(storage_path).await?.spawn())
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_sqlite_state() {
        let mut state = SqliteState::new();

        // set key and retrieve key as a string
        state.set("key", "value".to_string()).unwrap();
        assert_eq!("value", state.get::<String>("key").unwrap().unwrap());
        assert_eq!(None, state.get::<u64>("key").unwrap());

        // set key and retrieve key as a u64
        state.set("key", 64_u64).unwrap();
        assert_eq!(64, state.get::<u64>("key").unwrap().unwrap());
        assert_eq!(None, state.get::<String>("key").unwrap());

        // set key and retrieve key as a i64
        state.set("key", 64_i64).unwrap();
        assert_eq!(64, state.get::<i64>("key").unwrap().unwrap());
        assert_eq!(None, state.get::<String>("key").unwrap());
    }
}

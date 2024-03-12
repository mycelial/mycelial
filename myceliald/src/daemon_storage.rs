//! storage backend for daemon
//!

use anyhow::Context;
use common::PipeConfig;
use serde::{Deserialize, Serialize};
use sqlx::{sqlite::SqliteConnectOptions, ConnectOptions, Connection, Row, SqliteConnection};
use std::path::Path;
use tokio::sync::{
    mpsc::{channel, Receiver, Sender},
    oneshot::{channel as oneshot_channel, Sender as OneshotSender},
};

pub struct DaemonStorage {
    connection: SqliteConnection,
}

#[derive(Debug, Default)]
pub struct Credentials {
    pub client_id: String,
    pub client_secret: String,
}

#[derive(Debug, Default)]
pub struct ServerInfo {
    pub endpoint: String,
    pub token: String,
}

#[derive(Debug, Default)]
pub struct DaemonInfo {
    pub display_name: String,
    pub unique_id: String,
}

impl DaemonStorage {
    pub async fn new(path: &Path) -> anyhow::Result<Self> {
        let mut connection = SqliteConnectOptions::new()
            .filename(path)
            .create_if_missing(true)
            .connect()
            .await?;
        sqlx::migrate!().run(&mut connection).await?;
        tracing::info!("connected to {path:?}");
        Ok(Self { connection })
    }

    pub async fn store_daemon_info(
        &mut self,
        display_name: &str,
        unique_id: &str,
    ) -> anyhow::Result<()> {
        sqlx::query(
            "INSERT INTO daemon VALUES ('display_name', ?), ('unique_id', ?) \
            ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        )
        .bind(display_name)
        .bind(unique_id)
        .execute(&mut self.connection)
        .await?;
        Ok(())
    }

    pub async fn retrieve_daemon_info(&mut self) -> anyhow::Result<Option<DaemonInfo>> {
        let reply = sqlx::query("SELECT key, value FROM daemon")
            .fetch_all(&mut self.connection)
            .await?
            .into_iter()
            .fold(None, |daemon_info, row| match row.get(0) {
                "display_name" => Some(DaemonInfo {
                    display_name: row.get(1),
                    ..(daemon_info.unwrap_or_default())
                }),
                "unique_id" => Some(DaemonInfo {
                    unique_id: row.get(1),
                    ..(daemon_info.unwrap_or_default())
                }),
                key => {
                    tracing::error!("bad daemon info key: {}", key);
                    daemon_info
                }
            });
        Ok(reply)
    }

    pub async fn store_http_credentials(
        &mut self,
        client_id: String,
        client_secret: String,
    ) -> anyhow::Result<()> {
        sqlx::query(
            "INSERT INTO connection VALUES ('client_id', ?), ('client_secret', ?) \
            ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        )
        .bind(client_id)
        .bind(client_secret)
        .execute(&mut self.connection)
        .await?;
        Ok(())
    }

    pub async fn retrieve_http_credentials(&mut self) -> anyhow::Result<Option<Credentials>> {
        let reply = sqlx::query(
            "SELECT key, value FROM connection WHERE key in ('client_id', 'client_secret')",
        )
        .fetch_all(&mut self.connection)
        .await?
        .into_iter()
        .fold(None, |credentials, row| match row.get(0) {
            "client_id" => Some(Credentials {
                client_id: row.get(1),
                ..(credentials.unwrap_or_default())
            }),
            "client_secret" => Some(Credentials {
                client_secret: row.get(1),
                ..(credentials.unwrap_or_default())
            }),
            key => {
                tracing::error!("bad credentials key: {}", key);
                credentials
            }
        });
        Ok(reply)
    }

    pub async fn store_server_info(&mut self, endpoint: &str, token: &str) -> anyhow::Result<()> {
        sqlx::query(
            "INSERT INTO connection VALUES ('endpoint', ?), ('token', ?) \
            ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        )
        .bind(endpoint)
        .bind(token)
        .execute(&mut self.connection)
        .await?;
        Ok(())
    }

    pub async fn retrieve_server_info(&mut self) -> anyhow::Result<Option<ServerInfo>> {
        let reply =
            sqlx::query("SELECT key, value FROM connection WHERE key in ('endpoint', 'token')")
                .fetch_all(&mut self.connection)
                .await?
                .into_iter()
                .fold(None, |server_info, row| match row.get(0) {
                    "endpoint" => Some(ServerInfo {
                        endpoint: row.get(1),
                        ..(server_info.unwrap_or_default())
                    }),
                    "token" => Some(ServerInfo {
                        token: row.get(1),
                        ..(server_info.unwrap_or_default())
                    }),
                    key => {
                        tracing::error!("bad server info key: {}", key);
                        server_info
                    }
                });
        Ok(reply)
    }

    // FIXME: common crate
    // FIXME: there is a lot of garbage in pipe config
    pub async fn store_pipes(&mut self, configs: &[PipeConfig]) -> anyhow::Result<()> {
        let mut transaction = self.connection.begin().await?;
        for config in configs {
            sqlx::query("INSERT INTO pipe(id, config) VALUES(?, ?) ON CONFLICT DO UPDATE set config=excluded.config")
                .bind(config.id as i64)
                .bind(&serde_json::to_string(&config.pipe)?)
                .execute(&mut *transaction)
                .await?;
        }
        transaction.commit().await?;
        Ok(())
    }

    pub async fn remove_pipe(&mut self, id: u64) -> anyhow::Result<()> {
        sqlx::query("DELETE FROM pipe WHERE id=?")
            .bind(id as i64)
            .execute(&mut self.connection)
            .await?;
        Ok(())
    }

    // FIXME: common crate
    pub async fn retrieve_pipes(&mut self) -> anyhow::Result<Vec<PipeConfig>> {
        let pipes = sqlx::query("SELECT id, config FROM pipe")
            .fetch_all(&mut self.connection)
            .await?
            .into_iter()
            .filter_map(|row| {
                let id = row.get::<i64, _>(0) as u64;
                let raw_config = row.get::<String, _>(1);
                match serde_json::from_str(raw_config.as_str()) {
                    // FIXME: workspace id is just irrelevant to pipe configuration and should be
                    // stored separately, since it's metadata for UI
                    Ok(pipe) => Some(PipeConfig{ id, pipe, workspace_id: 1}),
                    Err(e) => {
                        tracing::error!("failed to parse pipe configuration, pipe_id: {id}, raw_config: {raw_config}");
                        None
                    }
                }
            })
            .collect();
        Ok(pipes)
    }

    pub async fn store_config_hash(&mut self, hash: &str) -> anyhow::Result<()> {
        sqlx::query(
            "INSERT INTO config VALUES ('config_hash', ?) ON CONFLICT(key) DO UPDATE SET value = excluded.value"
        )
            .bind(hash)
            .execute(&mut self.connection)
            .await?;
        Ok(())
    }

    pub async fn retrieve_config_hash(&mut self) -> anyhow::Result<Option<String>> {
        Ok(
            sqlx::query("SELECT value FROM config WHERE key == 'config_hash'")
                .fetch_optional(&mut self.connection)
                .await?
                .map(|row| row.get::<String, _>(0)),
        )
    }

    pub async fn retrieve_configs(&self) -> anyhow::Result<Vec<()>> {
        Ok(vec![])
    }

    pub async fn reset_state(&mut self) -> anyhow::Result<()> {
        let queries = &[
            "DELETE FROM connection",
            "DELETE FROM daemon",
            "DELETE FROM config",
            "DELETE FROM pipe",
        ];
        for query in queries {
            sqlx::query(query).execute(&mut self.connection).await?;
        }
        Ok(())
    }
}

pub async fn new(storage_path: &Path) -> anyhow::Result<DaemonStorage> {
    DaemonStorage::new(storage_path)
        .await
        .context("failed to initialized daemon storage")
}

//! storage backend for daemon
//!

use anyhow::Context;
use sqlx::{sqlite::SqliteConnectOptions, ConnectOptions, Connection, Row, SqliteConnection};
use std::path::Path;

#[derive(Debug)]
pub struct DaemonStorage {
    connection: SqliteConnection,
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

    pub async fn reset_state(&mut self) -> anyhow::Result<()> {
        let queries = &[
            "DELETE FROM connection",
            // FIXME: nodes/edges
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

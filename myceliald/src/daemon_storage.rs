//! storage backend for daemon
//!

use anyhow::{Context, Result};
use sqlx::{sqlite::SqliteConnectOptions, ConnectOptions, Connection, SqliteConnection};
use std::path::Path;

use crate::CertifiedKey;

#[derive(Debug)]
pub struct DaemonStorage {
    connection: SqliteConnection,
}

impl DaemonStorage {
    pub async fn new(path: &Path) -> Result<Self> {
        let mut connection = SqliteConnectOptions::new()
            .filename(path)
            .create_if_missing(true)
            .connect()
            .await?;
        sqlx::migrate!().run(&mut connection).await?;
        tracing::info!("connected to {path:?}");
        Ok(Self { connection })
    }

    pub async fn reset_state(&mut self) -> Result<()> {
        let queries = &[
            "DELETE FROM connection",
            // FIXME: nodes/edges
        ];
        for query in queries {
            sqlx::query(query).execute(&mut self.connection).await?;
        }
        Ok(())
    }

    pub async fn get_tls_url(&mut self) -> Result<Option<String>> {
        unimplemented!()
    }

    pub async fn get_certified_key(&mut self) -> Result<Option<CertifiedKey>> {
        unimplemented!()
    }
}

pub async fn new(storage_path: &Path) -> Result<DaemonStorage> {
    DaemonStorage::new(storage_path)
        .await
        .context("failed to initialized daemon storage")
}

//! storage backend for daemon
//!

use anyhow::{Context, Result};
use sqlx::{sqlite::SqliteConnectOptions, ConnectOptions, Connection, Row, SqliteConnection};
use std::path::Path;

use crate::CertifiedKey;

#[derive(Debug)]
pub struct DaemonStorage {
    connection: SqliteConnection,
}

const TLS_URL: &str = "tls_url";
const PRIVATE_KEY: &str = "private_key";
const CERTIFICATE: &str = "certificate";

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
        Ok(sqlx::query("SELECT value FROM connection WHERE key=?")
            .bind(TLS_URL)
            .fetch_optional(&mut self.connection)
            .await
            .map(|maybe_row| maybe_row.map(|row| row.get(0)))?)
    }

    pub async fn get_certified_key(&mut self) -> Result<Option<CertifiedKey>> {
        let (key, cert) = sqlx::query("SELECT key, value FROM connection WHERE key=? OR key=?")
            .bind(PRIVATE_KEY)
            .bind(CERTIFICATE)
            .fetch_all(&mut self.connection)
            .await?
            .into_iter()
            .fold((None, None), |(acc_key, acc_cert), row| {
                let key: String = row.get(0);
                let value: String = row.get(1);
                match key.as_str() {
                    PRIVATE_KEY => (Some(value), acc_cert),
                    CERTIFICATE => (acc_key, Some(value)),
                    _ => unreachable!("unexpected key: {key}"),
                }
            });
        match (key, cert) {
            (Some(key), Some(cert)) => Ok(Some(CertifiedKey {
                key,
                certificate: cert,
            })),
            _ => Ok(None),
        }
    }
}

pub async fn new(storage_path: &Path) -> Result<DaemonStorage> {
    DaemonStorage::new(storage_path)
        .await
        .context("failed to initialized daemon storage")
}

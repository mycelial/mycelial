//! storage backend for daemon
//!

use serde::{Deserialize, Serialize};
use sqlx::{sqlite::SqliteConnectOptions, ConnectOptions, Row, SqliteConnection};
use std::str::FromStr;

pub struct Storage {
    #[allow(unused)]
    path: String,
    connection: SqliteConnection,

    config: Option<Config>,
}

#[derive(Serialize, Deserialize)]
pub struct Config {
    client_id: String,
    client_secret: String,
}

impl Storage {
    pub async fn new(path: impl Into<String>) -> anyhow::Result<Self> {
        let path = path.into();
        let mut connection = SqliteConnectOptions::from_str(path.as_str())?
            .create_if_missing(true)
            .connect()
            .await?;
        sqlx::migrate!().run(&mut connection).await?;
        Ok(Self {
            path,
            connection,
            config: None,
        })
    }

    pub async fn write_config(&mut self, config: &Config) -> anyhow::Result<()> {
        sqlx::query("INSERT INTO client_creds (client_id, client_secret) VALUES (?, ?)")
            .bind(config.client_id.clone())
            .bind(config.client_secret.clone())
            .execute(&mut self.connection)
            .await?;
        Ok(())
    }

    pub async fn read_config(&mut self) -> anyhow::Result<Option<Config>> {
        let row = sqlx::query("SELECT client_id, client_secret FROM client_creds")
            .fetch_optional(&mut self.connection)
            .await?;
        match row {
            Some(row) => {
                let client_id: String = row.get(0);
                let client_secret: String = row.get(1);
                Ok(Some(Config {
                    client_id,
                    client_secret,
                }))
            }
            None => Ok(None),
        }
    }

    pub async fn store_client_creds(
        &mut self,
        client_id: String,
        client_secret: String,
    ) -> anyhow::Result<()> {
        self.write_config(&Config {
            client_id,
            client_secret,
        })
        .await?;
        Ok(())
    }

    pub async fn retrieve_client_creds(&mut self) -> anyhow::Result<Option<(String, String)>> {
        // Try to load the config if it's not already loaded
        if self.config.is_none() {
            if let Ok(Some(c)) = self.read_config().await {
                self.config = Some(c);
            } else {
                return Ok(None);
            }
        }

        // Assuming at this point self.config must be Some
        let c = self.config.as_ref().unwrap();
        Ok(Some((c.client_id.clone(), c.client_secret.clone())))
    }
}

pub async fn new(storage_path: String) -> anyhow::Result<Storage> {
    Storage::new(storage_path).await
}

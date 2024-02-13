//! storage backend for daemon
//!

use serde::{Deserialize, Serialize};
use std::fs;

pub struct Storage {
    #[allow(unused)]
    path: String,

    config: Option<Config>,
}

pub type StdError = Box<dyn std::error::Error + Send + Sync + 'static>;
#[derive(Serialize, Deserialize)]
pub struct Config {
    client_id: String,
    client_secret: String,
}

impl Storage {
    pub async fn new(path: impl Into<String>) -> Result<Self, StdError> {
        let path = path.into();
        Ok(Self { path, config: None })
    }

    fn read_config(&self) -> Result<Config, StdError> {
        let data = fs::read(self.path.clone())?;
        let text = String::from_utf8(data)?;
        let config: Config = toml::from_str(&text)?;
        Ok(config)
    }

    fn write_config(&self, config: &Config) -> Result<(), StdError> {
        let text = toml::to_string(config)?;
        std::fs::write(self.path.clone(), text)?;
        Ok(())
    }

    pub async fn store_client_creds(
        &mut self,
        client_id: String,
        client_secret: String,
    ) -> Result<(), StdError> {
        self.write_config(&Config {
            client_id,
            client_secret,
        })?;
        Ok(())
    }

    pub async fn retrieve_client_creds(&mut self) -> Result<Option<(String, String)>, StdError> {
        if self.config.is_none() {
            let config = self.read_config();
            match config {
                Ok(c) => {
                    self.config = Some(c);
                }
                Err(_) => {
                    return Ok(None);
                }
            }
        }

        let c = self.config.as_ref().unwrap();
        Ok(Some((c.client_id.clone(), c.client_secret.clone())))
    }
}

pub async fn new(storage_path: String) -> Result<Storage, StdError> {
    Storage::new(storage_path).await
}

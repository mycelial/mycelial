//! http client
//!
//! Poll mycelial server configuration endpoint

use std::{time::Duration, collections::HashSet};

use exp2::dynamic_pipe::{config::{Config, Value}, section, scheduler::SchedulerHandle};
use serde::Deserialize;
use serde_json::json;
use tokio::task::JoinHandle;
use base64::engine::{general_purpose::STANDARD as BASE64, Engine};

#[derive(Debug, Deserialize)]
struct ClientInfo {
    id: String,
}

#[derive(Debug, Deserialize)]
struct PipeConfigs {
    configs: Vec<PipeConfig>,
}

impl TryInto<Config> for PipeConfig {
    type Error = section::Error;

    fn try_into(self) -> Result<Config, Self::Error> {
        let value: Value = self.pipe.try_into()?;
        Config::try_from(value)
    }
}

/// Http Client
#[derive(Debug)]
struct Client {
    /// client name 
    name: String, 

    /// Mycelial server endpoint
    endpoint: String,

    /// Basic Auth token
    token: String,

    /// Client token
    client_token: String,

    /// SchedulerHandle
    scheduler_handle: SchedulerHandle,
}

/// PipeConfig
#[derive(Debug, Deserialize)]
struct PipeConfig {
    /// Scheduler needs to maintain pipe processes:
    /// - start new pipes
    /// - restart pipes on configuration update
    /// - stop pipes, when pipe was removed from configuration list
    ///
    /// To do that - each pipe config needs to be uniquely identified, so here is u64 integer to
    /// help with that.
    id: u64,

    /// # Example of config
    /// ```json
    /// {"configs": [
    ///     {
    ///         "id":1,
    ///         "pipe": {
    ///         "section": [
    ///             {
    ///                 "name": "sqlite",
    ///                 "path": "/tmp/test.sqlite",
    ///                 "query": "select * from test"
    ///             },
    ///             {
    ///                 "endpoint": "http://localhost:8080/ingestion",
    ///                 "name": "mycelial_net",
    ///                 "token": "mycelial_net_token"
    ///             }
    ///         ]
    ///     }
    /// }]}
    /// ```
    pipe: serde_json::Value,
}


fn is_for_client(config: &Config, name: &str) -> bool {
    config.get_sections().iter().any(|section | {
        match section.get("client") {
            Some(Value::String(client)) if client == name => true,
            _ => false
        }
    })
}

impl Client {
    fn new(
        name: impl Into<String>,
        endpoint: impl Into<String>,
        token: impl Into<String>,
        scheduler_handle: SchedulerHandle
    ) -> Self {
        Self {
            name: name.into(),
            endpoint: endpoint.into(),
            token: token.into(),
            client_token: "".into(),
            scheduler_handle
        }
    }

    async fn register(&mut self) -> Result<(), section::Error> {
        let client = reqwest::Client::new();
        let url = format!("{}/api/client", self.endpoint.as_str());
        let _x: ClientInfo = client
            .post(url)
            .header("Authorization", self.basic_auth())
            .json(&json!({ "id": &self.name }))
            .send()
            .await?.json().await?;

        let url = format!("{}/api/tokens", self.endpoint.as_str());
        let token: ClientInfo = client
            .post(url)
            .header("Authorization", self.basic_auth())
            .json(&json!({ "client_id": &self.name }))
            .send()
            .await?
            .json()
            .await?;

        self.client_token = token.id;
        Ok(())
    }

    async fn get_configs(&self) -> Result<Vec<PipeConfig>, section::Error> {
        let client = reqwest::Client::new();
        let url = format!("{}/api/pipe/configs", self.endpoint.as_str());
        let configs: PipeConfigs = client
            .get(url)
            .header("Authorization", self.basic_auth())
            .header("X-Authorization", self.client_auth())
            .send()
            .await?
            .json()
            .await?;
        Ok(configs.configs)
    }

    fn basic_auth(&self) -> String {
        format!("Basic {}", BASE64.encode(format!("{}:", self.token)))
    }

    fn client_auth(&self) -> String {
        format!("Bearer {}", self.client_token)
    }

    // spawns client
    pub fn spawn(mut self) -> JoinHandle<Result<(), section::Error>> {
        tokio::spawn(async move { self.enter_loop().await })
    }

    async fn enter_loop(&mut self) -> Result<(), section::Error> {
        while let Err(e) = self.register().await {
            log::error!("failed to register client: {:?}", e);
            tokio::time::sleep(Duration::from_secs(3)).await;
        }
        loop {
            
            let pipe_configs = match self.get_configs().await {
                Ok(pipe_configs) => pipe_configs,
                Err(e) => {
                    log::error!("failed to contact server: {:?}", e);
                    tokio::time::sleep(Duration::from_secs(5)).await;
                    continue
                }
            };

<<<<<<< HEAD
            log::debug!("pipe configs: {:#?}", pipe_configs);
=======
            // println!("pipe configs: {:?}", pipe_configs);

>>>>>>> 046c1cd (Add mycelite)
            let mut ids: HashSet<u64> = HashSet::from_iter(
                self.scheduler_handle.list_ids().await?.into_iter()
            );
            for pipe_config in pipe_configs.into_iter() {
                let id = pipe_config.id;
                let config: Config = match pipe_config.try_into() {
                    Ok(c) => c,
                    Err(e) => {
                        log::error!("bad pipe config: {:?}", e);
                        continue
                    }
                };
                if is_for_client(&config, &self.name) {
                    log::info!("scheduling pipe: {:#?}", &config);
                    if let Err(e) = self.scheduler_handle.add_pipe(id, config).await {
                        log::error!("failed to schedule pipe: {:?}", e);
                    }
                    ids.remove(&id);
                }
            }
            for id in ids.into_iter(){
                self.scheduler_handle.remove_pipe(id).await.unwrap()
            };

            tokio::time::sleep(Duration::from_secs(5)).await
        }
    }

    
}

pub fn new(
    name: impl Into<String>,
    endpoint: impl Into<String>,
    token: impl Into<String>,
    scheduler_handle: SchedulerHandle
) -> JoinHandle<Result<(), section::Error>> {
    Client::new(name, endpoint, token, scheduler_handle).spawn()
}

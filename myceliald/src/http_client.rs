//! http client
//!
//! Poll mycelial server configuration endpoint

use std::{collections::HashSet, time::Duration};

use base64::engine::{general_purpose::STANDARD as BASE64, Engine};
use common::{
    ClientConfig, IssueTokenRequest, IssueTokenResponse, PipeConfig, PipeConfigs,
    ProvisionClientRequest, ProvisionClientResponse,
};
use pipe::{
    config::{Config, Value},
    scheduler::SchedulerHandle,
    types::SectionError,
};
use tokio::task::JoinHandle;

/// Http Client
#[derive(Debug)]
struct Client {
    config: ClientConfig,

    /// Client token
    client_token: String,

    /// SchedulerHandle
    scheduler_handle: SchedulerHandle,
}

fn is_for_client(config: &Config, name: &str) -> bool {
    config.get_sections().iter().any(
        |section| matches!(section.get("client"), Some(Value::String(client)) if client == name),
    )
}

impl Client {
    fn new(config: ClientConfig, scheduler_handle: SchedulerHandle) -> Self {
        let client_token = config.server.token.clone();

        Self {
            config,
            client_token,
            scheduler_handle,
        }
    }

    async fn register(&mut self) -> Result<(), SectionError> {
        let client = reqwest::Client::new();
        let url = format!("{}/api/client", self.config.server.endpoint.as_str());
        let _x: ProvisionClientResponse = client
            .post(url)
            .header("Authorization", self.basic_auth())
            .json(&ProvisionClientRequest {
                client_config: self.config.clone(),
            })
            .send()
            .await?
            .json()
            .await?;

        let url = format!("{}/api/tokens", self.config.server.endpoint.as_str());
        let token: IssueTokenResponse = client
            .post(url)
            .header("Authorization", self.basic_auth())
            .json(&IssueTokenRequest {
                client_id: self.config.node.unique_id.clone(),
            })
            .send()
            .await?
            .json()
            .await?;

        self.client_token = token.id;
        Ok(())
    }

    async fn get_configs(&self) -> Result<Vec<PipeConfig>, SectionError> {
        let client = reqwest::Client::new();
        let url = format!("{}/api/pipe/configs", self.config.server.endpoint.as_str());
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
        format!(
            "Basic {}",
            BASE64.encode(format!("{}:", self.config.server.token))
        )
    }

    fn client_auth(&self) -> String {
        format!("Bearer {}", self.client_token)
    }

    // spawns client
    pub fn spawn(mut self) -> JoinHandle<Result<(), SectionError>> {
        tokio::spawn(async move { self.enter_loop().await })
    }

    async fn enter_loop(&mut self) -> Result<(), SectionError> {
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
                    continue;
                }
            };

            log::debug!("pipe configs: {:#?}", pipe_configs);
            let mut ids: HashSet<u64> =
                HashSet::from_iter(self.scheduler_handle.list_ids().await?.into_iter());
            for pipe_config in pipe_configs.into_iter() {
                let id = pipe_config.id;
                let config: Config = match pipe_config.try_into() {
                    Ok(c) => c,
                    Err(e) => {
                        log::error!("bad pipe config: {:?}", e);
                        continue;
                    }
                };
                if is_for_client(&config, &self.config.node.unique_id) {
                    if let Err(e) = self.scheduler_handle.add_pipe(id, config).await {
                        log::error!("failed to schedule pipe: {:?}", e);
                    }
                    ids.remove(&id);
                }
            }
            for id in ids.into_iter() {
                self.scheduler_handle.remove_pipe(id).await?;
            }

            tokio::time::sleep(Duration::from_secs(5)).await
        }
    }
}

pub fn new(
    config: ClientConfig,
    scheduler_handle: SchedulerHandle,
) -> JoinHandle<Result<(), SectionError>> {
    Client::new(config, scheduler_handle).spawn()
}

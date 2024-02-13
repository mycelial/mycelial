//! http client
//!
//! Poll mycelial server configuration endpoint

use std::{collections::HashSet, time::Duration};

use crate::daemon_storage::Storage;
use base64::engine::{general_purpose::STANDARD as BASE64, Engine};
use common::{
    ClientConfig, PipeConfig, PipeConfigs, ProvisionClientRequest, ProvisionClientResponse,
};
use pipe::{
    config::{Config, Value},
    scheduler::SchedulerHandle,
};
use section::SectionError;
use tokio::task::JoinHandle;

/// Http Client
struct Client {
    config: ClientConfig,

    /// SchedulerHandle
    scheduler_handle: SchedulerHandle,

    storage_handle: Storage,
}

fn is_for_client(config: &Config, name: &str) -> bool {
    config.get_sections().iter().any(
        |section| matches!(section.get("client"), Some(Value::String(client)) if client == name),
    )
}

impl Client {
    fn new(
        config: ClientConfig,
        scheduler_handle: SchedulerHandle,
        storage_handle: Storage,
    ) -> Self {
        Self {
            config,
            scheduler_handle,
            storage_handle,
        }
    }

    // Client should register only once. Subsequently, it should use the client_id and client_secret it gets back from the registration to authenticate itself.
    async fn register_if_not_registered(&mut self) -> Result<(), SectionError> {
        let state = self.storage_handle.retrieve_client_creds().await?;
        match state {
            Some((_client_id, _client_secret)) => {}
            None => {
                return self.register().await;
            }
        }
        Ok(())
    }

    // Client should register only once. Subsequently, it should use the client_id and client_secret it gets back from the registration to authenticate itself.
    async fn register(&mut self) -> Result<(), SectionError> {
        let client = reqwest::Client::new();
        let url = format!("{}/api/client", self.config.server.endpoint.as_str());
        let resp = client
            .post(url)
            .header("Authorization", self.basic_auth())
            .json(&ProvisionClientRequest {
                client_config: self.config.clone(),
            })
            .send()
            .await?;
        if resp.status() != 200 {
            return Err(format!(
                "failed to register client - status code: {:?}",
                resp.status()
            )
            .into());
        }
        let provision_client_resp: ProvisionClientResponse = resp.json().await?;

        self.storage_handle
            .store_client_creds(
                provision_client_resp.client_id.clone(),
                provision_client_resp.client_secret.clone(),
            )
            .await?;
        Ok(())
    }

    async fn get_configs(&mut self) -> Result<Vec<PipeConfig>, SectionError> {
        let client = reqwest::Client::new();
        let url = format!("{}/api/pipe", self.config.server.endpoint.as_str());
        let configs: PipeConfigs = client
            .get(url)
            .header("Authorization", self.daemon_auth().await?)
            .send()
            .await?
            .json()
            .await?;
        Ok(configs.configs)
    }

    async fn daemon_auth(&mut self) -> Result<String, SectionError> {
        let state = self.storage_handle.retrieve_client_creds().await?;
        if let Some((client_id, client_secret)) = state {
            return Ok(format!(
                "Basic {}",
                BASE64.encode(format!("{}:{}", client_id, client_secret))
            ));
        }
        Err(SectionError::from("no state found"))
    }

    fn basic_auth(&self) -> String {
        format!(
            "Basic {}",
            BASE64.encode(format!("{}:", self.config.node.auth_token))
        )
    }

    // spawns client
    pub fn spawn(mut self) -> JoinHandle<Result<(), SectionError>> {
        tokio::spawn(async move { self.enter_loop().await })
    }

    async fn enter_loop(&mut self) -> Result<(), SectionError> {
        while let Err(e) = self.register_if_not_registered().await {
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
    storage_handle: Storage,
) -> JoinHandle<Result<(), SectionError>> {
    Client::new(config, scheduler_handle, storage_handle).spawn()
}

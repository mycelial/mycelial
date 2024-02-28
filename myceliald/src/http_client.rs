//! http client
//!
//! Poll mycelial server configuration endpoint
#![allow(unused)]
use std::{collections::HashSet, time::Duration};

use crate::daemon_storage::{Credentials, ServerInfo};
use base64::engine::{general_purpose::STANDARD as BASE64, Engine};
// FIXME: common crate
use crate::DaemonMessage;
use common::{
    ClientConfig, Destination, PipeConfig, PipeConfigs, ProvisionDaemonRequest,
    ProvisionDaemonResponse, Source,
};
use pipe::{
    config::{Config, Value},
    scheduler::SchedulerHandle,
};
use reqwest::StatusCode;
use section::SectionError;
use tokio::sync::{
    mpsc::{channel, Receiver, Sender, UnboundedSender},
    oneshot::{channel as oneshot_channel, Sender as OneshotSender},
};

struct HttpClient {
    endpoint: Option<String>,
    token: Option<String>,
    unique_id: Option<String>,
    display_name: Option<String>,
    client_id: Option<String>,
    client_secret: Option<String>,
    submit_request: Option<SubmitSectionRequest>,
    tx: UnboundedSender<DaemonMessage>,
}

#[derive(Debug)]
pub enum HttpClientEvent {
    Configs {
        configs: Vec<PipeConfig>,
    },
    Credentials {
        client_id: String,
        client_secret: String,
    },
    SectionsSubmitted {
        config_hash: String,
    },
}

#[derive(Debug)]
struct SubmitSectionRequest {
    config_hash: String,
    sources: Vec<Source>,
    destinations: Vec<Destination>,
}

fn is_for_client(config: &Config, name: &str) -> bool {
    config.get_sections().iter().any(
        |section| matches!(section.get("client"), Some(Value::String(client)) if client == name),
    )
}

impl HttpClient {
    fn new(tx: UnboundedSender<DaemonMessage>) -> Self {
        Self {
            endpoint: None,
            token: None,
            unique_id: None,
            display_name: None,
            client_id: None,
            client_secret: None,
            submit_request: None,
            tx,
        }
    }

    async fn register(&mut self) -> Result<ProvisionDaemonResponse, SectionError> {
        if self.endpoint.is_none() || self.token.is_none() {
            Err("endpoint/token not set")?
        }
        if self.unique_id.is_none() || self.display_name.is_none() {
            Err("unique_id/display_name not set")?
        }
        let client = reqwest::Client::new();
        let url = format!("{}/api/daemon/provision", self.endpoint.as_ref().unwrap());
        let resp = client
            .post(url)
            .header(
                "Authorization",
                self.basic_auth(self.token.as_ref().unwrap(), ""),
            )
            .json(&ProvisionDaemonRequest {
                unique_id: self.unique_id.as_ref().unwrap().into(),
                display_name: self.display_name.as_ref().unwrap().into(),
            })
            .send()
            .await?;

        if resp.status() != 200 {
            return Err(format!(
                "status code {:?}, response: {:?}",
                resp.status(),
                resp.text().await?
            ))?;
        }
        Ok(resp.json::<ProvisionDaemonResponse>().await?)
    }

    async fn get_configs(&mut self) -> Result<PipeConfigs, SectionError> {
        let client = reqwest::Client::new();
        let url = format!("{}/api/pipe", self.endpoint.as_ref().unwrap());
        let auth = self.basic_auth(
            self.client_id.as_ref().unwrap(),
            self.client_secret.as_ref().unwrap(),
        );
        let response = client.get(url).header("Authorization", auth).send().await?;

        match response.status() {
            StatusCode::OK => Ok(response.json::<PipeConfigs>().await?),
            status => Err(format!(
                "failed to fetch pipe configs, status_code: {status}, response: {}",
                response.text().await?
            ))?,
        }
    }

    fn basic_auth(&self, user: &str, pass: &str) -> String {
        format!("Basic {}", BASE64.encode(format!("{user}:{pass}")))
    }

    async fn poll_configs(&mut self) {
        match self.get_configs().await {
            Ok(PipeConfigs { configs }) => {
                self.tx
                    .send(DaemonMessage::HttpClient(HttpClientEvent::Configs {
                        configs,
                    }))
                    .ok();
            }
            Err(e) => {
                tracing::error!("failed to get configs: {e}")
            }
        }
    }

    async fn maybe_register(&mut self) -> bool {
        if self.client_id.is_some() && self.client_secret.is_some() {
            return true;
        }
        match self.register().await {
            Ok(ProvisionDaemonResponse {
                client_id,
                client_secret,
                ..
            }) => {
                self.client_id = Some(client_id.clone());
                self.client_secret = Some(client_secret.clone());
                self.tx
                    .send(DaemonMessage::HttpClient(HttpClientEvent::Credentials {
                        client_id,
                        client_secret,
                    }))
                    .ok();
                true
            }
            Err(e) => {
                tracing::error!("failed to register: {e}");
                false
            }
        }
    }

    async fn maybe_submit_sections(&mut self) {
        if self.submit_request.is_none() {
            return;
        }
        let submit_request = self.submit_request.as_ref().unwrap();
        let client = reqwest::Client::new();
        let url = format!(
            "{}/api/daemon/submit_sections",
            self.endpoint.as_ref().unwrap()
        );
        let auth = self.basic_auth(
            self.client_id.as_ref().unwrap(),
            self.client_secret.as_ref().unwrap(),
        );
        let response = client
            .post(url)
            .header("Authorization", auth)
            .json(&serde_json::json!({
                "unique_id": self.unique_id.as_ref().unwrap(),
                "sources": submit_request.sources.as_slice(),
                "destinations": submit_request.destinations.as_slice(),
            }))
            .send()
            .await;
        match response {
            Ok(response) if response.status() == StatusCode::OK => {
                let req = self.submit_request.take().unwrap();
                self.tx
                    .send(DaemonMessage::HttpClient(
                        HttpClientEvent::SectionsSubmitted {
                            config_hash: req.config_hash,
                        },
                    ))
                    .ok();
                tracing::info!("sections where submitted");
            }
            Ok(response) => {
                let status = response.status();
                let text = response.text().await;
                let text = text.as_ref().map(|x| x.as_str()).unwrap_or("");
                tracing::error!(
                    "failed to submit sections, status code: {status}, response: {text}"
                );
            }
            Err(e) => {
                tracing::error!("failed to submit sections: {e}");
            }
        }
    }

    // spawns client
    pub fn spawn(mut self) -> HttpClientHandle {
        let (tx, mut rx) = channel(1);
        tokio::spawn(async move { self.enter_loop(&mut rx).await });
        HttpClientHandle { tx }
    }

    async fn enter_loop(
        &mut self,
        rx: &mut Receiver<HttpClientMessage>,
    ) -> Result<(), SectionError> {
        let mut interval = tokio::time::interval(Duration::from_secs(5));
        loop {
            tokio::select! {
                msg = rx.recv() => {
                    let message = match msg {
                        None => {
                            tracing::info!("handle was dropped, shutting down");
                            return Ok(())
                        },
                        Some(message) => message,
                    };
                    match message {
                        HttpClientMessage::SetConnection{
                            endpoint, token, unique_id, display_name, client_id, client_secret, reply_to
                        } => {
                            tracing::info!("setting connection info");
                            self.endpoint = Some(endpoint);
                            self.token = Some(token);
                            self.unique_id = Some(unique_id);
                            self.display_name = Some(display_name);
                            self.client_id = client_id;
                            self.client_secret = client_secret;
                            reply_to.send(()).ok();
                        },
                        HttpClientMessage::SubmitSections { submit_request, reply_to } => {
                            tracing::info!("submit section request registered");
                            self.submit_request = Some(submit_request);
                            reply_to.send(()).ok();
                        },
                        HttpClientMessage::Shutdown{ reply_to } => {
                            tracing::info!("shutting down");
                            return Ok(())
                        }
                    };
                }
                _tick = interval.tick() => {
                    if !self.maybe_register().await {
                        continue
                    }
                    self.maybe_submit_sections().await;
                    self.poll_configs().await;
                }
            }
        }
    }
}

#[derive(Debug)]
enum HttpClientMessage {
    SetConnection {
        endpoint: String,
        token: String,
        unique_id: String,
        display_name: String,
        client_id: Option<String>,
        client_secret: Option<String>,
        reply_to: OneshotSender<()>,
    },
    SubmitSections {
        submit_request: SubmitSectionRequest,
        reply_to: OneshotSender<()>,
    },
    Shutdown {
        reply_to: OneshotSender<()>,
    },
}

#[derive(Debug)]
pub struct HttpClientHandle {
    tx: Sender<HttpClientMessage>,
}

impl HttpClientHandle {
    pub async fn set_connection(
        &self,
        endpoint: &str,
        token: &str,
        unique_id: &str,
        display_name: &str,
        client_id: Option<&str>,
        client_secret: Option<&str>,
    ) -> anyhow::Result<()> {
        let (reply_to, rx) = oneshot_channel();
        let message = HttpClientMessage::SetConnection {
            endpoint: endpoint.into(),
            token: token.into(),
            unique_id: unique_id.into(),
            display_name: display_name.into(),
            client_id: client_id.map(Into::into),
            client_secret: client_secret.map(Into::into),
            reply_to,
        };
        self.tx.send(message).await?;
        Ok(rx.await?)
    }

    pub async fn submit_sections(
        &self,
        config_hash: String,
        sources: Vec<Source>,
        destinations: Vec<Destination>,
    ) -> anyhow::Result<()> {
        let (reply_to, rx) = oneshot_channel();
        let submit_request = SubmitSectionRequest {
            config_hash,
            sources,
            destinations,
        };
        let message = HttpClientMessage::SubmitSections {
            submit_request,
            reply_to,
        };
        self.tx.send(message).await?;
        Ok(())
    }

    pub async fn shutdown(&self) -> anyhow::Result<()> {
        let (reply_to, rx) = oneshot_channel();
        let message = HttpClientMessage::Shutdown { reply_to };
        self.tx.send(message).await?;
        Ok(rx.await?)
    }
}

pub fn new(tx: UnboundedSender<crate::DaemonMessage>) -> HttpClientHandle {
    HttpClient::new(tx).spawn()
}

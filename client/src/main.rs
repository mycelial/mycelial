//! # Client
//! - communicates with server, grabs and persists pipe configs:
//!     - dumb server endpoint polling
//!     - server dumbly returns all existing pipes
//! - schedules and runs pipes
use std::{time::Duration, collections::HashSet};

use base64::engine::{general_purpose::STANDARD as BASE64, Engine};
use clap::Parser;
use exp2::dynamic_pipe::{
    config::Config,
    registry::{Constructor, Registry},
    scheduler::{Scheduler, ScheduleResult},
    section,
    section_impls::{mycelial_net, sqlite},
};
use serde::Deserialize;
use tokio::time::sleep;

#[derive(Parser)]
struct CLI {
    /// Server endpoint
    #[clap(
        short,
        long,
        env = "ENDPOINT",
        default_value = "http://localhost:8080/pipe/configs"
    )]
    endpoint: String,

    /// Server authorization token
    #[clap(short, long, env = "ENDPOINT_TOKEN")]
    token: String,
}

/// Setup & populate registry
fn setup_registry() -> Registry {
    let arr: &[(&str, Constructor)] = &[
        ("sqlite", sqlite::constructor),
        ("mycelial_net", mycelial_net::constructor),
    ];
    arr.iter()
        .fold(Registry::new(), |mut acc, &(section_name, constructor)| {
            acc.register_section(section_name, constructor);
            acc
        })
}

#[derive(Debug)]
struct Client {
    endpoint: String,
    token: String,
}

#[derive(Debug, Deserialize)]
pub struct RawConfig {
    id: u64,
    raw_config: String,
}

#[derive(Debug, Deserialize)]
pub struct Configs {
    configs: Vec<RawConfig>,
}

impl Client {
    fn new(endpoint: impl Into<String>, token: impl Into<String>) -> Self {
        Self {
            endpoint: endpoint.into(),
            token: token.into(),
        }
    }
    async fn get_configs(&self) -> Result<Vec<RawConfig>, section::Error> {
        let client = reqwest::Client::new();
        let url = format!("{}/pipe/configs", self.endpoint.as_str());
        let configs: Configs = client
            .get(url)
            .header("Authorization", self.basic_auth())
            .send()
            .await?
            .json()
            .await?;
        Ok(configs.configs)
    }

    fn basic_auth(&self) -> String {
        format!("Basic {}", BASE64.encode(format!("{}:", self.token)))
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = CLI::try_parse()?;
    let client = Client::new(cli.endpoint, cli.token);
    let scheduler = Scheduler::new(setup_registry()).spawn();

    loop {
        match client.get_configs().await {
            Ok(raw_configs) => {
                let res = raw_configs
                    .iter()
                    .map(
                        |raw_conf| match Config::try_from_json(&raw_conf.raw_config) {
                            Ok(c) => Ok((raw_conf.id, c)),
                            Err(e) => Err(e),
                        },
                    )
                    .collect::<Result<Vec<_>, _>>();
                if let Ok(configs) = res {
                    let mut ids: HashSet<u64> = HashSet::from_iter(scheduler.list_ids().await.unwrap().into_iter());
                    for (id, config) in configs {
                        match scheduler.add_pipe(id, config.clone()).await {
                            Err(e) => {
                                println!("failed to schedule pipe with id '{id}', error: {e:?}, config:\n{config:?}")
                            },
                            Ok(ScheduleResult::New) => {
                                println!("new pipe with id '{id}' scheduled with config:\n{config:?}")
                            },
                            Ok(ScheduleResult::Updated) => {
                                println!("pipe with id '{id}' re-scheduled, new config:\n{config:?}")
                            },
                            _ => (),
                        }
                        ids.remove(&id);
                    }
                    for id in ids.into_iter(){
                        scheduler.remove_pipe(id).await.unwrap()
                    };
                } else {
                    println!("raw_configs: {:?}", raw_configs);
                    println!("failed to parse configs: {:?}", res);
                }
            }
            Err(e) => {
                println!("failed to contact server: {:?}", e);
            }
        };
        sleep(Duration::from_secs(5)).await;
    }
}

//! # Client
//! - communicates with server, grabs and persists pipe configs:
//!     - dumb server endpoint polling
//!     - server dumbly returns all existing pipes
//! - schedules and runs pipes
use std::{time::Duration, collections::HashSet};

use base64::engine::{general_purpose::STANDARD as BASE64, Engine};
use clap::Parser;
use exp2::dynamic_pipe::{
    config::{Config, Value},
    registry::{Constructor, Registry},
    scheduler::Scheduler,
    section,
    section_impls::{mycelial_net, sqlite, kafka_source, snowflake_source, snowflake_destination},
};
use serde::Deserialize;
use serde_json::json;
use tokio::time::sleep;

#[derive(Parser)]
struct CLI {
    /// Server endpoint
    #[clap(
        short,
        long,
        env = "ENDPOINT",
        default_value = "http://localhost:8080/"
    )]
    endpoint: String,

    /// Server authorization token
    #[clap(short, long, env = "ENDPOINT_TOKEN")]
    token: String,

    /// Client name
    #[clap(short, long, env = "CLIENT_NAME", default_value = "test client")]
    name: String,
}

/// Setup & populate registry
fn setup_registry() -> Registry {
    let arr: &[(&str, Constructor)] = &[
        ("sqlite", sqlite::constructor),
        ("mycelial_net", mycelial_net::constructor),
        ("kafka_source", kafka_source::constructor),
        ("snowflake_source", snowflake_source::constructor),
        ("snowflake_destination", snowflake_destination::constructor),
    ];
    arr.iter()
        .fold(Registry::new(), |mut acc, &(section_name, constructor)| {
            acc.register_section(section_name, constructor);
            acc
        })
}

/// Http Client
#[derive(Debug)]
struct Client {
    /// Mycelial server endpoint
    endpoint: String,
    /// Basic Auth token
    token: String,
    /// Client token
    client_token: String,
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

impl TryInto<Config> for PipeConfig {
    type Error = section::Error;

    fn try_into(self) -> Result<Config, Self::Error> {
        let value: Value = self.pipe.try_into()?;
        Ok(Config::try_from(value)?)
    }
}


#[derive(Debug, Deserialize)]
struct ClientInfo {
    id: String,
}

#[derive(Debug, Deserialize)]
struct PipeConfigs {
    configs: Vec<PipeConfig>,
}

impl Client {
    fn new(endpoint: impl Into<String>, token: impl Into<String>) -> Self {
        Self {
            endpoint: endpoint.into(),
            token: token.into(),
            client_token: "".into(),
        }
    }

    async fn register(&mut self, client_id: impl AsRef<str>) -> Result<(), reqwest::Error> {
        let client_id = client_id.as_ref();
        let client = reqwest::Client::new();
        let url = format!("{}/api/client", self.endpoint.as_str());
        let _x: ClientInfo = client
            .post(url)
            .header("Authorization", self.basic_auth())
            .json(&json!({ "id": client_id }))
            .send()
            .await?.json().await?;

        let url = format!("{}/api/tokens", self.endpoint.as_str());
        let token: ClientInfo = client
            .post(url)
            .header("Authorization", self.basic_auth())
            .json(&json!({ "client_id": client_id }))
            .send()
            .await?
            .json()
            .await?;

        self.client_token = token.id;
        Ok(())
    }

    async fn get_configs(&self) -> Result<Vec<PipeConfig>, section::Error> {
        let client = reqwest::Client::new();
        let url = format!("{}/pipe/configs", self.endpoint.as_str());
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

}

fn is_for_client(config: &Config, name: &str) -> bool {
    config.get_sections().into_iter().filter_map(|section | {
        match section.get("client") {
            Some(value) => {
                match value {
                    Value::String(client) => Some(client.eq(name)),
                    _ => None,
                }
            }
            None => None,
        }
    }).collect().len() > 0
}

/// FIXME:
/// - prints & unwraps in error handling
/// - better structure - http client and scheduler mushed together in the same loop
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = CLI::try_parse()?;
    let mut client = Client::new(cli.endpoint, cli.token);
    while let Err(e) = client.register(&cli.name).await {
        println!("failed to register client: {:?}", e);
        tokio::time::sleep(Duration::from_secs(3)).await;
    }

    let scheduler = Scheduler::new(setup_registry()).spawn();

    loop {
        match client.get_configs().await {
            Ok(pipe_configs) => {
                let mut ids: HashSet<u64> = HashSet::from_iter(scheduler.list_ids().await.unwrap().into_iter());
                for pipe_config in pipe_configs.into_iter() {
                    let id = pipe_config.id;
                    let config: Config = match pipe_config.try_into() {
                        Ok(c) => c,
                        Err(e) => {
                            println!("bad pipe config: {:?}", e);
                            continue
                        }
                    };
                    if is_for_client(&config, &cli.name) {
                        if let Err(e) = scheduler.add_pipe(id, config).await {
                            println!("failed to schedule pipe: {:?}", e);
                        }
                    }
                    ids.remove(&id);
                }
                for id in ids.into_iter(){
                    scheduler.remove_pipe(id).await.unwrap()
                };
            }
            Err(e) => {
                println!("failed to contact server: {:?}", e);
            }
        };
        sleep(Duration::from_secs(5)).await;
    }
}

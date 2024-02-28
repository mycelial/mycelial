//! # Client
//! - communicates with server, grabs and persists pipe configs:
//!     - dumb server endpoint polling
//!     - server dumbly returns all existing pipes
//! - schedules and runs pipes
mod constructors;
mod daemon_storage;
mod http_client;
mod runtime;
mod storage;

use clap::Parser;
use common::ClientConfig;
use std::fs::File;
use std::io::Read;
use anyhow::{anyhow, Context, Result};

#[derive(Parser)]
struct Cli {
    /// Path to the TOML config file
    #[clap(short, long, env = "CONFIG_PATH")]
    config: String,
}

fn read_config(path: &str) -> Result<ClientConfig> {
    let mut config = String::new();
    let mut config_file = File::open(path)
        .context(format!("failed to open config file at '{path}'"))?;
    config_file.read_to_string(&mut config)?;
    Ok(toml::from_str(&config)?)
}

async fn run() -> Result<()> {
    let cli = Cli::try_parse()?;
    let config = read_config(&cli.config)?;

    let storage_handle = storage::new(config.node.storage_path.clone()).await?;
    let runtime_handle = runtime::new(storage_handle.clone());
    let daemon_storage = daemon_storage::new(config.node.storage_path.clone()).await?;
    let client_handle = http_client::new(config, runtime_handle, daemon_storage);
    client_handle.await?.map_err(|e| anyhow!(e))?;
    Ok(())
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    if let Err(e) = run().await {
        tracing::error!("{}", e);
    }
}

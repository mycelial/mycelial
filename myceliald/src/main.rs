//! # Client
//! - communicates with server, grabs and persists pipe configs:
//!     - dumb server endpoint polling
//!     - server dumbly returns all existing pipes
//! - schedules and runs pipes
mod constructors;
mod http_client;
mod runtime;
mod storage;

use clap::Parser;
use common::ClientConfig;
use section::SectionError;
use std::fs::File;
use std::io::Read;
use std::{io, result};

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error(transparent)]
    Section(#[from] SectionError),

    #[error(transparent)]
    Io(#[from] io::Error),

    #[error(transparent)]
    Toml(#[from] toml::de::Error),

    #[error(transparent)]
    Clap(#[from] clap::Error),

    #[error(transparent)]
    TokioTask(#[from] tokio::task::JoinError),
}

pub type Result<T> = result::Result<T, Error>;

#[derive(Parser)]
struct Cli {
    /// Path to the TOML config file
    #[clap(short, long, env = "CONFIG_PATH")]
    config: String,
}

fn read_config(path: &str) -> Result<ClientConfig> {
    let mut config = String::default();
    let mut config_file = File::open(path)?;
    config_file.read_to_string(&mut config)?;

    Ok(toml::from_str(&config)?)
}

async fn run() -> Result<()> {
    let cli = Cli::try_parse()?;
    let config = read_config(&cli.config)?;

    let storage_handle = storage::new(config.node.storage_path.clone()).await?;
    let runtime_handle = runtime::new(storage_handle);
    let client_handle = http_client::new(config, runtime_handle);
    client_handle.await??;
    Ok(())
}

#[tokio::main]
async fn main() {
    env_logger::init();
    if let Err(e) = run().await {
        eprintln!("{}", e);
    }
}

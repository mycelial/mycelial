//! # Client
//! - communicates with server, grabs and persists pipe configs:
//!     - dumb server endpoint polling
//!     - server dumbly returns all existing pipes
//! - schedules and runs pipes
mod http_client;
mod runtime;
mod storage;

use std::fs::File;
use std::{io, result};
use std::io::Read;
use clap::Parser;
use exp2::dynamic_pipe::section;
use myc_config::Config;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error(transparent)]
    Section(#[from] section::Error),

    #[error(transparent)]
    Io(#[from] io::Error),

    #[error(transparent)]
    Toml(#[from] toml::de::Error),

    #[error(transparent)]
    Clap(#[from] clap::Error),

    #[error(transparent)]
    TokioTask(#[from] tokio::task::JoinError)
}

pub type Result<T> = result::Result<T, Error>;

#[derive(Parser)]
struct Cli {
    /// Path to the TOML config file
    #[clap(short, long, env = "CONFIG_PATH")]
    config: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    pretty_env_logger::init();
    let cli = Cli::try_parse()?;
    let config = read_config(&cli.config)?;

    let storage_handle = storage::new(config.node.storage_path.clone()).await?;
    let runtime_handle = runtime::new(storage_handle);
    let client_handle = http_client::new(
        config,
        runtime_handle);
    client_handle.await??;

    Ok(())
}

fn read_config(path: &str) -> Result<Config> {
    let mut config = String::default();
    let mut config_file = File::open(path)?;
    config_file.read_to_string(&mut config)?;

    Ok(toml::from_str(&config)?)
}
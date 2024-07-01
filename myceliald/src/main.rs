use std::{env::current_dir, path::PathBuf};

use anyhow::Result;
use clap::{error::ErrorKind, Parser};
use myceliald::Daemon;
use tracing_subscriber::{prelude::*, EnvFilter};

#[derive(Debug, Parser)]
#[command(name = "myceliald")]
#[command(version)]
struct Cli {
    /// Path to the TOML config file
    #[arg(short, long, env = "CONFIG_PATH")]
    config: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer().with_ansi(false))
        .with(EnvFilter::from_default_env())
        .init();
    let cli = match Cli::try_parse() {
        Err(e) if e.kind() == ErrorKind::DisplayVersion => {
            print!("{}", e);
            return Ok(());
        }
        Err(e) => Err(e)?,
        Ok(cli) => cli,
    };
    let mut config_path = PathBuf::from(cli.config);
    if !config_path.is_absolute() {
        config_path = current_dir()?.join(config_path)
    }
    Daemon::start(config_path).await?;
    Ok(())
}

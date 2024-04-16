use std::{env::current_dir, path::PathBuf};

use anyhow::Result;
use clap::Parser;
use myceliald::Daemon;

#[derive(Parser)]
struct Cli {
    /// Path to the TOML config file
    #[clap(short, long, env = "CONFIG_PATH")]
    config: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    let cli = Cli::try_parse()?;
    let mut config_path = PathBuf::from(cli.config);
    if !config_path.is_absolute() {
        config_path = current_dir()?.join(config_path)
    }
    Daemon::start(config_path).await?;
    Ok(())
}

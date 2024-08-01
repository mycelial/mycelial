
use anyhow::Result;
use clap::{error::ErrorKind, Parser};
use myceliald::Daemon;
use tracing_subscriber::{prelude::*, EnvFilter};

#[derive(Debug, Parser)]
#[command(version)]
struct Cli {
    #[clap(env="DATABASE_PATH", default_value="myceliald.db")]
    database_path: String
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
    Daemon::start(&cli.database_path.as_str()).await?;
    Ok(())
}

use anyhow::Result;
use clap::{error::ErrorKind, Parser, Subcommand};
use tracing::level_filters::LevelFilter;
use tracing_subscriber::{prelude::*, EnvFilter};

#[derive(Debug, Parser)]
#[command(version)]
struct Cli {
    #[clap(env = "DATABASE_PATH", default_value = "myceliald.db")]
    database_path: String,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    Join {
        #[clap(long, env = "MYCELIAL_CONTROL_PLANE_URL")]
        control_plane_url: String,
        #[clap(long, env = "MYCELIAL_CONTROL_PLANCE_TLS_URL")]
        control_plane_tls_url: String,
        #[clap(long, env = "MYCELIAL_JOIN_TOKEN")]
        join_token: String,
    },
    Reset,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer().with_ansi(false))
        .with(
            EnvFilter::builder()
                .with_default_directive(LevelFilter::INFO.into())
                .from_env_lossy(),
        )
        .init();
    let cli = match Cli::try_parse() {
        Err(e) if e.kind() == ErrorKind::DisplayVersion => {
            print!("{}", e);
            return Ok(());
        }
        Err(e) => Err(e)?,
        Ok(cli) => cli,
    };
    let mut runtime = myceliald::new(&cli.database_path).await?;
    match cli.command {
        Some(Commands::Join {
            control_plane_url,
            control_plane_tls_url,
            join_token,
        }) => {
            runtime
                .join(&control_plane_url, &control_plane_tls_url, &join_token)
                .await?;
            tracing::info!("join successful");
        }
        Some(Commands::Reset) => {
            runtime.reset().await?;
            tracing::info!("runtime state reset");
        }
        None => {
            runtime.run().await?;
        }
    };
    runtime.shutdown().await?;
    Ok(())
}

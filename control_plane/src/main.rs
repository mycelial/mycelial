use anyhow::Result;
use clap::Parser;

#[derive(Debug, Parser)]
pub struct Cli {
    #[clap(short, long, default_value = "localhost:7777")]
    listen_addr: String
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::try_parse()?;
    tracing_subscriber::fmt().with_ansi(false).init();
    control_plane::run(cli.listen_addr.as_str()).await
}
use clap::Parser;
use control_plane::Result;

#[derive(Debug, Parser)]
pub struct Cli {
    #[clap(
        long,
        env = "MYCELIAL_HTTP_API_LISTEN_ADDR",
        default_value = "0.0.0.0:8000"
    )]
    http_api_listen_addr: String,

    #[clap(
        long,
        env = "MYCELIAL_DAEMON_API_LISTEN_ADDR",
        default_value = "0.0.0.0:8010"
    )]
    daemon_api_listen_addr: String,

    #[clap(
        long,
        env = "MYCELIAL_DATABASE_URL",
        default_value = "sqlite://control_plane.db"
    )]
    database_url: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::try_parse()?;
    tracing_subscriber::fmt().with_ansi(false).init();
    control_plane::run(
        cli.http_api_listen_addr.as_str(),
        cli.daemon_api_listen_addr.as_str(),
        cli.database_url.as_str(),
    )
    .await
}

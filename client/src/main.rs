//! # Client
//! Client is on-edge device daemon with experementation around pipe included:
//! - pipeline is statically defined
//!   * sqlite -> arrow -> mycelial_server
//! - transforms collected data into arrow recordbatch
//! - sends data serilized via arrow-ipc to mycelial server endpoint

use std::time::Duration;
use clap::Parser;
use exp2::{sqlite::Sqlite, channel::channel, arrow::ToArrow, Section, stub::Stub, net::mycelial::Mycelial, black_hole::BlackHole};

#[derive(Parser)]
struct CLI {
    /// Server endpoint
    #[clap(short, long, env = "ENDPOINT", default_value = "http://localhost:8080/ingestion")]
    endpoint: String,

    /// Server authorization token
    #[clap(short, long, env = "ENDPOINT_TOKEN")]
    token: String,

    /// Sqlite path
    #[clap(short, long, env = "SQLITE_PATH")]
    sqlite_path: String,

}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = CLI::try_parse()?;

    let sqlite = Sqlite::new(cli.sqlite_path, "SELECT * FROM test", Duration::from_secs(5));
    let (from_sqlite, to_arrow) = channel(1);
    let sqlite_handle = tokio::spawn(async { sqlite.start(Stub::<()>::new(), from_sqlite).await });

    let arrow_transform = ToArrow::new();
    let (from_arrow, to_mycelial_net) = channel(1);
    let arrow_handle = tokio::spawn(async { arrow_transform.start(to_arrow, from_arrow).await });

    let mycelial_net = Mycelial::new(cli.endpoint, cli.token);
    let mycelial_net_handle = tokio::spawn(async { mycelial_net.start(to_mycelial_net, BlackHole::new())}.await );

    sqlite_handle.await.ok();
    arrow_handle.await.ok();
    mycelial_net_handle.await.ok();

    Ok(())

}

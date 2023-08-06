//! # Client
//! - communicates with server, grabs and persists pipe configs:
//!     - dumb server endpoint polling
//!     - server dumbly returns all existing pipes
//! - schedules and runs pipes
mod http_client;
mod runtime; mod storage;
mod macros;

use clap::Parser;
use exp2::dynamic_pipe::section;

#[derive(Parser)]
struct CLI {
    /// Server endpoint
    #[clap(
        short,
        long,
        env = "ENDPOINT",
        default_value = "http://localhost:8080/"
    )]
    endpoint: String,

    /// Server authorization token
    #[clap(short, long, env = "ENDPOINT_TOKEN")]
    token: String,

    /// Client name
    #[clap(short, long, env = "CLIENT_NAME", default_value = "test client")]
    name: String,

    /// Storage path (SQLite)
    #[clap(short, long, env = "STORAGE_PATH")]
    storage_path: String
}

#[tokio::main]
async fn main() -> Result<(), section::Error> {
    let cli = CLI::try_parse()?;
    let storage_handle = storage::new(cli.storage_path).await?;
    let runtime_handle = runtime::new(storage_handle);
    let client_handle = http_client::new(cli.name, cli.endpoint, cli.token, runtime_handle);
    Ok(client_handle.await??)
}

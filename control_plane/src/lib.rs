pub mod http;
//pub mod db_pool;

use anyhow::Result;
use tokio::net::TcpListener;

pub async fn run(listen_addr: &str) -> Result<()> {
    let router = http::new();
    let listener = TcpListener::bind(listen_addr).await?;
    tracing::info!("listening at {}", listener.local_addr()?);
    axum::serve(listener, router.into_make_service()).await?;
    Ok(())
}
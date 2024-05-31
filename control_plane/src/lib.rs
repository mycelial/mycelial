pub(crate) mod app;
pub(crate) mod http;

use anyhow::Result;
use tokio::net::TcpListener;
use std::sync::Arc;

pub async fn run(listen_addr: &str, connection_string: &str) -> Result<()> {
    let app = Arc::new(app::App::new(connection_string).await?);
    let listener = TcpListener::bind(listen_addr).await?;
    tracing::info!("listening at {}", listener.local_addr()?);
    axum::serve(listener, http::new(app).into_make_service()).await?;
    Ok(())
}

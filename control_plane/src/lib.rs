pub(crate) mod app;
pub(crate) mod http;
pub(crate) mod tls_server;

use std::{net::SocketAddr, sync::Arc};

pub use app::{AppError, Result};

use futures::FutureExt;
use tokio::net::TcpListener;

async fn run_http_api(http_api_addr: &str, app: app::AppState) -> Result<()> {
    let http_api_listener = TcpListener::bind(http_api_addr).await?;
    tracing::info!(
        "listening for http API calls at {}",
        http_api_listener.local_addr()?
    );
    axum::serve(http_api_listener, http::api::new(app).into_make_service()).await?;
    Ok(())
}

async fn run_daemon_api(daemon_api_addr: &str, app: app::AppState) -> Result<()> {
    let daemon_api_addr: SocketAddr = daemon_api_addr.parse()?;
    tracing::info!("listening for daemon API calls at {}", daemon_api_addr);
    tls_server::serve(
        daemon_api_addr,
        http::daemon_api::new(Arc::clone(&app)),
        app,
    )
    .await
    .map_err(|e| anyhow::anyhow!("{e}"))?;
    Ok(())
}

pub async fn run(http_api_addr: &str, daemon_api_addr: &str, database_url: &str) -> Result<()> {
    let app = Arc::new(app::AppBuilder::new(database_url).await?.build().await?);
    app.migrate().await?;

    futures::select! {
        res = run_http_api(http_api_addr, Arc::clone(&app)).fuse() => {
            tracing::error!("http api exited");
            res
        },
        res = run_daemon_api(daemon_api_addr, app).fuse() => {
            tracing::error!("daemon api exited");
            res
        }
    }
}

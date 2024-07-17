pub(crate) mod app;
pub(crate) mod http;

pub use app::{AppError, Result};

use tokio::net::TcpListener;

pub async fn run(listen_addr: &str, database_url: &str) -> Result<()> {
    let app = app::App::new(database_url).await?;
    app.init().await?;
    let listener = TcpListener::bind(listen_addr).await?;
    tracing::info!("listening at {}", listener.local_addr()?);
    axum::serve(listener, http::new(app).into_make_service()).await?;
    Ok(())
}

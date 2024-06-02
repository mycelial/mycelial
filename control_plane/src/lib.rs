pub(crate) mod app;
pub(crate) mod http;

use tokio::net::TcpListener;

pub async fn run(listen_addr: &str, connection_string: &str) -> anyhow::Result<()> {
    let app = app::App::new(connection_string).await?;
    app.init().await?;
    let listener = TcpListener::bind(listen_addr).await?;
    tracing::info!("listening at {}", listener.local_addr()?);
    axum::serve(listener, http::new(app).into_make_service()).await?;
    Ok(())
}

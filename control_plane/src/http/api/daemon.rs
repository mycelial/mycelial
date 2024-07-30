use axum::extract::{Path, State};
use axum::response::IntoResponse;
use axum::Json;

use crate::app::{App, DaemonJoinRequest, DaemonToken};
use crate::http::Result;

pub async fn create_token(State(app): State<App>) -> Result<Json<DaemonToken>> {
    let token = app.create_daemon_token().await?;
    Ok(Json(token))
}

pub async fn list_tokens(State(app): State<App>) -> Result<Json<Vec<DaemonToken>>> {
    let tokens = app.list_daemon_tokens().await?;
    Ok(Json(tokens))
}

pub async fn delete_token(State(app): State<App>, Path(id): Path<String>) -> Result<()> {
    let id: uuid::Uuid = id.parse()?;
    app.delete_daemon_token(id).await?;
    Ok(())
}

pub async fn join(
    State(app): State<App>,
    Json(join_request): Json<DaemonJoinRequest>,
) -> Result<impl IntoResponse> {
    tracing::debug!("got daemon join request");
    Ok("")
}

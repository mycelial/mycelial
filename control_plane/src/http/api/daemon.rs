use axum::extract::{Path, State};
use axum::Json;

use crate::app::{AppState, DaemonJoinRequest, DaemonJoinResponse, DaemonToken};
use crate::http::Result;

pub async fn create_token(State(app): State<AppState>) -> Result<Json<DaemonToken>> {
    let token = app.create_daemon_token().await?;
    Ok(Json(token))
}

pub async fn list_tokens(State(app): State<AppState>) -> Result<Json<Vec<DaemonToken>>> {
    let tokens = app.list_daemon_tokens().await?;
    Ok(Json(tokens))
}

pub async fn delete_token(State(app): State<AppState>, Path(id): Path<String>) -> Result<()> {
    let id: uuid::Uuid = id.parse()?;
    app.delete_daemon_token(id).await?;
    Ok(())
}

pub async fn join(
    State(app): State<AppState>,
    Json(join_request): Json<DaemonJoinRequest>,
) -> Result<Json<DaemonJoinResponse>> {
    let response = app.daemon_join(join_request).await?;
    Ok(Json(response))
}

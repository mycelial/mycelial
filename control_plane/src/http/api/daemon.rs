use axum::extract::{Path, State};
use axum::{http::StatusCode, Json};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::app::{
    AppErrorKind, AppState, Daemon, DaemonJoinRequest, DaemonJoinResponse, DaemonToken,
};
use crate::http::Result;
use crate::AppError;

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
) -> Result<Json<DaemonJoinResponse>, (StatusCode, Json<Value>)> {
    match app.daemon_join(join_request).await {
        Ok(response) => Ok(Json(response)),
        Err(AppError {
            kind: AppErrorKind::TokenUsed,
            ..
        }) => Err((
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "token used"})),
        )),
        Err(AppError {
            kind: AppErrorKind::JoinRequestHashMissmatch,
            ..
        }) => Err((
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "hash missmatch"})),
        )),
        Err(_) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "internal error"})),
        )),
    }
}

pub async fn list_daemons(State(app): State<AppState>) -> Result<Json<Vec<Daemon>>> {
    app.list_daemons().await.map(Json)
}

#[derive(Deserialize)]
pub struct SetName {
    name: String,
}

pub async fn set_name(
    State(app): State<AppState>,
    Path(id): Path<String>,
    name: Json<SetName>,
) -> Result<()> {
    app.set_daemon_name(id.parse()?, &name.name).await?;
    Ok(())
}

pub async fn unset_name(State(app): State<AppState>, Path(id): Path<String>) -> Result<()> {
    app.unset_daemon_name(id.parse()?).await?;
    Ok(())
}

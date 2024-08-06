//! Workspaces routes

use crate::{
    app::{AppState, Workspace},
    http::Result,
};
use axum::{
    extract::{Path, State},
    response::IntoResponse,
    Json,
};

pub async fn create(
    app: State<AppState>,
    Json(workspace): Json<Workspace>,
) -> Result<impl IntoResponse> {
    app.create_workspace(&workspace).await?;
    Ok(Json("ok"))
}

pub async fn read(app: State<AppState>) -> Result<Json<Vec<Workspace>>> {
    Ok(Json(app.read_workspaces().await?))
}

pub async fn delete(app: State<AppState>, Path(name): Path<String>) -> Result<impl IntoResponse> {
    app.delete_workspace(&name).await?;
    Ok(Json("ok"))
}

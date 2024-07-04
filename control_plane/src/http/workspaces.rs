//! Workspaces routes

use crate::{
    app::{db::Workspace, App},
    http::Result,
};
use axum::{
    extract::{Path, State},
    response::IntoResponse,
    Json,
};

pub async fn create(
    app: State<App>,
    Json(workspace): Json<Workspace>,
) -> Result<impl IntoResponse> {
    app.create_workspace(&workspace).await?;
    Ok(Json("ok"))
}

pub async fn read(app: State<App>) -> Result<Json<Vec<Workspace>>> {
    Ok(Json(app.read_workspaces().await?))
}

pub async fn delete(app: State<App>, Path(name): Path<String>) -> Result<impl IntoResponse> {
    app.delete_workspace(&name).await?;
    Ok(Json("ok"))
}

//! Workspaces routes

use crate::http::Result;
use axum::{
    extract::{Path, State},
    response::IntoResponse,
    Json,
};

use crate::app::{model::Workspace, App};

pub async fn create_workspaces(
    app: State<App>,
    Json(workspace): Json<Workspace>,
) -> Result<impl IntoResponse> {
    app.create_workspace(&workspace).await?;
    Ok(Json("ok"))
}

pub async fn get_workspaces(app: State<App>) -> Result<Json<Vec<Workspace>>> {
    Ok(Json(app.get_workspaces().await?))
}

pub async fn delete_workspaces(
    app: State<App>,
    Path(name): Path<String>,
) -> Result<impl IntoResponse> {
    app.delete_workspace(&name).await?;
    Ok(Json("ok"))
}

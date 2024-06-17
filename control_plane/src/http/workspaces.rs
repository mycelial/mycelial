//! Workspaces routes

use crate::http::Result;
use axum::{
    extract::{Path, State},
    response::IntoResponse,
    routing::{delete, post},
    Json, Router,
};

use crate::app::{model::Workspace, App};

async fn create_workspaces(
    app: State<App>,
    Json(workspace): Json<Workspace>,
) -> Result<impl IntoResponse> {
    app.create_workspace(&workspace).await?;
    Ok(Json("ok"))
}

async fn read_workspaces(app: State<App>) -> Result<Json<Vec<Workspace>>> {
    Ok(Json(app.read_workspaces().await?))
}

async fn update_workspaces() -> impl IntoResponse {
    unimplemented!("update workspaces")
}

async fn delete_workspaces(app: State<App>, Path(name): Path<String>) -> Result<impl IntoResponse> {
    app.delete_workspace(&name).await?;
    Ok(Json("ok"))
}

pub fn new(app: crate::app::App) -> Router {
    Router::new()
        .route(
            "/api/workspaces",
            post(create_workspaces)
                .get(read_workspaces)
                .put(update_workspaces),
        )
        .route("/api/workspaces/:name", delete(delete_workspaces))
        .with_state(app)
}

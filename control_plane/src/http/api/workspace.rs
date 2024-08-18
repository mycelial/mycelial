use crate::{
    app::{AppState, WorkspaceGraph, WorkspaceUpdate},
    http::Result,
};
use axum::{
    extract::{Path, State},
    Json,
};

// Return workspace state:
// - nodes, edges
pub async fn read(
    State(app): State<AppState>,
    Path(workspace_name): Path<String>,
) -> Result<Json<WorkspaceGraph>> {
    app.get_workspace_graph(&workspace_name).await.map(Json)
}

pub async fn update(
    State(app): State<AppState>,
    Json(mut updates): Json<Vec<WorkspaceUpdate>>,
) -> Result<()> {
    app.update_workspace(updates.as_mut_slice()).await
}

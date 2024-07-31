use crate::{
    app::{App, Graph, WorkspaceUpdate},
    http::Result,
};
use axum::{
    extract::{Path, State},
    Json,
};

// Return workspace state:
// - nodes, edges
pub async fn read(
    State(app): State<App>,
    Path(workspace_name): Path<String>,
) -> Result<Json<Graph>> {
    app.get_graph(&workspace_name).await.map(Json)
}

pub async fn update(
    State(app): State<App>,
    Json(mut updates): Json<Vec<WorkspaceUpdate>>,
) -> Result<()> {
    app.update_workspace(updates.as_mut_slice()).await
}
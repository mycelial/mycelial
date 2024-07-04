use crate::{
    app::{db::Graph, App},
    http::Result,
};
use axum::{
    extract::{Path, State},
    Json,
};
use serde::Deserialize;
use uuid::Uuid;

// Return workspace state:
// - nodes, edges
pub async fn read(
    State(app): State<App>,
    Path(workspace_name): Path<String>,
) -> Result<Json<Graph>> {
    Ok(app.get_graph(&workspace_name).await.map(Json)?)
}

#[derive(Debug, Deserialize)]
pub struct WorkspaceUpdate {
    name: String,
    operations: Vec<WorkspaceOperation>,
}

#[derive(Debug, Deserialize)]
pub struct Config {
    node_type: String,
    fields: serde_json::Value,
}

#[derive(Debug, Deserialize)]
pub enum WorkspaceOperation {
    AddNode {
        id: Uuid,
        x: f64,
        y: f64,
        config: Config,
    },
    UpdateNode {},
    RemoveNode {},
    AddEdge {},
    RemoveEdge {},
}

pub async fn update(State(app): State<App>, ops: Json<Vec<WorkspaceUpdate>>) -> Result<()> {
    tracing::info!("ops: {:?}", ops);
    Ok(())
}

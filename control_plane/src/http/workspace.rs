use axum::{extract::{Path, State}, Json};
use crate::{app::{db::Graph, App}, http::Result};


// Return workspace state:
// - nodes, edges
pub async fn read(State(app): State<App>, Path(name): Path<String>) -> Result<Json<Graph>> {
    Ok(app.get_graph(&name).await.map(Json)?)
}

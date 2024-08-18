use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Graph {
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Node {
    pub id: uuid::Uuid,
    #[serde(skip_serializing_if="Option::is_none")]
    pub display_name: Option<String>,
    pub config: Box<dyn config::Config>,
    #[serde(skip_serializing_if="Option::is_none")]
    pub x: Option<f64>,
    #[serde(skip_serializing_if="Option::is_none")]
    pub y: Option<f64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Edge {
    pub from_id: uuid::Uuid,
    pub to_id: uuid::Uuid,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag="type")]
pub enum Message {
    GetGraph,
    GetGraphResponse {

    }
}

impl Message {
    pub fn get_graph() -> Self {
        Self::GetGraph
    }
}
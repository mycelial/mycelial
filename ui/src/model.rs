use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workspace {
    pub name: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NodeState {
    pub id: u64,
    pub x: f64,
    pub y: f64,
    #[serde(skip)]
    pub w: f64,
    #[serde(skip)]
    pub h: f64,
    #[serde(skip)]
    pub port_diameter: f64,
    pub node_type: String,
    pub config: Box<dyn config::Config>,
}

impl NodeState {
    pub fn new(
        id: u64,
        node_type: String,
        x: f64,
        y: f64,
        config: Box<dyn config::Config>,
    ) -> Self {
        Self {
            id,
            node_type,
            x,
            y,
            w: 0.0,
            h: 0.0,
            port_diameter: 12.0,
            config,
        }
    }

    pub fn input_pos(&self) -> (f64, f64) {
        let offset = self.port_diameter / 2.0;
        (self.x - offset, self.y + self.h / 2.0 - offset)
    }

    pub fn output_pos(&self) -> (f64, f64) {
        let offset = self.port_diameter / 2.0;
        (self.x - offset + self.w, self.y + self.h / 2.0 - offset)
    }
}

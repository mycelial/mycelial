use super::node_config::{Config, ConfigTrait};

pub type NodeType = &'static str;

// Node of the graph
//
// Each node has:
// 1. unique Id - UUID (for now)
// 2. set of coordinates
#[derive(Debug)]
pub struct NodeState {
    // graph id
    pub id: u64,
    pub node_type: NodeType,
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
    pub port_diameter: f64,
    pub config: Box<dyn ConfigTrait>,
}

impl NodeState {
    pub fn new(id: u64, node_type: NodeType, x: f64, y: f64) -> Self {
        Self {
            id,
            node_type,
            x,
            y,
            w: 0.0,
            h: 0.0,
            port_diameter: 12.0,
            config: Box::<Config>::default(),
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

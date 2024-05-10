use dioxus::prelude::*;
use std::collections::BTreeMap;

// Simple graph
#[derive(Debug)]
pub struct Graph {
    nodes: BTreeMap<u64, Signal<NodeState>>,
    edges: BTreeMap<u64, u64>,
    counter: u64,
}

impl Graph {
    pub fn new() -> Self {
        Self {
            nodes: BTreeMap::new(),
            edges: BTreeMap::new(),
            counter: 0,
        }
    }

    pub fn get_id(&mut self) -> u64 {
        let id = self.counter;
        self.counter += 1;
        id
    }

    pub fn add_node(&mut self, id: u64, node: Signal<NodeState>) {
        self.nodes.insert(id, node);
    }

    pub fn get_node(&self, id: u64) -> Option<Signal<NodeState>> {
        self.nodes.get(&id).copied()
    }

    pub fn remove_node(&mut self, id: u64) {
        self.nodes.remove(&id);
        self.remove_edge(id);
    }

    pub fn iter_nodes(&self) -> impl Iterator<Item = (u64, Signal<NodeState>)> + Clone + '_ {
        self.nodes.iter().map(|(id, node_state)| (*id, *node_state))
    }

    pub fn add_edge(&mut self, from_node: u64, to_node: u64) {
        self.edges.insert(from_node, to_node);
    }

    pub fn remove_edge(&mut self, from_node: u64) {
        self.edges.remove(&from_node);
    }

    pub fn iter_edges(&self) -> impl Iterator<Item = (u64, u64)> + Clone + '_ {
        self.edges.iter().map(|(key, value)| (*key, *value))
    }
}

// Node of the graph
//
// Each node has:
// 1. unique Id - UUID (for now)
// 2. set of coordinates
#[derive(Debug)]
pub struct NodeState {
    pub id: u64,
    pub node_type: &'static str,
    pub  x: f64,
    pub  y: f64,
    pub  w: f64,
    pub  h: f64,
    pub  port_diameter: f64,
}

impl NodeState {
    pub fn new(id: u64, node_type: &'static str, x: f64, y: f64) -> Self {
        Self {
            id,
            node_type,
            x,
            y,
            w: 0.0,
            h: 0.0,
            port_diameter: 12.0,
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
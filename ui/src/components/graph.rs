use std::collections::{BTreeMap, HashSet};

#[derive(Debug, Clone, Copy)]
pub enum GraphOperation<T> {
    AddNode(T),
    AddEdge(u64, u64),
    RemoveNode(T),
    RemoveEdge(u64, u64),
}

// Simple graph
#[derive(Debug)]
pub struct Graph<T: std::fmt::Debug> {
    nodes: BTreeMap<u64, T>,
    edges: BTreeMap<u64, u64>,
    counter: u64,
}

impl<T: std::fmt::Debug + Clone> Default for Graph<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: std::fmt::Debug + Clone> Graph<T> {
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

    pub fn add_node(&mut self, id: u64, node: T) -> GraphOperation<T> {
        self.nodes.insert(id, node.clone());
        GraphOperation::AddNode(node)
    }

    pub fn get_node(&self, id: u64) -> Option<&T> {
        self.nodes.get(&id)
    }

    pub fn remove_node(&mut self, id: u64) -> Vec<GraphOperation<T>> {
        let mut ops = vec![];
        if let Some(op) = self.nodes
            .remove(&id)
            .map(GraphOperation::RemoveNode) { ops.push(op) }
        if let Some(op) = self.remove_edge(id)
            .map(|(from_node, to_node)| GraphOperation::RemoveEdge(from_node, to_node)) { ops.push(op) }
        self.remove_edge_to(id)
            .map(|(from_node, to_node)| GraphOperation::RemoveEdge(from_node, to_node))
            .for_each(|op| ops.push(op));
        ops
    }

    pub fn iter_nodes(&self) -> impl Iterator<Item = (u64, &T)> + Clone + '_ {
        self.nodes.iter().map(|(id, node_state)| (*id, node_state))
    }

    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    // This function adds an edge from `from_node` to `to_node`.
    pub fn add_edge(&mut self, from_node: u64, to_node: u64) -> Vec<GraphOperation<T>> {
        let mut ops= vec![];
        if from_node == to_node {
            return ops;
        }
        let mut visited = HashSet::<u64>::from_iter([from_node, to_node]);
        let mut next = to_node;
        while let Some(node) = self.edges.get(&next).copied() {
            if !visited.insert(node) {
                return ops;
            };
            next = node;
        }
        if let Some(prev_node) = self.edges.insert(from_node, to_node) {
            ops.push(GraphOperation::RemoveEdge(from_node, prev_node));
        }
        ops.push(GraphOperation::AddEdge(from_node, to_node));
        ops
    }

    pub fn get_child_node(&self, from_node: u64) -> Option<&T> {
        match self.edges.get(&from_node).copied() {
            Some(to_node) => self.get_node(to_node),
            None => None,
        }
    }

    pub fn get_parent_nodes(&self, to_node: u64) -> impl Iterator<Item = &T> + Clone + '_ {
        self.iter_edges()
            .filter(move |(_, to)| *to == to_node)
            .map(|(from_node, _)| self.get_node(from_node).unwrap())
    }

    pub fn remove_edge(&mut self, from_node: u64) -> Option<(u64, u64)> {
        self.edges
            .remove(&from_node)
            .map(|to_node| (from_node, to_node))
    }

    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }

    fn remove_edge_to(&mut self, to_node: u64) -> impl Iterator<Item = (u64, u64)> + '_ {
        self.iter_edges()
            .fold(vec![], |mut acc, (from, to)| {
                if to == to_node {
                    acc.push(from)
                }
                acc
            })
            .into_iter()
            .filter_map(|from| self.remove_edge(from))
    }

    pub fn iter_edges(&self) -> impl Iterator<Item = (u64, u64)> + Clone + '_ {
        self.edges.iter().map(|(key, value)| (*key, *value))
    }
}

#[cfg(test)]
mod test {
    use std::collections::HashSet;

    use quickcheck::TestResult;

    use super::*;

    #[derive(Debug, Clone, Copy, PartialEq)]
    struct TestNode {
        id: u64,
    }

    #[test]
    fn test_graph_add_iter_remove_node() {
        let check = |input: Vec<u64>| -> TestResult {
            let mut graph = Graph::new();
            let unique_ids: HashSet<u64> = HashSet::from_iter(input.iter().copied());

            input.iter().for_each(|&id| {
                graph.add_node(id, TestNode { id });
            });
            let nodes = graph
                .iter_nodes()
                .map(|(id, &node)| {
                    assert_eq!(id, node.id);
                    (id, node)
                })
                .collect::<Vec<_>>();
            assert_eq!(unique_ids.len(), nodes.len());

            for id in input.iter().copied() {
                graph.remove_node(id);
            }
            assert_eq!(graph.node_count(), 0);
            TestResult::from_bool(true)
        };
        quickcheck::quickcheck(check as fn(_: Vec<u64>) -> TestResult)
    }

    #[test]
    fn test_graph_add_iter_remove_edges() {
        let check = |input: Vec<u64>| -> TestResult {
            let mut graph = Graph::new();
            input.iter().for_each(|&id| {
                graph.add_node(id, TestNode { id });
            });
            let unique_ids: HashSet<u64> = HashSet::from_iter(input.iter().copied());
            if unique_ids.len() < 2 {
                return TestResult::discard();
            }
            for window in unique_ids
                .iter()
                .copied()
                .collect::<Vec<_>>()
                .as_slice()
                .windows(2)
            {
                let (from_node, to_node) = (window[0], window[1]);
                graph.add_edge(from_node, to_node);
            }
            assert_eq!(graph.edge_count(), unique_ids.len() - 1);

            for &node in input.iter() {
                graph.remove_edge(node);
            }
            assert_eq!(
                graph.edge_count(),
                0,
                "expected edge count to be 0, got {}",
                graph.edge_count()
            );
            TestResult::from_bool(true)
        };
        quickcheck::quickcheck(check as fn(_: Vec<u64>) -> TestResult)
    }

    #[test]
    fn test_graph_edges_cleanup_on_node_removal() {
        let check = |input: Vec<u64>| -> TestResult {
            let mut graph = Graph::new();
            input.iter().for_each(|&id| {
                graph.add_node(id, TestNode { id });
            });
            let unique_ids: HashSet<u64> = HashSet::from_iter(input.iter().copied());
            if unique_ids.len() < 2 {
                return TestResult::discard();
            }
            for window in unique_ids
                .iter()
                .copied()
                .collect::<Vec<_>>()
                .as_slice()
                .windows(2)
            {
                let (from_node, to_node) = (window[0], window[1]);
                graph.add_edge(from_node, to_node);
            }
            assert_eq!(graph.edge_count(), unique_ids.len() - 1);

            for &node in input.iter() {
                let parents = graph.get_parent_nodes(node).copied().collect::<Vec<_>>();
                let child = graph.get_child_node(node).copied();
                graph.remove_node(node);
                for parent in parents {
                    assert!(graph.get_child_node(parent.id).is_none())
                }
                if let Some(TestNode { id }) = child {
                    assert_eq!(graph.get_parent_nodes(id).count(), 0);
                }
            }
            assert_eq!(
                graph.edge_count(),
                0,
                "expected edge count to be 0, got {}",
                graph.edge_count()
            );
            assert_eq!(
                graph.node_count(),
                0,
                "expected node count to be 0, got {}",
                graph.node_count()
            );

            TestResult::from_bool(true)
        };
        quickcheck::quickcheck(check as fn(_: Vec<u64>) -> TestResult)
    }

    #[test]
    fn test_graph_edge_loop() {
        let mut graph = Graph::new();
        graph.add_node(0, TestNode { id: 0 });
        graph.add_edge(0, 0);
        assert_eq!(graph.edge_count(), 0);
    }

    #[test]
    fn test_prop_no_loops() {
        let check = |nodes: Vec<u64>| -> TestResult {
            let nodes = Vec::from_iter(
                HashSet::<u64>::from_iter(nodes.iter().copied())
                    .iter()
                    .copied(),
            );
            if nodes.is_empty() {
                return TestResult::discard();
            }
            let mut graph = Graph::new();
            nodes.iter().for_each(|&id| {
                graph.add_node(id, TestNode { id });
            });
            nodes.as_slice().windows(2).for_each(|pair| {
                graph.add_edge(pair[0], pair[1]);
            });
            // try to add edge from last node to every other one
            let last_node = *nodes.last().unwrap();
            for node in nodes {
                graph.add_edge(last_node, node);
                assert!(graph.get_child_node(last_node).is_none())
            }
            TestResult::from_bool(true)
        };
        quickcheck::quickcheck(check as fn(Vec<u64>) -> TestResult)
    }
}

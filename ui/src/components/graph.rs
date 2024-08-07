use std::collections::{BTreeMap, HashSet};
use std::hash::Hash;

pub trait GraphKey: std::fmt::Debug + Copy + Ord + Hash {}
pub trait GraphValue: std::fmt::Debug + Clone {}

impl<T> GraphKey for T where T: std::fmt::Debug + Copy + Ord + Hash {}
impl<T> GraphValue for T where T: std::fmt::Debug + Clone {}

#[derive(Debug, Clone, Copy)]
pub enum GraphOperation<K: GraphKey, T: GraphValue> {
    AddNode(T),
    AddEdge(K, K),
    RemoveNode(T),
    RemoveEdge(K, K),
}

#[derive(Debug)]
pub struct Graph<K: GraphKey, V: GraphValue> {
    nodes: BTreeMap<K, V>,
    edges: BTreeMap<K, K>,
}

impl<K: GraphKey, V: GraphValue> Default for Graph<K, V> {
    fn default() -> Self {
        Self::new()
    }
}

impl<K: GraphKey, V: GraphValue> Graph<K, V> {
    pub fn new() -> Self {
        Self {
            nodes: BTreeMap::new(),
            edges: BTreeMap::new(),
        }
    }

    pub fn add_node(&mut self, id: K, node: V) -> GraphOperation<K, V> {
        self.nodes.insert(id, node.clone());
        GraphOperation::AddNode(node)
    }

    pub fn get_node(&self, id: K) -> Option<&V> {
        self.nodes.get(&id)
    }

    pub fn remove_node(&mut self, id: K) -> Vec<GraphOperation<K, V>> {
        let mut ops = vec![];
        if let Some(op) = self.nodes.remove(&id).map(GraphOperation::RemoveNode) {
            ops.push(op)
        }
        if let Some(op) = self.remove_edge(id) {
            ops.push(op)
        }
        self.remove_edge_to(id).for_each(|op| ops.push(op));
        ops
    }

    pub fn iter_nodes(&self) -> impl Iterator<Item = (K, &V)> + Clone + '_ {
        self.nodes.iter().map(|(id, node_state)| (*id, node_state))
    }

    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    // This function adds an edge from `from_node` to `to_node`.
    pub fn add_edge(&mut self, from_node: K, to_node: K) -> Vec<GraphOperation<K, V>> {
        let mut ops = vec![];
        // loops are not allowed
        if from_node == to_node {
            return ops;
        }
        // check both nodes exist
        if !(self.nodes.contains_key(&from_node) && self.nodes.contains_key(&to_node)) {
            return ops;
        };
        let mut visited = HashSet::<K>::from_iter([from_node, to_node]);
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

    pub fn get_child_node(&self, from_node: K) -> Option<&V> {
        match self.edges.get(&from_node).copied() {
            Some(to_node) => self.get_node(to_node),
            None => None,
        }
    }

    pub fn get_parent_nodes(&self, to_node: K) -> impl Iterator<Item = &V> + Clone + '_ {
        self.iter_edges()
            .filter(move |(_, to)| *to == to_node)
            .map(|(from_node, _)| self.get_node(from_node).unwrap())
    }

    pub fn remove_edge(&mut self, from_node: K) -> Option<GraphOperation<K, V>> {
        self.edges
            .remove(&from_node)
            .map(|to_node| GraphOperation::RemoveEdge(from_node, to_node))
    }

    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }

    fn remove_edge_to(&mut self, to_node: K) -> impl Iterator<Item = GraphOperation<K, V>> + '_ {
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

    pub fn iter_edges(&self) -> impl Iterator<Item = (K, K)> + Clone + '_ {
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

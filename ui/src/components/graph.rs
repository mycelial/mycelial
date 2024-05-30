use std::collections::{BTreeMap, HashSet};

// Simple graph
#[derive(Debug)]
pub struct Graph<T: std::fmt::Debug> {
    nodes: BTreeMap<u64, T>,
    edges: BTreeMap<u64, u64>,
    // maps node type to associated element
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

    pub fn add_node(&mut self, id: u64, node: T) {
        self.nodes.insert(id, node);
    }

    pub fn get_node(&self, id: u64) -> Option<&T> {
        self.nodes.get(&id)
    }

    pub fn remove_node(&mut self, id: u64) {
        self.nodes.remove(&id);
        self.remove_edge(id);
        self.remove_edge_to(id);
    }

    pub fn iter_nodes(&self) -> impl Iterator<Item = (u64, &T)> + Clone + '_ {
        self.nodes.iter().map(|(id, node_state)| (*id, node_state))
    }

    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    // This function adds an edge from `from_node` to `to_node`.
    pub fn add_edge(&mut self, from_node: u64, to_node: u64) {
        // If `from_node` and `to_node` are the same, we do nothing and return.
        if from_node == to_node {
            return;
        }

        // If there are only two nodes in the graph, check for a loop manually.
        if self.nodes.len() == 2 {
            // If the `to_node` is the also a `from_node`,
            if self.edges.contains_key(&to_node) && self.edges[&to_node] == from_node {
                // A loop is detected, so we do nothing and return.
                return;
            }
        }

        // Temporarily add the edge to the `edges` HashMap.
        self.edges.insert(from_node, to_node);

        // Check if adding the edge has created a loop in the graph.
        let has_loop: bool = self.has_loop();

        // If a loop has been detected, remove the edge.
        if has_loop {
            self.remove_edge(from_node);
        }
    }

    // This function checks if there is a loop in the graph.
    fn has_loop(&self) -> bool {
        // Iterate over all nodes in the graph.
        for node in self.nodes.keys() {
            // Create a HashSet to keep track of visited nodes.
            let mut visited: HashSet<u64> = HashSet::new();

            // Perform a depth-first search from the current node.
            let has_loop_from_node: bool = self.dfs(*node, *node, &mut visited);

            // If a loop is detected, return true.
            if has_loop_from_node {
                return true;
            }
        }

        // If no loop is detected, return false.
        false
    }

    // This function performs a depth-first search from the current node.
    fn dfs(&self, current: u64, parent: u64, visited: &mut std::collections::HashSet<u64>) -> bool {
        // Add the current node to the set of visited nodes.
        visited.insert(current);

        // If the current node has an outgoing edge,
        if let Some(&next) = self.edges.get(&current) {
            // and the next node is not the parent node and has been visited before,
            if next != parent && visited.contains(&next) {
                // then a loop has been detected, so return true.
                return true;
            }

            // If the next node has not been visited before,
            if !visited.contains(&next) {
                // perform a depth-first search from the next node.
                let has_loop_from_next: bool = self.dfs(next, current, visited);

                // If a loop is detected, return true.
                if has_loop_from_next {
                    return true;
                }
            }
        }

        // If no loop is detected, return false.
        false
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

    pub fn remove_edge(&mut self, from_node: u64) {
        self.edges.remove(&from_node);
    }

    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }

    fn remove_edge_to(&mut self, to_node: u64) {
        self.iter_edges()
            .fold(vec![], |mut acc, (from, to)| {
                if to == to_node {
                    acc.push(from)
                }
                acc
            })
            .into_iter()
            .for_each(|from| {
                self.remove_edge(from);
            })
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

            input
                .iter()
                .for_each(|&id| graph.add_node(id, TestNode { id }));
            let nodes = graph
                .iter_nodes()
                .map(|(id, &node)| {
                    assert_eq!(id, node.id);
                    (id, node)
                })
                .collect::<Vec<_>>();
            assert_eq!(unique_ids.len(), nodes.len());

            for id in input.iter().copied() {
                graph.remove_node(id)
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
            input
                .iter()
                .for_each(|&id| graph.add_node(id, TestNode { id }));
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
            input
                .iter()
                .for_each(|&id| graph.add_node(id, TestNode { id }));
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
    fn test_graph_no_loops_created() {
        let mut graph = Graph::new();
        for i in 0..10 {
            graph.add_node(i, format!("node{}", i));
        }
        for i in 0..9 {
            graph.add_edge(i, i + 1);
        }
        // Attempt to create a loop
        graph.add_edge(9, 0);
        // Check that no loop was created
        assert!(!graph.has_loop());
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
            nodes
                .iter()
                .for_each(|&id| graph.add_node(id, TestNode { id }));
            nodes
                .as_slice()
                .windows(2)
                .for_each(|pair| graph.add_edge(pair[0], pair[1]));
            graph.add_edge(*nodes.last().unwrap(), *nodes.first().unwrap());
            assert!(graph.get_child_node(*nodes.last().unwrap()).is_none());
            TestResult::from_bool(true)
        };
        quickcheck::quickcheck(check as fn(Vec<u64>) -> TestResult)
    }
}

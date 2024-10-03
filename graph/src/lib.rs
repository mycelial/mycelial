use std::collections::{BTreeMap, BTreeSet};
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

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
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

    // iter all nodes, including 'dangling' ones
    pub fn all_nodes(&self) -> BTreeSet<K> {
        self.nodes.keys().copied()
            .chain(self.edges.iter().flat_map(|(from, to)| [*from, *to]))
            .collect()
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
        if self.check_loop(from_node, to_node) {
            return ops;
        }
        if let Some(prev_node) = self.edges.insert(from_node, to_node) {
            ops.push(GraphOperation::RemoveEdge(from_node, prev_node));
        }
        ops.push(GraphOperation::AddEdge(from_node, to_node));
        ops
    }

    pub fn get_edge(&self, from_node: K) -> Option<K> {
        self.edges.get(&from_node).copied()
    }

    fn check_loop(&self, from_node: K, to_node: K) -> bool {
        let mut visited = BTreeSet::<K>::from_iter([from_node, to_node]);
        let mut next = to_node;
        while let Some(node) = self.edges.get(&next).copied() {
            if !visited.insert(node) {
                return true;
            };
            next = node;
        }
        false
    }

    // Add edge partial edge
    //
    // Function doesn't generate graph operations since it's not supposed to be used by UI.
    // For now function is used only in myceliald scheduler.
    // Connected nodes of the graph can be scheduled on different machines.
    // Edge, which connects such nodes can't be added to graph via `add_edge` function, since from_node or to_node
    // doesn't exist in such partial graph.
    // if both from_node and to_node exists in the graph - checked add edge will be performed to avoid forming loops
    pub fn add_edge_partial(&mut self, from_node: K, to_node: K) {
        match (
            self.nodes.contains_key(&from_node),
            self.nodes.contains_key(&to_node),
        ) {
            (true, true) => {
                self.add_edge(from_node, to_node);
            }
            (from, to) if from ^ to && !self.check_loop(from_node, to_node) => {
                self.edges.insert(from_node, to_node);
            }
            _ => (),
        }
    }

    pub fn add_edge_unchecked(&mut self, from_node: K, to_node: K) {
        self.edges.insert(from_node, to_node);
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

    pub fn get_subgraphs(&self) -> Vec<Graph<K, V>> {
        let mut graphs = Vec::<Graph<K, V>>::new();
        let mut stack = Vec::<StackItem<K, V>>::new();
        let mut visited = BTreeMap::<K, usize>::new();

        #[derive(Debug)]
        enum StackItem<K, V> {
            Node(K, V),
            Edge(K, K),
        }

        for node in self.all_nodes() {
            if visited.contains_key(&node) {
                continue;
            }
            let mut graph_index = graphs.len();
            let mut key = node;
            loop {
                if let Some(index) = visited.get(&key) {
                    graph_index = *index;
                    break;
                }
                // graph can be partial
                if let Some(node) = self.nodes.get(&key) {
                    stack.push(StackItem::Node(key, node.clone()));
                }
                match self.edges.get(&key) {
                    Some(to_node) if self.nodes.contains_key(to_node) => {
                        stack.push(StackItem::Edge(key, *to_node));
                        key = *to_node;
                    }
                    Some(to_node) => {
                        stack.push(StackItem::Edge(key, *to_node));
                        graph_index = graphs.len();
                        graphs.push(Graph::new());
                        break;
                    }
                    None => {
                        if !stack.is_empty() {
                            graphs.push(Graph::new());
                        }
                        break;
                    }
                }
            }
            if stack.is_empty() {
                continue;
            }
            let graph = graphs.get_mut(graph_index).unwrap();
            while let Some(item) = stack.pop() {
                match item {
                    StackItem::Node(key, node) => {
                        if self.nodes.contains_key(&key) {
                            visited.insert(key, graph_index);
                        }
                        graph.add_node(key, node);
                    }
                    StackItem::Edge(from, to) => {
                        if self.nodes.contains_key(&to) {
                            visited.insert(to, graph_index);
                        }
                        // edge can be inserted before node
                        // so we need to add edge without any checks here
                        // it's safe, since stack of items can be build only from checked graph
                        graph.add_edge_unchecked(from, to)
                    }
                }
            }
        }
        graphs
    }
}

#[cfg(test)]
mod test {
    use std::collections::BTreeMap;

    use quickcheck::TestResult;

    use super::*;

    #[derive(Debug, Clone, Copy, PartialEq)]
    struct TestNode {
        id: u64,
    }

    #[test]
    fn graph_add_iter_remove_node() {
        let check = |input: Vec<u64>| -> TestResult {
            let mut graph = Graph::new();
            let unique_ids: BTreeSet<u64> = BTreeSet::from_iter(input.iter().copied());

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
    fn graph_add_iter_remove_edges() {
        let check = |input: Vec<u64>| -> TestResult {
            let mut graph = Graph::new();
            input.iter().for_each(|&id| {
                graph.add_node(id, TestNode { id });
            });
            let unique_ids: BTreeSet<u64> = BTreeSet::from_iter(input.iter().copied());
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
    fn graph_edges_cleanup_on_node_removal() {
        let check = |input: Vec<u64>| -> TestResult {
            let mut graph = Graph::new();
            input.iter().for_each(|&id| {
                graph.add_node(id, TestNode { id });
            });
            let unique_ids: BTreeSet<u64> = BTreeSet::from_iter(input.iter().copied());
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
    fn graph_edge_loop() {
        let mut graph = Graph::new();
        graph.add_node(0, TestNode { id: 0 });
        graph.add_edge(0, 0);
        assert_eq!(graph.edge_count(), 0);
    }

    #[test]
    fn prop_no_loops() {
        let check = |nodes: Vec<u64>| -> TestResult {
            let nodes = Vec::from_iter(
                BTreeSet::<u64>::from_iter(nodes.iter().copied())
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

    #[test]
    fn subgraphs() {
        let build_graph = |nodes: &mut dyn Iterator<Item = usize>| {
            let mut graph = Graph::new();
            nodes.for_each(|node| {
                graph.add_node(node, node);
            });
            graph
        };
        let add_edges = |graph: &mut Graph<usize, usize>,
                         edges: &mut dyn Iterator<Item = (usize, usize)>| {
            edges.for_each(|(from, to)| {
                graph.add_edge(from, to);
            });
        };

        let mut graph = build_graph(&mut (1..10));
        add_edges(
            &mut graph,
            &mut [(1, 2), (2, 4), (5, 2), (3, 4), (6, 7), (8, 3)].into_iter(),
        );

        assert_eq!(
            vec![
                {
                    let mut sub_graph = build_graph(&mut [1, 2, 3, 4, 5, 8].into_iter());
                    add_edges(
                        &mut sub_graph,
                        &mut [(1, 2), (2, 4), (3, 4), (5, 2), (8, 3)].into_iter(),
                    );
                    sub_graph
                },
                {
                    let mut sub_graph = build_graph(&mut [6, 7].into_iter());
                    add_edges(&mut sub_graph, &mut [(6, 7)].into_iter());
                    sub_graph
                },
                { build_graph(&mut [9].into_iter()) },
            ],
            graph.get_subgraphs()
        );
    }

    #[test]
    fn subgraphs_partial_graph() {
        let mut graph = Graph::new();
        graph.add_node(1, 1);
        graph.add_edge_partial(1, 2);
        graph.add_node(3, 3);
        graph.add_edge_partial(4, 3);
        assert_eq!(
            vec![
                {
                    let mut graph = Graph::new();
                    graph.add_node(1, 1);
                    graph.add_edge_partial(1, 2);
                    graph
                },
                {
                    let mut graph = Graph::new();
                    graph.add_node(3, 3);
                    graph.add_edge_partial(4, 3);
                    graph
                }
            ],
            graph.get_subgraphs()
        );
    }

    #[derive(Debug, Clone, Copy)]
    struct XorShift {
        state: u64,
    }

    impl XorShift {
        fn new(state: u64) -> Self {
            Self {
                state: state.max(1),
            }
        }
    }

    impl Iterator for XorShift {
        type Item = u64;

        fn next(&mut self) -> Option<Self::Item> {
            self.state ^= self.state << 13;
            self.state ^= self.state >> 7;
            self.state ^= self.state << 17;
            Some(self.state)
        }
    }

    #[test]
    // splitted subgraphs in sum should result in exactly same graph
    fn prop_subgraph_node_edge_count() {
        let check = |prng_state: u64, edges: BTreeMap<u8, u8>| -> TestResult {
            let mut prng = XorShift::new(prng_state);
            let mut graph = Graph::new();
            let mut initial_nodes = BTreeSet::new();
            for (from, to) in edges.iter().map(|(from, to)| (*from, *to)) {
                // randomize dangling side
                let node = match prng.next().unwrap() % 2 {
                    0 => from,
                    _ => to,
                };
                graph.add_node(node, node);
                initial_nodes.insert(node);
                graph.add_edge_partial(from, to);
            }
            let mut subgraph_total_nodes = BTreeSet::new();
            let mut subgraph_total_edges = BTreeMap::new();
            for graph in graph.get_subgraphs() {
                for (key, _) in graph.iter_nodes() {
                    subgraph_total_nodes.insert(key);
                }
                for (from, to) in graph.iter_edges() {
                    subgraph_total_edges.insert(from, to);
                }
            }
            assert_eq!(initial_nodes, subgraph_total_nodes);
            assert_eq!(
                graph.iter_edges().collect::<BTreeMap<_, _>>(),
                subgraph_total_edges
            );
            TestResult::from_bool(true)
        };
        quickcheck::quickcheck(check as fn(u64, BTreeMap<u8, u8>) -> TestResult);
    }

    #[test]
    // validate graphs are splitted correctly through alternative implementation of sub-graphing
    fn prop_subgraph_validity() {
        let check = |prng_state: u64, edges: BTreeMap<u8, u8>| -> TestResult {
            let mut prng = XorShift::new(prng_state);
            let mut initial_graph = Graph::new();
            let mut initial_nodes = BTreeSet::new();
            for (from, to) in edges.iter().map(|(from, to)| (*from, *to)) {
                // randomize dangling side
                let node = match prng.next().unwrap() % 2 {
                    0 => from,
                    _ => to,
                };
                initial_graph.add_node(node, node);
                initial_nodes.insert(node);
                initial_graph.add_edge_partial(from, to);
            }
            let mut graphs = vec![];
            let mut nodes = initial_graph.all_nodes().into_iter().collect::<Vec<_>>();
            let edges_hashmap = initial_graph.iter_edges().collect::<BTreeMap<_, _>>();
            let mut visited = BTreeSet::<u8>::new();
            while let Some(node) = nodes.pop() {
                if visited.contains(&node) {
                    continue;
                }
                let mut stack = vec![node];

                #[derive(Debug)]
                enum GraphOp {
                    AddNode(u8),
                    AddEdge(u8, u8),
                }
                let mut graph_ops: Vec<GraphOp> = vec![];

                while let Some(node) = stack.pop() {
                    if !initial_nodes.contains(&node) {
                        continue;
                    }
                    graph_ops.push(GraphOp::AddNode(node));
                    edges_hashmap.iter().fold(&mut stack, |stack, (&f, &t)| {
                        // f is a parent
                        if t == node {
                            if !visited.contains(&f) {
                                if initial_nodes.contains(&f) {
                                    stack.push(f);
                                }
                                visited.insert(f);
                            }
                            graph_ops.push(GraphOp::AddEdge(f, node));
                        }
                        // t is a child
                        if f == node {
                            if !visited.contains(&t) {
                                if initial_nodes.contains(&t) {
                                    stack.push(t);
                                }
                                visited.insert(t);
                            }
                            graph_ops.push(GraphOp::AddEdge(node, t));
                        };
                        stack
                    });
                }
                if !graph_ops.is_empty() {
                    let graph = {
                        graphs.push(Graph::new());
                        graphs.last_mut().unwrap()
                    };
                    while let Some(op) = graph_ops.pop() {
                        match op {
                            GraphOp::AddNode(node) => {
                                graph.add_node(node, node);
                            }
                            GraphOp::AddEdge(from, to) => graph.add_edge_unchecked(from, to),
                        }
                    }
                }
            }
            let mut subgraphs = initial_graph.get_subgraphs();
            subgraphs.sort();
            graphs.sort();
            assert_eq!(subgraphs, graphs, "initial graph: {:?}", initial_graph);
            TestResult::from_bool(true)
        };
        quickcheck::quickcheck(check as fn(u64, BTreeMap<u8, u8>) -> TestResult);
    }
}

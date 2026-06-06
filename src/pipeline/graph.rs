use bitvec::vec::BitVec;

use crate::{
    id::{IdContainer, NumericId},
    pipeline::NodeId,
};

pub(crate) struct Graph {
    adj: IdContainer<Vec<Vec<NodeId>>>,
}

#[allow(unused)]
impl Graph {
    #[inline]
    pub fn len(&self) -> usize {
        self.adj.len()
    }

    pub fn count_edges(&self) -> usize {
        self.adj.iter().map(|v| v.len()).sum()
    }

    pub fn from_edges<E>(edges: E) -> Self
    where
        E: IntoIterator<Item = (NodeId, NodeId)>,
    {
        let mut adj = IdContainer::new(vec![]);

        for (from, to) in edges.into_iter() {
            adj.resize(usize::max(from.as_index(), to.as_index()) + 1, vec![]);

            adj[from].push(to);
        }

        Graph { adj }
    }

    /// Constructs a new graph (`H`) from this one (`G`) such that:
    /// H = { (b, a) | (a, b) ∈ G }
    pub fn transpose(&self) -> Self {
        let mut h = IdContainer::new(vec![vec![]; self.adj.len()]);

        for (node, neighbors) in self.adj.iter().enumerate() {
            for &neighbor in neighbors {
                h[neighbor].push(NodeId::from_index(node));
            }
        }

        Self { adj: h }
    }

    pub fn edges(&self) -> impl Iterator<Item = (NodeId, NodeId)> {
        self.adj
            .iter()
            .enumerate()
            .flat_map(|(i, v)| v.iter().copied().map(move |to| (NodeId::from_index(i), to)))
    }

    pub fn neighbors(&self, node: NodeId) -> impl Iterator<Item = NodeId> {
        self.adj[node].iter().copied()
    }

    /// Constructs a valid topological order and returns it.
    /// If there is a cycle, returns [`None`].
    ///
    /// If you can guarantee that the graph is a DAG, consider [`Graph::toposort_acyclic`]
    /// for a faster alternative.
    pub fn toposort(&self) -> Option<Vec<NodeId>> {
        let order = unsafe { self.toposort_acyclic() };

        let mut node_to_ord_idx = IdContainer::new(vec![0; order.len()]);

        for (i, &node) in order.iter().enumerate() {
            node_to_ord_idx[node] = i;
        }

        for (from, to) in self.edges() {
            if node_to_ord_idx[from] >= node_to_ord_idx[to] {
                return None;
            }
        }

        Some(order)
    }

    /// Constructs a valid topological order and returns it.
    ///
    /// # Safety
    /// If the graph has cycles, silently gives an invalid topological order.
    pub unsafe fn toposort_acyclic(&self) -> Vec<NodeId> {
        let mut stack = Vec::with_capacity(self.adj.len());

        let mut vis = IdContainer::new(BitVec::repeat(false, self.adj.len()));

        for index in 0..self.adj.len() {
            let node_id = NodeId::from_index(index);

            self.toposort_dfs(&mut stack, &mut vis, node_id);
        }

        stack.reverse();

        stack
    }

    fn toposort_dfs(
        &self,
        stack: &mut Vec<NodeId>,
        vis: &mut IdContainer<BitVec>,
        current: NodeId,
    ) {
        if vis[current] {
            return;
        }

        vis.set(current.as_index(), true);

        for &neighbor in &self.adj[current] {
            self.toposort_dfs(stack, vis, neighbor);
        }

        stack.push(current);
    }
}

impl FromIterator<(NodeId, NodeId)> for Graph {
    #[inline]
    fn from_iter<T: IntoIterator<Item = (NodeId, NodeId)>>(iter: T) -> Self {
        Self::from_edges(iter)
    }
}

#[cfg(test)]
mod test {
    use std::collections::HashSet;

    use crate::{
        id::NumericId,
        pipeline::{NodeId, graph::Graph},
    };

    macro_rules! adj {
        ($($n:literal),* $(,)?) => {
            vec![
                $(
                    NodeId::from_index($n - 1)
                ),*
            ]
        };
    }

    macro_rules! edge {
        ($n:literal, $m:literal) => {
            (NodeId::from_index($n - 1), NodeId::from_index($m - 1))
        };
    }

    fn is_valid_topo_order(ord: &[NodeId], g: &Graph) -> bool {
        for (from, to) in g.edges() {
            let from_index = ord.iter().position(|&x| x == from);
            let to_index = ord.iter().position(|&x| x == to);

            match (from_index, to_index) {
                (Some(from_index), Some(to_index)) => {
                    if from_index >= to_index {
                        eprintln!("{} -> {} broken", from.as_index(), to.as_index());
                        return false;
                    }
                }
                _ => return false,
            }
        }

        true
    }

    #[test]
    fn edges() {
        let graph = Graph {
            adj: vec![adj![2, 3, 4], adj![3], adj![], adj![1]].into(),
        };

        let expected: HashSet<_> = [
            edge!(1, 2),
            edge!(2, 3),
            edge!(1, 3),
            edge!(1, 4),
            edge!(4, 1),
        ]
        .into_iter()
        .collect();

        let got: HashSet<_> = graph.edges().collect();

        assert_eq!(expected, got);
    }

    #[test]
    fn from_edges() {
        let graph = Graph::from_edges([edge!(1, 2), edge!(4, 3), edge!(2, 4)]);

        assert_eq!(
            graph.adj.into_inner(),
            vec![adj![2], adj![4], adj![], adj![3]]
        );
    }

    #[test]
    fn transpose() {
        let g = Graph {
            adj: vec![adj![2, 3], adj![3], adj![1, 2]].into(),
        };

        let h = g.transpose();

        assert_eq!(h.adj.into_inner(), vec![adj![3], adj![1, 3], adj![1, 2]]);
    }

    #[test]
    fn toposort() {
        let g = Graph {
            adj: vec![
                adj![3, 5, 6],
                adj![5, 6],
                adj![4, 5],
                adj![],
                adj![6],
                adj![],
            ]
            .into(),
        };

        let ord = g.toposort().expect("topological order must exist");

        assert!(is_valid_topo_order(&ord, &g));
    }

    #[test]
    fn toposort_cycle() {
        let g = Graph {
            adj: vec![
                adj![3, 5, 6],
                adj![5, 6],
                adj![4, 5],
                adj![],
                adj![6],
                adj![1],
            ]
            .into(),
        };

        assert!(g.toposort().is_none());
    }
}

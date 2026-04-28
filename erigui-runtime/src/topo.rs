//! Topological sort of an `erigui_widgets::node_graph::Graph` using Kahn's
//! algorithm. Detects cycles. Returned order is the worker-thread execution
//! order: every node appears after all of its upstream producers.

use erigui_widgets::node_graph::Graph;
use std::collections::{HashMap, HashSet, VecDeque};

use crate::{ExecError, NodeId};

/// Run Kahn's algorithm over the graph's `edges` and return a deterministic
/// topological order. Returns [`ExecError::Cycle`] if any directed cycle
/// exists. Nodes with no edges still appear in the output.
///
/// `Graph::nodes[i].id` is `usize` in-memory; we narrow to the wave-1
/// workflow `u32` representation. A node id that doesn't fit `u32::MAX`
/// would be a bug elsewhere (the JSON envelope can't hold it); we treat it
/// as a missing-node error.
pub fn topo_sort(graph: &Graph) -> Result<Vec<NodeId>, ExecError> {
    let mut indegree: HashMap<NodeId, usize> = HashMap::with_capacity(graph.nodes.len());
    let mut adj: HashMap<NodeId, Vec<NodeId>> = HashMap::with_capacity(graph.nodes.len());
    let mut order: Vec<NodeId> = Vec::with_capacity(graph.nodes.len());

    // Seed every declared node with indegree 0.
    for node in &graph.nodes {
        let id = narrow(node.id)?;
        indegree.entry(id).or_insert(0);
        adj.entry(id).or_default();
    }

    // Walk edges, accumulate indegree.
    for edge in &graph.edges {
        let from = narrow(edge.from_node)?;
        let to = narrow(edge.to_node)?;
        if !indegree.contains_key(&from) {
            return Err(ExecError::MissingNode { node_id: from });
        }
        if !indegree.contains_key(&to) {
            return Err(ExecError::MissingNode { node_id: to });
        }
        adj.get_mut(&from).expect("seeded above").push(to);
        *indegree.get_mut(&to).expect("seeded above") += 1;
    }

    // Stable iteration order: visit nodes in graph.nodes declaration order
    // when seeding the queue so the resulting order is deterministic across
    // hash-randomization runs.
    let mut queue: VecDeque<NodeId> = VecDeque::new();
    let mut seen: HashSet<NodeId> = HashSet::new();
    for node in &graph.nodes {
        let id = narrow(node.id)?;
        if indegree.get(&id).copied().unwrap_or(0) == 0 && seen.insert(id) {
            queue.push_back(id);
        }
    }

    while let Some(id) = queue.pop_front() {
        order.push(id);
        // Children are pushed in the same order they appear in `adj`, which
        // is the order edges were declared — deterministic for stable output.
        if let Some(children) = adj.get(&id) {
            for &child in children {
                let entry = indegree.get_mut(&child).expect("seeded above");
                *entry -= 1;
                if *entry == 0 && seen.insert(child) {
                    queue.push_back(child);
                }
            }
        }
    }

    if order.len() != indegree.len() {
        return Err(ExecError::Cycle);
    }
    Ok(order)
}

fn narrow(id: usize) -> Result<NodeId, ExecError> {
    u32::try_from(id).map_err(|_| ExecError::MissingNode {
        node_id: u32::MAX, // sentinel; the original `usize` is too large
    })
}

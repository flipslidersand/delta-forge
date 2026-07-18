use petgraph::{graph::NodeIndex, Direction, Graph};
use std::collections::VecDeque;

use crate::node::Node;

/// BFS from `start` through outgoing edges, marking every reachable node dirty.
pub(crate) fn propagate_dirty(graph: &mut Graph<Node, ()>, start: NodeIndex) {
    let mut queue = VecDeque::new();
    queue.push_back(start);

    while let Some(idx) = queue.pop_front() {
        graph[idx].dirty = true;
        let neighbors: Vec<NodeIndex> = graph
            .neighbors_directed(idx, Direction::Outgoing)
            .collect();
        for next in neighbors {
            if !graph[next].dirty {
                queue.push_back(next);
            }
        }
    }
}

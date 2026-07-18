use std::any::Any;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use anyhow::{bail, Result};
use petgraph::algo::{is_cyclic_directed, toposort};
use petgraph::graph::NodeIndex;
use petgraph::Graph;
use rayon::prelude::*;

use crate::log::{Log, LogEntry};
use crate::node::{BoxedValue, Node, NodeKind};
use crate::scheduler::propagate_dirty;

/// Incremental computation engine.
///
/// Nodes are named; edges express "A depends on B" (B → A in the directed graph,
/// so B is upstream of A).
pub struct DeltaForge {
    graph: Graph<Node, ()>,
    /// name → NodeIndex
    index: HashMap<String, NodeIndex>,
    log: Log,
}

impl DeltaForge {
    pub fn new() -> Self {
        DeltaForge {
            graph: Graph::new(),
            index: HashMap::new(),
            log: Log::default(),
        }
    }

    // ── Node construction ─────────────────────────────────────────────────────

    /// Register a named input node (user sets the value via `set_input`).
    pub fn input(&mut self, name: &str) -> NodeIndex {
        let idx = self.graph.add_node(Node {
            name: name.to_string(),
            kind: NodeKind::Input,
            dirty: false,
            value: None,
            compute_fn: None,
        });
        self.index.insert(name.to_string(), idx);
        idx
    }

    /// Register a named compute node with a type-erased function.
    ///
    /// `f` receives a slice of its dependencies' values (in dependency order).
    pub fn compute<F>(&mut self, name: &str, f: F) -> NodeIndex
    where
        F: Fn(&[Option<&BoxedValue>]) -> BoxedValue + Send + Sync + 'static,
    {
        let idx = self.graph.add_node(Node {
            name: name.to_string(),
            kind: NodeKind::Compute,
            dirty: true,
            value: None,
            compute_fn: Some(Box::new(f)),
        });
        self.index.insert(name.to_string(), idx);
        idx
    }

    /// Add a dependency edge: `dep` → `node` (node depends on dep).
    pub fn add_dep(&mut self, node: &str, dep: &str) -> Result<()> {
        let node_idx = self.node_idx(node)?;
        let dep_idx = self.node_idx(dep)?;
        self.graph.add_edge(dep_idx, node_idx, ());
        if is_cyclic_directed(&self.graph) {
            // Remove the edge we just added to restore a valid state
            let eid = self.graph.find_edge(dep_idx, node_idx).unwrap();
            self.graph.remove_edge(eid);
            bail!("adding dep '{dep}' → '{node}' would create a cycle");
        }
        Ok(())
    }

    // ── Value access ──────────────────────────────────────────────────────────

    /// Set an input node's value and mark all downstream nodes dirty.
    pub fn set_input<T: Any + Send + Sync>(&mut self, name: &str, value: T) -> Result<()> {
        let idx = self.node_idx(name)?;
        if self.graph[idx].kind != NodeKind::Input {
            bail!("'{name}' is not an Input node");
        }
        self.graph[idx].value = Some(Box::new(value));
        propagate_dirty(&mut self.graph, idx);
        // Input itself doesn't need recomputation, clear its dirty flag
        self.graph[idx].dirty = false;
        Ok(())
    }

    /// Get the cached value of a node, downcast to `T`.
    pub fn get<T: Any + Clone + Send + Sync>(&self, name: &str) -> Result<T> {
        let idx = self.node_idx(name)?;
        let val = self.graph[idx]
            .value
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("'{name}' has no value (run topo_recompute first)"))?;
        val.downcast_ref::<T>()
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("'{name}' value type mismatch"))
    }

    // ── Recomputation ─────────────────────────────────────────────────────────

    /// Topological recompute: runs dirty Compute nodes in dependency order.
    pub fn topo_recompute(&mut self) -> Result<()> {
        let order = toposort(&self.graph, None)
            .map_err(|_| anyhow::anyhow!("cycle detected during toposort"))?;

        for idx in order {
            if !self.graph[idx].dirty || self.graph[idx].kind == NodeKind::Input {
                continue;
            }

            let t0 = Instant::now();

            // Gather dep values
            let dep_indices: Vec<NodeIndex> = self
                .graph
                .neighbors_directed(idx, petgraph::Direction::Incoming)
                .collect();

            let dep_vals: Vec<Option<*const BoxedValue>> = dep_indices
                .iter()
                .map(|&d| self.graph[d].value.as_ref().map(|v| v as *const _))
                .collect();

            // SAFETY: We hold &mut self exclusively; no other borrows exist.
            let result = {
                let f = self.graph[idx].compute_fn.as_ref().ok_or_else(|| {
                    anyhow::anyhow!("Compute node '{}' has no function", self.graph[idx].name)
                })?;
                let args: Vec<Option<&BoxedValue>> = dep_vals
                    .iter()
                    .map(|p| p.map(|ptr| unsafe { &*ptr }))
                    .collect();
                f(&args)
            };

            let elapsed = t0.elapsed().as_micros() as u64;
            let name = self.graph[idx].name.clone();

            self.graph[idx].value = Some(result);
            self.graph[idx].dirty = false;

            self.log.push(LogEntry {
                node: name,
                recomputed: true,
                duration_us: elapsed,
            });
        }
        Ok(())
    }

    /// Parallel topological recompute using Rayon.
    ///
    /// Nodes in the same topological "layer" (no dependency among them) run in parallel.
    pub fn topo_recompute_parallel(&mut self) -> Result<()> {
        let order = toposort(&self.graph, None)
            .map_err(|_| anyhow::anyhow!("cycle detected"))?;

        // Assign depth (layer) to each node: depth[n] = 1 + max(depth[dep])
        let mut depth: HashMap<NodeIndex, usize> = HashMap::new();
        for &idx in &order {
            let max_dep = self
                .graph
                .neighbors_directed(idx, petgraph::Direction::Incoming)
                .map(|d| depth.get(&d).copied().unwrap_or(0))
                .max()
                .unwrap_or(0);
            depth.insert(idx, max_dep + 1);
        }

        // Group nodes by layer
        let max_depth = depth.values().copied().max().unwrap_or(0);
        let mut layers: Vec<Vec<NodeIndex>> = vec![vec![]; max_depth + 1];
        for (&idx, &d) in &depth {
            layers[d].push(idx);
        }

        // Process each layer; nodes within a layer are independent
        for layer in &layers {
            let dirty: Vec<NodeIndex> = layer
                .iter()
                .copied()
                .filter(|&idx| {
                    self.graph[idx].dirty && self.graph[idx].kind == NodeKind::Compute
                })
                .collect();

            if dirty.is_empty() {
                continue;
            }

            // Compute all nodes in this layer in parallel
            let results: Vec<(NodeIndex, BoxedValue, u64)> = dirty
                .par_iter()
                .map(|&idx| {
                    let t0 = Instant::now();
                    let dep_indices: Vec<NodeIndex> = self
                        .graph
                        .neighbors_directed(idx, petgraph::Direction::Incoming)
                        .collect();

                    let dep_vals: Vec<Option<*const BoxedValue>> = dep_indices
                        .iter()
                        .map(|&d| self.graph[d].value.as_ref().map(|v| v as *const _))
                        .collect();

                    let f = self.graph[idx].compute_fn.as_ref().unwrap();
                    // SAFETY: dep nodes are in earlier layers and already computed;
                    // no two nodes in this layer share edges.
                    let args: Vec<Option<&BoxedValue>> = dep_vals
                        .iter()
                        .map(|p| p.map(|ptr| unsafe { &*ptr }))
                        .collect();
                    let result = f(&args);
                    (idx, result, t0.elapsed().as_micros() as u64)
                })
                .collect();

            for (idx, val, elapsed) in results {
                let name = self.graph[idx].name.clone();
                self.graph[idx].value = Some(val);
                self.graph[idx].dirty = false;
                self.log.push(LogEntry {
                    node: name,
                    recomputed: true,
                    duration_us: elapsed,
                });
            }
        }
        Ok(())
    }

    // ── Utilities ─────────────────────────────────────────────────────────────

    pub fn print_log(&self) {
        self.log.print();
    }

    pub fn log(&self) -> &Log {
        &self.log
    }

    pub fn clear_log(&mut self) {
        self.log.clear();
    }

    fn node_idx(&self, name: &str) -> Result<NodeIndex> {
        self.index
            .get(name)
            .copied()
            .ok_or_else(|| anyhow::anyhow!("node '{name}' not found"))
    }
}

impl Default for DeltaForge {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn build_dag() -> DeltaForge {
        let mut df = DeltaForge::new();
        df.input("a");
        df.input("b");
        df.compute("sum", |deps| {
            let a = deps[0].and_then(|v| v.downcast_ref::<i64>()).copied().unwrap_or(0);
            let b = deps[1].and_then(|v| v.downcast_ref::<i64>()).copied().unwrap_or(0);
            Box::new(a + b)
        });
        df.add_dep("sum", "a").unwrap();
        df.add_dep("sum", "b").unwrap();
        df
    }

    #[test]
    fn basic_recompute() {
        let mut df = build_dag();
        df.set_input("a", 3_i64).unwrap();
        df.set_input("b", 4_i64).unwrap();
        df.topo_recompute().unwrap();
        assert_eq!(df.get::<i64>("sum").unwrap(), 7);
    }

    #[test]
    fn incremental_only_recomputes_dirty() {
        let mut df = build_dag();
        df.set_input("a", 1_i64).unwrap();
        df.set_input("b", 2_i64).unwrap();
        df.topo_recompute().unwrap();
        assert_eq!(df.log().entries().len(), 1); // only "sum"

        df.clear_log();
        // change b → sum should recompute; a is untouched
        df.set_input("b", 10_i64).unwrap();
        df.topo_recompute().unwrap();
        assert_eq!(df.get::<i64>("sum").unwrap(), 11);
        assert_eq!(df.log().entries().len(), 1);
    }

    #[test]
    fn cycle_detection() {
        let mut df = DeltaForge::new();
        df.input("x");
        df.compute("y", |_| Box::new(0_i64));
        df.add_dep("y", "x").unwrap();
        // x depends on y → would form x→y→x cycle
        assert!(df.add_dep("x", "y").is_err());
    }

    #[test]
    fn parallel_recompute_matches_sequential() {
        let mut df = build_dag();
        df.set_input("a", 5_i64).unwrap();
        df.set_input("b", 6_i64).unwrap();
        df.topo_recompute_parallel().unwrap();
        assert_eq!(df.get::<i64>("sum").unwrap(), 11);
    }
}

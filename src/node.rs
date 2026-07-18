use std::any::Any;

/// A node is either a named input (user-provided value) or a derived computation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NodeKind {
    Input,
    Compute,
}

/// Runtime-erased cached value for a Compute node.
pub(crate) type BoxedValue = Box<dyn Any + Send + Sync>;

/// Metadata and state for a single graph node.
pub(crate) struct Node {
    pub name: String,
    pub kind: NodeKind,
    /// Whether this node needs recomputation.
    pub dirty: bool,
    /// Last computed / set value (type-erased).
    pub value: Option<BoxedValue>,
    /// Compute function for NodeKind::Compute.
    pub compute_fn: Option<Box<dyn Fn(&[Option<&BoxedValue>]) -> BoxedValue + Send + Sync>>,
}

impl std::fmt::Debug for Node {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Node")
            .field("name", &self.name)
            .field("kind", &self.kind)
            .field("dirty", &self.dirty)
            .finish()
    }
}

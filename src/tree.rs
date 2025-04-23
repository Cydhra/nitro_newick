pub type NodeId = usize;

pub struct UnrootedTree {
    nodes: Vec<TreeNode>,
}

pub struct TreeNode {
    label: NodeLabel,
    edges: Vec<DirectedEdge>,
}

/// A directed edge in the tree.
/// The direction is arbitrary, as all trees are undirected and therefore each edge has a reverse
/// edge.
pub struct DirectedEdge {
    target: NodeId,
    support: Option<f64>,
    branch_length: Option<f64>,
}

pub enum NodeLabel {
    None,
    Name(String),
    Support(f64),
}

pub struct SimpleTreeBuilder {
    tree: TreeNode,
}

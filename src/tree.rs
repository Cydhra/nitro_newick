use std::mem;
use crate::TreeBuilder;

pub type NodeId = usize;

/// A simple unrooted tree structure.
/// The tree is represented as a vector of nodes, where each node contains a label and a list of
/// edges.
/// The edges are represented as directed edges, meaning each edge exists twice: once for each
/// direction.
/// Consequently, modification requires finding and modifying both edges.
pub struct UnrootedTree {
    nodes: Vec<TreeNode>,
}

impl UnrootedTree {

    /// Creates a new `UnrootedTree` with no nodes.
    fn new() -> Self {
        UnrootedTree {
            nodes: Vec::new(),
        }
    }

    /// Creates a new `UnrootedTree` with the specified node capacity, ensuring no reallocation occurs
    /// when adding nodes up to that capacity.
    fn with_capacity(capacity: usize) -> Self {
        UnrootedTree {
            nodes: Vec::with_capacity(capacity),
        }
    }

    /// Returns the number of nodes in the tree.
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Returns a reference to the node with the specified ID.
    /// Panics if the node ID is out of bounds.
    pub fn node(&self, id: NodeId) -> &TreeNode {
        &self.nodes[id]
    }

    /// Returns a mutable reference to the node with the specified ID.
    /// Panics if the node ID is out of bounds.
    pub fn node_mut(&mut self, id: NodeId) -> &mut TreeNode {
        &mut self.nodes[id]
    }
}

/// A node in the tree.
pub struct TreeNode {
    pub label: Option<String>,
    edges: Vec<DirectedEdge>,
}

impl TreeNode {
    /// Creates a new `TreeNode` with the specified label and an empty list of edges.
    pub fn new(label: Option<String>) -> Self {
        TreeNode {
            label,
            edges: Vec::new(),
        }
    }

    /// Returns the edges of the node.
    pub fn edges(&self) -> &[DirectedEdge] {
        &self.edges
    }
}

/// A directed edge in the tree.
/// The direction is arbitrary, as all trees are undirected and therefore each edge has a reverse
/// edge containing the same support and branch length values.
pub struct DirectedEdge {
    target: NodeId,
    pub support: Option<f64>,
    pub branch_length: Option<f64>,
}

impl DirectedEdge {
    /// Creates a new `DirectedEdge` with the specified target node ID, support value, and branch
    /// length.
    pub fn new(target: NodeId, support: Option<f64>, branch_length: Option<f64>) -> Self {
        DirectedEdge {
            target,
            support,
            branch_length,
        }
    }

    /// Returns the target node ID of the edge.
    pub fn target(&self) -> NodeId {
        self.target
    }
}

/// A [`TreeBuilder`] implementation for creating [`UnrootedTree`]s for use in the [`Parser`].
/// 
/// [`TreeBuilder`]: crate::TreeBuilder
/// [`Parser`]: crate::parser::Parser
/// [`UnrootedTree`]: UnrootedTree
pub struct SimpleTreeBuilder {
    tree: UnrootedTree,
}

impl SimpleTreeBuilder {
    /// Creates a new `SimpleTreeBuilder` with no nodes.
    pub fn new() -> Self {
        SimpleTreeBuilder {
            tree: UnrootedTree::new(),
        }
    }

    /// Creates a new `SimpleTreeBuilder` with the specified node capacity, ensuring no reallocation
    /// occurs when adding nodes up to that capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        SimpleTreeBuilder {
            tree: UnrootedTree::with_capacity(capacity),
        }
    }
}

impl TreeBuilder for SimpleTreeBuilder {
    type Tree = UnrootedTree;
    type NodeId = NodeId;

    fn build(&mut self) -> Self::Tree {
        let mut new_tree = UnrootedTree::new();
        mem::swap(&mut self.tree, &mut new_tree);
        new_tree
    }

    fn add_node(&mut self, label: Option<String>) -> Self::NodeId {
        let node_id = self.tree.nodes.len();
        self.tree.nodes.push(TreeNode::new(label));
        node_id
    }

    fn add_edge(&mut self, parent: Self::NodeId, child: Self::NodeId, support: Option<f64>, branch_length: Option<f64>) {
        self.tree.nodes[parent].edges.push(DirectedEdge::new(child, support, branch_length));
        self.tree.nodes[child].edges.push(DirectedEdge::new(parent, support, branch_length));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tree_builder() {
        let mut builder = SimpleTreeBuilder::new();
        let node1 = builder.add_node(Some("A".to_string()));
        let node2 = builder.add_node(Some("B".to_string()));
        let node3 = builder.add_node(Some("C".to_string()));

        builder.add_edge(node1, node2, Some(0.9), Some(0.5));
        builder.add_edge(node2, node3, Some(0.8), Some(0.3));

        let tree = builder.build();

        assert_eq!(tree.node_count(), 3);
        assert_eq!(tree.node(node1).label, Some("A".to_string()));
        assert_eq!(tree.node(node2).label, Some("B".to_string()));
        assert_eq!(tree.node(node3).label, Some("C".to_string()));
        
        assert_eq!(tree.node(node1).edges().len(), 1);
        assert_eq!(tree.node(node1).edges()[0].target, node2);
        assert_eq!(tree.node(node2).edges().len(), 2);
        assert_eq!(tree.node(node2).edges()[0].target, node1);
        assert_eq!(tree.node(node2).edges()[1].target, node3);
        
        assert_eq!(tree.node(node3).edges().len(), 1);
        assert_eq!(tree.node(node3).edges()[0].target, node2);
        
        assert_eq!(tree.node(node1).edges()[0].support, Some(0.9));
        assert_eq!(tree.node(node1).edges()[0].branch_length, Some(0.5));
        assert_eq!(tree.node(node2).edges()[0].support, Some(0.9));
        assert_eq!(tree.node(node2).edges()[0].branch_length, Some(0.5));
        assert_eq!(tree.node(node2).edges()[1].support, Some(0.8));
        assert_eq!(tree.node(node2).edges()[1].branch_length, Some(0.3));
        assert_eq!(tree.node(node3).edges()[0].support, Some(0.8));
        assert_eq!(tree.node(node3).edges()[0].branch_length, Some(0.3));
    }
}
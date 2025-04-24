use crate::TreeBuilder;
use std::mem;

pub type NodeId = usize;

/// A traversal order of the nodes in the tree.
/// This is a list of node IDs in the order they are to be traversed.
/// The order is arbitrary and can be used for various purposes, such as
/// depth-first search or breadth-first search.
/// The order is not guaranteed to visit each node, or to visit a node just once.
pub type TraversalOrder = [NodeId];

/// A simple unrooted tree structure.
/// The tree is represented as a vector of nodes, where each node contains a label and a list of
/// edges.
/// The edges are represented as directed edges, meaning each edge exists twice: once for each
/// direction.
/// Consequently, modification requires finding and modifying both edges.
pub struct UnrootedTree {
    nodes: Vec<TreeNode>,
    virtual_root: Option<NodeId>,
}

impl UnrootedTree {
    /// Creates a new `UnrootedTree` with no nodes.
    fn new() -> Self {
        UnrootedTree {
            nodes: Vec::new(),
            virtual_root: None,
        }
    }

    /// Creates a new `UnrootedTree` with the specified node capacity, ensuring no reallocation occurs
    /// when adding nodes up to that capacity.
    fn with_capacity(capacity: usize) -> Self {
        UnrootedTree {
            nodes: Vec::with_capacity(capacity),
            virtual_root: None,
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

    /// Returns the virtual root of the tree.
    /// Returns `None` if the tree has no virtual root.
    pub fn virtual_root(&self) -> Option<NodeId> {
        self.virtual_root
    }

    /// Generate a post-order traversal of the tree starting from the specified root node.
    pub fn post_order(&self, root: NodeId) -> Vec<NodeId> {
        let mut order = Vec::with_capacity(self.node_count());
        let mut stack = Vec::with_capacity(self.node_count() << 1);
        stack.push((root, root));

        while let Some((parent, node)) = stack.pop() {
            order.push(node);
            for edge in self.nodes[node].edges() {
                if edge.target() != parent {
                    stack.push((node, edge.target()));
                }
            }
        }

        order.reverse();
        order
    }

    /// Returns the length of the tree, which is the number of nodes in the tree.
    /// This is equivalent to the `node_count()` method.
    pub fn len(&self) -> usize {
        self.node_count()
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
        self.tree.virtual_root = Some(node_id);
        node_id
    }

    fn add_edge(
        &mut self,
        parent: Self::NodeId,
        child: Self::NodeId,
        support: Option<f64>,
        branch_length: Option<f64>,
    ) {
        self.tree.nodes[parent]
            .edges
            .push(DirectedEdge::new(child, support, branch_length));
        self.tree.nodes[child]
            .edges
            .push(DirectedEdge::new(parent, support, branch_length));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::Parser;

    #[test]
    fn test_tree_builder() {
        let mut builder = SimpleTreeBuilder::new();
        let node3 = builder.add_node(Some("C".to_string()));
        let node2 = builder.add_node(Some("B".to_string()));
        let node1 = builder.add_node(Some("A".to_string()));

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

        assert_eq!(tree.post_order(2), vec![node3, node2, node1]);
    }

    #[test]
    fn test_parsing() {
        let newick = "(A:0.5,(B:0.8,C:0.2)D:0.1)R;";
        let builder = SimpleTreeBuilder::new();
        let mut parser = Parser::new(newick.as_bytes(), builder);
        let result = parser.parse().expect("Parsing failed.");
        let tree = result.expect("Parser returned no tree.");

        assert_eq!(tree.node_count(), 5);
        assert_eq!(tree.node(0).label, Some(String::from("A")));
        assert_eq!(tree.node(1).label, Some(String::from("B")));
        assert_eq!(tree.node(2).label, Some(String::from("C")));
        assert_eq!(tree.node(3).label, Some(String::from("D")));
        assert_eq!(tree.node(4).label, Some(String::from("R")));

        assert_eq!(tree.node(0).edges().len(), 1);
        assert_eq!(tree.node(0).edges()[0].target, 4);
        assert_eq!(tree.node(0).edges()[0].branch_length, Some(0.5));
        assert_eq!(tree.node(1).edges().len(), 1);
        assert_eq!(tree.node(1).edges()[0].target, 3);
        assert_eq!(tree.node(1).edges()[0].branch_length, Some(0.8));
        assert_eq!(tree.node(2).edges().len(), 1);
        assert_eq!(tree.node(2).edges()[0].target, 3);
        assert_eq!(tree.node(2).edges()[0].branch_length, Some(0.2));
        assert_eq!(tree.node(3).edges().len(), 3);
        assert_eq!(tree.node(3).edges()[0].target, 1);
        assert_eq!(tree.node(3).edges()[0].branch_length, Some(0.8));
        assert_eq!(tree.node(3).edges()[1].target, 2);
        assert_eq!(tree.node(3).edges()[1].branch_length, Some(0.2));
        assert_eq!(tree.node(3).edges()[2].target, 4);
        assert_eq!(tree.node(3).edges()[2].branch_length, Some(0.1));
        assert_eq!(tree.node(4).edges().len(), 2);
        assert_eq!(tree.node(4).edges()[0].target, 0);
        assert_eq!(tree.node(4).edges()[0].branch_length, Some(0.5));
        assert_eq!(tree.node(4).edges()[1].target, 3);
        assert_eq!(tree.node(4).edges()[1].branch_length, Some(0.1));

        assert_eq!(tree.post_order(4), vec![0, 1, 2, 3, 4]);
    }
}

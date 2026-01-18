use crate::tree::TreeError::{ChildNoParent, DiscordantEdgeData, ParentNoChild};
use crate::{TreeBuilder, TreeSerialize};
use std::{iter, mem};

pub type NodeId = usize;

/// A traversal order of the nodes in the tree.
/// This is a list of node IDs in the order they are to be traversed.
/// The order is arbitrary and can be used for various purposes, such as
/// depth-first search or breadth-first search.
/// The order is not guaranteed to visit each node, or to visit a node just once.
pub type TraversalOrder = [NodeId];

/// A simple (unrooted) tree structure.
/// The tree is represented as a vector of nodes, where each node contains a label and a list of
/// edges.
/// The edges are represented as directed edges, meaning each edge exists twice: once for each
/// direction.
/// Consequently, modifications to the tree topology require finding and modifying both edges.
///
/// The tree can be unrooted, in which case a virtual root is used, which points to one of the
/// tree's nodes.
/// This is why the tree has each edge twice, as (re-)rooting the tree changes the traversal direction
/// of some edges.
///
/// The tree does not contain any additional information that cannot be stored in the newick format.
/// Consequently, the structure is both `Send` and `Sync`, and parsing from and serializing to newick
/// is efficient.
#[derive(Clone, Debug)]
pub struct NTree {
    nodes: Vec<TreeNode>,
    virtual_root: Option<DirectedEdge>,
}

#[derive(Debug, Clone)]
pub enum TreeError {
    /// Error when removing an edge: Parent has no edge to the specified child.
    ParentNoChild,

    /// Error when removing an edge: Child has no edge to the specified parent.
    ChildNoParent,

    /// Edge data of the parent edge and the corresponding child edge are not equal.
    DiscordantEdgeData,
}

impl NTree {
    /// Creates a new `NTree` with no nodes.
    fn new() -> Self {
        NTree {
            nodes: Vec::new(),
            virtual_root: None,
        }
    }

    /// Creates a new `NTree` with the specified node capacity, ensuring no reallocation occurs
    /// when adding nodes up to that capacity.
    fn with_capacity(capacity: usize) -> Self {
        NTree {
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

    /// Returns an iterator over all nodes in the tree in arbitrary order.
    pub fn nodes(&self) -> impl Iterator<Item = &TreeNode> {
        self.nodes.iter()
    }

    /// Returns a mutable iterator over all nodes in the tree in arbitrary order.
    pub fn nodes_mut(&mut self) -> impl Iterator<Item = &mut TreeNode> {
        self.nodes.iter_mut()
    }

    /// Add a node to the tree. It will not be connected to the tree yet.
    /// The node ID is returned, which can be used to uniquely identify the node in the tree.
    ///
    /// The `edge_hint` parameter is used to provide a hint for the number of edges
    /// that will be added to the node later.
    pub fn add_node(&mut self, label: Option<String>, edge_hint: usize) -> NodeId {
        let node_id = self.nodes.len();
        self.nodes.push(TreeNode::with_capacity(label, edge_hint));
        node_id
    }

    /// Add an edge between two existing nodes in the tree.
    /// The assignment of parent and child is arbitrary if the tree is unrooted.
    /// If the tree is rooted, the parent must be closer to the root than the child.
    /// In any case, the edge will be added to both the parent and child, so two-way traversal
    /// is always possible.
    /// An edge can only be added between two nodes that are already part of the tree.
    /// There is no check to prevent adding the same edge multiple times, which will result in logical
    /// errors on traversal and serialization.
    pub fn add_edge(&mut self, parent: NodeId, child: NodeId, support: Option<f64>, branch_length: Option<f64>) {
        self.nodes[parent]
            .edges
            .push(DirectedEdge::new(child, support, branch_length));
        self.nodes[child]
            .edges
            .push(DirectedEdge::new(parent, support, branch_length));
    }

    /// Remove an edge between two nodes in the tree.
    /// The assignment of parent and child is arbitrary.
    ///
    /// If the edge does not exist, the operation will return either [ParentNoChild], or
    /// [ChildNoParent], depending on which condition is found first.
    /// If the edge exists, but has different metadata between the two directions, a [DiscordantEdgeData]
    /// error is returned.
    ///
    /// If the edge was removed successfully, the branch length and branch support are returned,
    /// if they existed.
    pub fn remove_edge(&mut self, parent: NodeId, child: NodeId) -> Result<(Option<f64>, Option<f64>), TreeError> {
        let branch_len;
        let branch_support;

        if let Some(child_edge) = self.nodes[parent].edges.iter().position(|e| e.target == child) {
            branch_len = self.nodes[parent].edges[child_edge].branch_length;
            branch_support = self.nodes[parent].edges[child_edge].support;

            if let Some(parent_edge) = self.nodes[child].edges.iter().position(|e| e.target == parent) {
                if branch_len != self.nodes[child].edges[parent_edge].branch_length {
                    return Err(DiscordantEdgeData);
                }

                if branch_support != self.nodes[child].edges[parent_edge].branch_length {
                    return Err(DiscordantEdgeData);
                }

                self.nodes[child].edges.swap_remove(parent_edge);
            } else {
                return Err(ChildNoParent);
            }

            self.nodes[parent].edges.swap_remove(child_edge);
        } else {
            return Err(ParentNoChild);
        }

        Ok((branch_len, branch_support))
    }

    /// Returns an iterator over the nodes in the tree in the specified traversal order.
    /// The order is a list of node IDs in the order they are to be traversed.
    /// The order is not guaranteed to visit each node, or to visit a node just once.
    pub fn traverse(&self, order: &TraversalOrder) -> impl Iterator<Item = &TreeNode> {
        order.iter().map(|&id| &self.nodes[id])
    }

    /// Returns the virtual root of the tree.
    /// Returns `None` if the tree has no virtual root.
    pub fn virtual_root(&self) -> Option<NodeId> {
        self.virtual_root.as_ref().map(|e| e.target)
    }

    /// Generate a postorder traversal order of the tree starting from the specified root node.
    /// The order is a list of node IDs in the order they are to be traversed.
    ///
    /// This function's return value is intended to be used with the [`traverse`] method.
    /// Note, that due to the nature of mutability in Rust, a mutable version of [`traverse`] cannot be
    /// provided. You can manually implement it using the [`node_mut`] method.
    ///
    /// [`traverse`]: NTree::traverse
    /// [`node_mut`]: NTree::node_mut
    pub fn postorder(&self, root: NodeId) -> Vec<NodeId> {
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

    /// Returns an iterator over the edges in the tree in postorder traversal order starting from the
    /// specified root node. The iterator yields tuples of the form (parent_id, edge).
    /// The edge returned in each iteration is the directed edge from the parent node to the child node,
    /// the child being the node that would be traversed in a normal postorder traversal.
    ///
    /// Note, that this function returns an iterator, unlike the [`postorder`] method, which returns a
    /// [`TraversalOrder`].
    ///
    /// [`postorder`]: NTree::postorder
    /// [`TraversalOrder`]: TraversalOrder
    pub fn edge_postorder(&self, root: NodeId) -> impl Iterator<Item = (NodeId, DirectedEdge)> {
        let mut stack = Vec::with_capacity(self.node_count() << 1);
        stack.push((
            (root, root, self.get_tree_support(), self.get_tree_branch_length()),
            self.get_children(root, root),
        ));
        iter::from_fn(move || {
            loop {
                if let Some(((parent_id, node_id, support, branch_length), mut children)) = stack.pop() {
                    if let Some((child_id, child_support, child_branch_length)) = children.next() {
                        stack.push(((parent_id, node_id, support, branch_length), children));
                        stack.push((
                            (node_id, *child_id, child_support.clone(), child_branch_length.clone()),
                            self.get_children(node_id, *child_id),
                        ));
                    } else {
                        return Some((parent_id, DirectedEdge::new(node_id, support, branch_length)));
                    }
                } else {
                    return None;
                }
            }
        })
    }

    /// Generate a preorder traversal order of the tree starting from the specified root node.
    /// The order is a list of node IDs in the order they are to be traversed.
    ///
    /// This function's return value is intended to be used with the [`traverse`] method.
    /// Note, that due to the nature of mutability in Rust, a mutable version of [`traverse`] cannot be
    /// provided. You can manually implement it using the [`node_mut`] method.
    ///
    /// [`traverse`]: NTree::traverse
    /// [`node_mut`]: NTree::node_mut
    pub fn preorder(&self, root: NodeId) -> Vec<NodeId> {
        let mut order = Vec::with_capacity(self.node_count());
        let mut stack = Vec::with_capacity(self.node_count() << 1);
        stack.push((root, root));

        while let Some((parent, node)) = stack.pop() {
            order.push(node);
            let start = stack.len();

            for edge in self.nodes[node].edges() {
                if edge.target() != parent {
                    stack.push((node, edge.target()));
                }
            }

            // reverse the order sibling nodes are visited, so we visit the first sibling in the
            // array first
            let end = stack.len();
            stack[start..end].reverse();
        }

        order
    }

    /// Returns a traversal order of the nodes in the tree in an unspecified order,
    /// but guaranteed to visit each node exactly once (and cache-friendly).
    /// This is intended for use with the [`traverse`] method.
    pub fn any_order(&self) -> Vec<NodeId> {
        (0..self.node_count()).collect()
    }

    /// Returns the length of the tree, which is the number of nodes in the tree.
    /// This is equivalent to the `node_count()` method.
    pub fn len(&self) -> usize {
        self.node_count()
    }

    /// Returns true if the tree contains no nodes.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// A node in the tree.
#[derive(Clone, Debug)]
pub struct TreeNode {
    pub label: Option<String>,
    #[cfg(feature = "smallvec")]
    edges: smallvec::SmallVec<[DirectedEdge; 3]>,
    #[cfg(not(feature = "smallvec"))]
    edges: Vec<DirectedEdge>,
}

impl TreeNode {
    /// Creates a new `TreeNode` with the specified label and an empty list of edges.
    pub fn new(label: Option<String>) -> Self {
        TreeNode {
            label,
            #[cfg(feature = "smallvec")]
            edges: smallvec::SmallVec::new(),
            #[cfg(not(feature = "smallvec"))]
            edges: Vec::new(),
        }
    }

    /// Creates a new `TreeNode` with the specified label and a specified capacity for edges.
    pub fn with_capacity(label: Option<String>, capacity: usize) -> Self {
        TreeNode {
            label,
            #[cfg(feature = "smallvec")]
            edges: smallvec::SmallVec::with_capacity(capacity),
            #[cfg(not(feature = "smallvec"))]
            edges: Vec::with_capacity(capacity),
        }
    }

    /// Returns the edges of the node.
    pub fn edges(&self) -> &[DirectedEdge] {
        &self.edges
    }

    /// Returns true, if the node is a tip (leaf) node.
    pub fn is_tip(&self) -> bool {
        self.edges.len() == 1
    }
}

/// A directed edge in the tree.
/// The direction is arbitrary, as all trees are undirected and therefore each edge has a reverse
/// edge containing the same support and branch length values.
#[derive(Clone, Debug)]
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

/// A [`TreeBuilder`] implementation for creating [`NTree`]s for use in the [`Parser`].
///
/// [`TreeBuilder`]: crate::TreeBuilder
/// [`Parser`]: crate::parser::Parser
/// [`NTree`]: NTree
pub struct SimpleTreeBuilder {
    tree: NTree,
}

impl SimpleTreeBuilder {
    /// Creates a new `SimpleTreeBuilder` with no nodes.
    pub fn new() -> Self {
        SimpleTreeBuilder { tree: NTree::new() }
    }

    /// Creates a new `SimpleTreeBuilder` with the specified node capacity, ensuring no reallocation
    /// occurs when adding nodes up to that capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        SimpleTreeBuilder {
            tree: NTree::with_capacity(capacity),
        }
    }
}

impl TreeBuilder for SimpleTreeBuilder {
    type Tree = NTree;
    type NodeId = NodeId;

    fn build(&mut self) -> Self::Tree {
        let mut new_tree = NTree::new();
        mem::swap(&mut self.tree, &mut new_tree);
        new_tree
    }

    fn add_node(&mut self, label: Option<String>, edge_hint: usize) -> Self::NodeId {
        self.tree.add_node(label, edge_hint)
    }

    fn add_edge(
        &mut self,
        parent: Self::NodeId,
        child: Self::NodeId,
        support: Option<f64>,
        branch_length: Option<f64>,
    ) {
        self.tree.add_edge(parent, child, support, branch_length)
    }

    fn set_virtual_root(&mut self, node: Self::NodeId, support: Option<f64>, branch_length: Option<f64>) {
        self.tree.virtual_root = Some(DirectedEdge::new(node, support, branch_length));
    }
}

impl TreeSerialize for NTree {
    type NodeId = NodeId;

    fn get_virtual_root(&self) -> Option<Self::NodeId> {
        self.virtual_root()
    }

    fn get_tree_support(&self) -> Option<f64> {
        self.virtual_root.as_ref().and_then(|e| e.support)
    }

    fn get_tree_branch_length(&self) -> Option<f64> {
        self.virtual_root.as_ref().and_then(|e| e.branch_length)
    }

    fn get_children(
        &self,
        parent: Self::NodeId,
        node: Self::NodeId,
    ) -> impl Iterator<Item = (&Self::NodeId, Option<f64>, Option<f64>)> {
        self.nodes[node].edges.iter().filter_map(move |edge| {
            if edge.target == parent {
                return None;
            }
            Some((&edge.target, edge.support, edge.branch_length))
        })
    }

    fn get_label(&self, node: &Self::NodeId) -> Option<&String> {
        self.nodes[*node].label.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::Parser;

    #[test]
    fn test_tree_builder() {
        let mut builder = SimpleTreeBuilder::new();
        let node3 = builder.add_node(Some("C".to_string()), 0);
        let node2 = builder.add_node(Some("B".to_string()), 1);
        let node1 = builder.add_node(Some("A".to_string()), 1);

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

        assert_eq!(tree.postorder(2), vec![node3, node2, node1]);
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

        assert_eq!(tree.postorder(4), vec![0, 1, 2, 3, 4]);
    }

    #[test]
    fn test_edge_postorder() {
        let newick = "(A:0.5,(B:0.8,C:0.2)D:0.1)R;";
        let builder = SimpleTreeBuilder::new();
        let mut parser = Parser::new(newick.as_bytes(), builder);
        let result = parser.parse().expect("Parsing failed.");
        let tree = result.expect("Parser returned no tree.");

        let postorder_edges: Vec<_> = tree.edge_postorder(4).collect();
        assert_eq!(postorder_edges.len(), 5);
        assert_eq!(tree.node(postorder_edges[0].0).label, Some(String::from("R")));
        assert_eq!(tree.node(postorder_edges[0].1.target).label, Some(String::from("A")));
        assert_eq!(postorder_edges[0].1.branch_length, Some(0.5));
        assert_eq!(tree.node(postorder_edges[1].0).label, Some(String::from("D")));
        assert_eq!(tree.node(postorder_edges[1].1.target).label, Some(String::from("B")));
        assert_eq!(postorder_edges[1].1.branch_length, Some(0.8));
        assert_eq!(tree.node(postorder_edges[2].0).label, Some(String::from("D")));
        assert_eq!(tree.node(postorder_edges[2].1.target).label, Some(String::from("C")));
        assert_eq!(postorder_edges[2].1.branch_length, Some(0.2));
        assert_eq!(tree.node(postorder_edges[3].0).label, Some(String::from("R")));
        assert_eq!(tree.node(postorder_edges[3].1.target).label, Some(String::from("D")));
        assert_eq!(postorder_edges[3].1.branch_length, Some(0.1));
        assert_eq!(tree.node(postorder_edges[4].0).label, Some(String::from("R")));
        assert_eq!(tree.node(postorder_edges[4].1.target).label, Some(String::from("R")));
        assert_eq!(postorder_edges[4].1.branch_length, None);
    }

    #[test]
    fn test_preorder() {
        let newick = "(A,(B,C)D)R;";
        let builder = SimpleTreeBuilder::new();
        let mut parser = Parser::new(newick.as_bytes(), builder);
        let result = parser.parse().expect("Parsing failed.");
        let tree = result.expect("Parser returned no tree.");

        assert_eq!(tree.node_count(), 5);
        assert_eq!(tree.preorder(tree.virtual_root().unwrap()), vec![4, 0, 3, 1, 2]);
    }

    #[test]
    fn test_remove_edge() {
        let mut builder = SimpleTreeBuilder::new();
        let node_a = builder.add_node(Some("A".into()), 1);
        let node_c = builder.add_node(Some("C".into()), 1);
        let node_b = builder.add_node(Some("B".into()), 2);
        builder.add_edge(node_b, node_c, None, None);

        let node_root = builder.add_node(Some("root".into()), 2);
        builder.add_edge(node_root, node_a, None, None);
        builder.add_edge(node_root, node_b, None, None);

        let mut tree = builder.build();

        assert_eq!(tree.node(node_b).edges.len(), 2);
        assert_eq!(tree.node(node_a).edges.len(), 1);
        assert_eq!(tree.node(node_c).edges.len(), 1);

        assert!(tree.remove_edge(node_b, node_c).is_ok());

        assert_eq!(tree.node(node_b).edges.len(), 1);
        assert_eq!(tree.node(node_a).edges.len(), 1);
        assert_eq!(tree.node(node_c).edges.len(), 0);

        tree.add_edge(node_a, node_c, None, None);

        assert_eq!(tree.node(node_b).edges.len(), 1);
        assert_eq!(tree.node(node_a).edges.len(), 2);
        assert_eq!(tree.node(node_c).edges.len(), 1);
    }
}

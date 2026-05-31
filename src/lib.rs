#![warn(rustdoc::broken_intra_doc_links)]
#![warn(missing_docs)]

//! A minimal newick parser that constructs a simple adjacency-list-based [tree] structure.
//! It can construct arbitrary tree types by implementing a [builder trait] and handing it to
//! the [parser].
//!
//! # Minimal Tree
//! The provided tree data structure is minimal: It stores only information required for Newick,
//! and provides full support for tree modification.
//! No further data is stored keeping the tree as flexible as possible for downstream usage.
//! It connects edges with double edges to allow full traversal from any root, and simplify rerooting.
//!
//! If you expect your tree to be strictly bifurcating, you can enable the `smallvec` crate feature
//! to enable a [Small Vector Optimization](https://docs.rs/smallvec/latest/smallvec/).
//! This optimization does not break if the tree is not strictly bifurcating but no longer provides
//! a benefit on non-bifurcating nodes.
//!
//! # Branch Support
//! The parser automatically parses numerical node labels as branch support.
//! They are treated differently both in the builder trait and the [tree] data structure,
//! to facilitate mapping the support to edges.
//! This mitigates a common issue of Newick-based software, where rerooting or manipulating trees
//! with branch support values incorrectly assigns support values after the traversal order changes.
//!
//! # String Label Handling
//! Newick supports two methods of encoding string node labels.
//! If nothing in the label needs to be escaped, the label can be written to the Newick string as is.
//! If the label contains spaces, the label either needs to be surrounded by single-quotes,
//! or the spaces need to be replaced with underscores.
//! If the label contains Newick characters, it must be surrounded by single-quotes.
//! The library supports both modes, and an Option to enforce either mode.
//! In case of Newick characters in the label when the underscore mode is enforced, the serializer
//! will eagerly replace characters with underscores even if they aren't whitespace.
//!
//! The parser translates underscores of unquoted strings into whitespace to revert the encoding.
//! This can be disabled to handle incorrectly encoded files.
//!
//! [builder trait]: TreeBuilder
//! [parser]: parser::Parser
//! [tree]: tree::NTree

pub mod config;
pub mod parser;
pub mod serializer;
pub mod tokenizer;
pub mod tree;

/// A trait for building tree structures.
/// The trait is used by the [`Parser`] to create trees from newick data.
/// Implementations of the trait allow the parser to create different tree data structures.
pub trait TreeBuilder {
    /// The tree structure that will be built by the builder.
    type Tree;

    /// The node ID type used to identify nodes in the tree.
    type NodeId: Clone;

    /// Build an empty tree structure and reset the builder to its initial state.
    fn build(&mut self) -> Self::Tree;

    /// Add a node to the tree. It will not be connected to the tree yet.
    /// The node ID is returned, which can be used to uniquely identify the node in the tree.
    /// The node ID of a node must not change once the node has been added to the tree.
    ///
    /// The `edge_hint` parameter is used to provide a hint for the number of edges
    /// that will be added to the node during the parsing process.
    fn add_node(&mut self, label: Option<String>, edge_hint: usize) -> Self::NodeId;

    /// Add an edge between two existing nodes in the tree.
    /// The assignment of parent and child is arbitrary if the tree is unrooted.
    /// If the tree is rooted, the parent must be closer to the root than the child.
    /// An edge can only be added between two nodes that are already part of the tree.
    fn add_edge(&mut self, parent: Self::NodeId, child: Self::NodeId, support: Option<f64>, branch_length: Option<f64>);

    /// Set the virtual root edge of the tree.
    fn set_virtual_root(&mut self, node: Self::NodeId, support: Option<f64>, branch_length: Option<f64>);
}

/// A trait for building tree structures.
/// The trait is used by the Serializer to create newick data from tree structures.
/// Implementations of the trait allow the serializer to work with different tree data structures.
pub trait TreeSerialize {
    /// The type used for node identification. Must be trivially copyable. IDs must not change
    /// after the node has been added to the tree.
    type NodeId: Copy;

    /// Get the (virtual) root node of the tree.
    fn get_virtual_root(&self) -> Option<Self::NodeId>;

    /// Get the support value of the tree, which is stored as the name of the root node.
    /// If the implementing type does not support this, it should return None.
    fn get_tree_support(&self) -> Option<f64>;

    /// Newick does allow the root to have a branch length, even though this information is not
    /// associated with any edge in the tree.
    /// If the implementing type does not support this, it should return None.
    fn get_tree_branch_length(&self) -> Option<f64>;

    /// Get the children of a node in the tree, given the parent node. The iterator must not
    /// include an edge to the parent node.
    /// The iterator returns tuples of the form (child_node_id, support, branch_length).
    fn get_children(
        &self,
        parent: Self::NodeId,
        node: Self::NodeId,
    ) -> impl Iterator<Item = (&Self::NodeId, Option<f64>, Option<f64>)>;

    /// Get the label of a node in the tree.
    fn get_label(&self, node: &Self::NodeId) -> Option<&String>;
}

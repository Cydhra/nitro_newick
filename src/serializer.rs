use std::iter::Peekable;
use std::marker::PhantomData;
use crate::TreeSerialize;

/// A struct representing a node in the tree during serialization.
struct Node<'a, N: Clone, I: Iterator<Item = (&'a N, Option<f64>, Option<f64>)>> {
    id: &'a N,
    label: Option<&'a String>,
    support: Option<f64>,
    branch_length: Option<f64>,
    children: Peekable<I>,
}

/// A serializer for trees in Newick format.
/// This struct is generic over the tree type `T`, which must implement the `TreeSerialize` trait.
/// It is used to query the tree structure during serialization.
pub struct Serializer<T: TreeSerialize> { tree_type: PhantomData<T> }

impl<T: TreeSerialize> Serializer<T> {
    /// Creates a new instance of the `Serializer`.
    pub fn new() -> Self {
        Serializer { tree_type: PhantomData }
    }

    /// Helper function to push node data into the result string.
    fn push_node_data(result: &mut String, label: Option<&String>, support: Option<f64>, branch_length: Option<f64>) {
        if let Some(label) = label {
            result.push_str(&format!("{}", label));
        } else if let Some(support) = support {
            result.push_str(&format!("{}", support));
        }

        if let Some(branch_length) = branch_length {
            result.push_str(&format!(":{}", branch_length));
        }
    }

    /// Serializes the tree into a newick format string.
    pub fn serialize(&self, tree: &T) -> String {
        let root = tree.get_virtual_root();
        if root.is_none() {
            return String::from(';');
        }

        let mut result = String::new();
        let mut stack = Vec::new();
        let mut children = tree.get_children(root.as_ref().unwrap(), root.as_ref().unwrap()).peekable();

        if children.peek().is_none() {
            Self::push_node_data(&mut result, tree.get_label(root.as_ref().unwrap()), None, None);
            result.push(';');
            return result;
        } else {
            result.push('(');
            stack.push(Node {
                id: root.as_ref().unwrap(),
                label: tree.get_label(root.as_ref().unwrap()),
                support: None,
                branch_length: None,
                children,
            });
        }

        loop {
            let node = stack.last_mut().unwrap();
            if let Some((child_id, support, branch_length)) = node.children.next() {
                let mut children = tree.get_children(node.id, child_id).peekable();
                if children.peek().is_some() {
                    result.push('(');
                    stack.push(Node {
                        id: child_id,
                        label: tree.get_label(child_id),
                        support,
                        branch_length,
                        children,
                    });

                    // skip adding a comma, and descend into the child
                    continue;
                } else {
                    Self::push_node_data(&mut result, tree.get_label(child_id), support, branch_length);
                }
            } else {
                let node = stack.pop().unwrap();
                result.push(')');
                Self::push_node_data(&mut result, node.label, node.support, node.branch_length);
            }

            if let Some(parent) = stack.last_mut() {
                if parent.children.peek().is_some() {
                    result.push(',');
                }
            } else {
                break;
            }
        }

        result.push(';');
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tree::{SimpleTreeBuilder, UnrootedTree};
    use crate::parser::Parser;

    #[test]
    fn test_serialize() {
        let newick = "(A:0.1,B:0.2,(C:0.3,D:0.4):0.5);";
        let mut parser = Parser::new(newick.as_bytes(), SimpleTreeBuilder::new());
        let tree = parser.parse().unwrap().expect("Parse Error");
        let serializer = Serializer::<UnrootedTree>::new();
        let serialized = serializer.serialize(&tree);
        assert_eq!(serialized, newick);
    }

    #[test]
    fn test_serialize_anonymous() {
        let newick = "(:0.1,:0.2,(,D:0.4)F);";
        let mut parser = Parser::new(newick.as_bytes(), SimpleTreeBuilder::new());
        let tree = parser.parse().unwrap().expect("Parse Error");
        let serializer = Serializer::<UnrootedTree>::new();
        let serialized = serializer.serialize(&tree);
        assert_eq!(serialized, newick);
    }
}
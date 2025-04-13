pub struct UnrootedTree {

}

pub struct TreeNode {
    label: NodeLabel,
    children: Vec<TreeNode>,
}

pub enum NodeLabel {
    None,
    Name(String),
    Support(f64),
}

pub struct SimpleTreeBuilder {
    tree: TreeNode,
}
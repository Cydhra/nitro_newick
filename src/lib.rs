mod tokenizer;

pub mod tree;
mod parser;

pub trait TreeBuilder {
    type Tree;
    
    /// Build an empty tree structure
    fn build_empty_tree(&mut self) -> Self::Tree;
}

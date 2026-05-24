use crate::TreeBuilder;
use crate::config::Settings;
use crate::tokenizer::Token::*;
use crate::tokenizer::{Token, Tokenizer, TokenizerError};
use snafu::prelude::*;
use std::io::Read;

/// Error type for the parser
#[derive(Debug, Snafu)]
pub enum ParseError {
    /// Error while reading from the input stream
    #[snafu(display("Error while reading from input stream: {source}"))]
    InputError {
        /// Underlying tokenizer error. This may contain more underlying errors.
        source: TokenizerError,
    },

    /// Unexpected token in the input stream
    #[snafu(display("Unexpected token: expected {} but found {:?}: {}", expected.iter().map(|e| e.to_string()).collect::<Vec<_>>().join(", "), found, reason))]
    UnexpectedToken {
        /// List of tokens that were expected.
        expected: Vec<Token>,

        /// The token that was found instead.
        found: Token,

        /// Additional, human-readable reason behind the parse error which may contain contextual information.
        reason: String,
    },
}

/// Newick parser that transforms a token stream into one or more trees.
/// Each call to [`parse`] yields one tree, until an [`End`] token is encountered.
/// There is no recovery built into the Parser, meaning, if an error is encountered,
/// subsequent calls to the parser attempt to parse a new tree directly behind the erroneous token.
///
/// The parser constructs the tree by calling the provided implementation of [`TreeBuilder`].
/// A [`SimpleTreeBuilder`] is available to parse the tree into a minimal data structure that can represent all data from the Newick file.
/// Alternative implementations can be used to construct other tree data types.
///
/// [`parse`]: Parser::parse
/// [`End`]: End
/// [`TreeBuilder`]: TreeBuilder
/// [`SimpleTreeBuilder`]: crate::tree::SimpleTreeBuilder
pub struct Parser<R: Read, B: TreeBuilder> {
    tokenizer: Tokenizer<R>,
    builder: B,

    /// Flag to indicate if the parser has finished parsing the current tree.
    /// This is used to determine if the parser is expecting a semicolon or an end token,
    /// or if such a token should be rejected.
    tree_finished: bool,
    /// Flag to indicate if the parser is expecting a sibling node before the next closing
    /// parenthesis. Required to handle cases where a node is defined without a name, branch length or
    /// support value, but a prior comma has already been consumed.
    expect_sibling: bool,
}

impl<R: Read, B: TreeBuilder> Parser<R, B> {
    /// Create a new parser instance from the given newick input stream and a tree builder instance.
    pub fn new(reader: R, builder: B) -> Self {
        Self::with_settings(reader, builder, Settings::default())
    }

    /// Create a new parser instance from the given newick input stream, a tree builder instance, and custom [`Settings`].
    pub fn with_settings(reader: R, builder: B, settings: Settings) -> Self {
        Parser {
            tokenizer: Tokenizer::with_settings(reader, settings),
            builder,
            tree_finished: true,
            expect_sibling: false,
        }
    }

    /// Parse the input stream and build a tree structure.
    /// Consumes the input stream until a semicolon or end token is encountered,
    /// and builds a tree structure from the tokens.
    /// If more non-end tokens are encountered after a semicolon,
    /// the parser can be called again to parse the next tree.
    pub fn parse(&mut self) -> Result<Option<B::Tree>, ParseError> {
        let mut stack = vec![];

        loop {
            let token = self.tokenizer.next_token().context(InputSnafu {})?;

            // mark tree as unfinished if we encounter a token that is not a semicolon or end
            if !matches!(token, Semicolon | End) {
                self.tree_finished = false;
            }

            match token {
                OpenParen => {
                    // push a new node to the stack
                    stack.push(vec![]);

                    // an open parenthesis means we expect at least one child node
                    self.expect_sibling = true;
                }
                CloseParen => {
                    if stack.is_empty() {
                        return Err(ParseError::UnexpectedToken {
                            expected: vec![OpenParen, Semicolon],
                            found: token,
                            reason: "No opening parenthesis found prior".to_string(),
                        });
                    }

                    // if we still expect a sibling, it means we have a node without a name, branch length or
                    // support value, but a prior comma has already been consumed
                    if self.expect_sibling {
                        let anonymous_child = self.builder.add_node(None, 1);
                        stack.last_mut().unwrap().push((anonymous_child, None, None));
                        self.expect_sibling = false;
                    }

                    let has_info = matches!(self.tokenizer.peek(), Ok(Colon) | Ok(Name(_)) | Ok(Float(_)));
                    let mut node_label = None;
                    let mut node_support = None;
                    let mut node_branch_length = None;

                    // check if the next token is a colon, name, or float, and if so, parse the node
                    // label, support, and branch length
                    if has_info {
                        let (label, support, branch_length) = self.consume_named_node_info()?;
                        node_label = label;
                        node_support = support;
                        node_branch_length = branch_length;
                    } else if stack.len() > 1 {
                        // consume trailing comma if present and this isn't the root node
                        self.consume_trailing_comma()?;
                    }

                    // pop children from the stack and append to the current node
                    let children = stack.pop().unwrap();
                    let node_id = self.builder.add_node(node_label, children.len() + 1);
                    for (child, branch_support, branch_length) in children {
                        self.builder
                            .add_edge(node_id.clone(), child, branch_support, branch_length);
                    }

                    // push current edge to the parent children
                    if let Some(children) = stack.last_mut() {
                        children.push((node_id, node_support, node_branch_length));
                    } else {
                        // if there are no children, we are at the root node
                        self.builder.set_virtual_root(node_id, node_support, node_branch_length);
                    }
                }
                Name(name) => {
                    // if we encounter a name, it means there is a named leaf node, because otherwise
                    // we would have encountered a close parenthesis first, and consumed the
                    // name
                    let branch_length = self.consume_branch_length()?;

                    // push leaf node to the parent children
                    if let Some(children) = stack.last_mut() {
                        let node_id = self.builder.add_node(Some(name), 1);
                        children.push((node_id, None, branch_length));
                    } else {
                        // if there are no children, we are at the root node
                        let node_id = self.builder.add_node(Some(name), 0);
                        self.builder.set_virtual_root(node_id, None, branch_length);
                    }
                }
                Float(support) => {
                    // if we encounter a float, it means there is a leaf node with support, because otherwise
                    // we would have encountered a close parenthesis first, and consumed the
                    // support
                    let branch_length = self.consume_branch_length()?;

                    // push leaf node to the parent children
                    if let Some(children) = stack.last_mut() {
                        let node_id = self.builder.add_node(None, 1);
                        children.push((node_id, Some(support), branch_length));
                    } else {
                        return Err(ParseError::UnexpectedToken {
                            expected: vec![OpenParen, Semicolon],
                            found: Float(support),
                            reason: "No opening parenthesis found prior".to_string(),
                        });
                    }
                }
                Colon => {
                    // if we encounter a colon, it means there is a nameless leaf node, because otherwise
                    // we would have encountered the name first, and consumed the branch length

                    let branch_length_token = self.tokenizer.next_token().context(InputSnafu {})?;
                    let branch_length = if let Float(branch_length) = branch_length_token {
                        branch_length
                    } else {
                        return Err(ParseError::UnexpectedToken {
                            expected: vec![Float(0.0)],
                            found: branch_length_token,
                            reason: "Expected a branch length after colon".to_string(),
                        });
                    };

                    self.consume_trailing_comma()?;

                    if let Some(children) = stack.last_mut() {
                        let node_id = self.builder.add_node(None, 1);
                        children.push((node_id, None, Some(branch_length)));
                    } else {
                        return Err(ParseError::UnexpectedToken {
                            expected: vec![OpenParen, Semicolon],
                            found: Float(branch_length),
                            reason: "No opening parenthesis found prior".to_string(),
                        });
                    }
                }
                Comma => {
                    // if we encounter a comma, it means there is an unnamed leaf node, because otherwise
                    // we would have encountered a close parenthesis first, and consumed the
                    // comma

                    // add a leaf node
                    let node_id = self.builder.add_node(None, 1);

                    // push current edge to the parent children
                    if let Some(children) = stack.last_mut() {
                        // there has to be at least one more node, but expect_sibling must be already
                        // set to true at this point
                        debug_assert!(self.expect_sibling);

                        children.push((node_id, None, None));
                    } else {
                        return Err(ParseError::UnexpectedToken {
                            expected: vec![OpenParen, Semicolon],
                            found: token,
                            reason: "No opening parenthesis found prior".to_string(),
                        });
                    }
                }
                Semicolon => {
                    if !stack.is_empty() {
                        return Err(ParseError::UnexpectedToken {
                            expected: vec![CloseParen],
                            found: token,
                            reason: "There are unclosed parentheses".to_string(),
                        });
                    }

                    // an end token is now legal
                    self.tree_finished = true;
                    return Ok(Some(self.builder.build()));
                }
                End => {
                    if !stack.is_empty() {
                        return Err(ParseError::UnexpectedToken {
                            expected: vec![CloseParen],
                            found: token,
                            reason: "There are unclosed parentheses".to_string(),
                        });
                    } else if !self.tree_finished {
                        return Err(ParseError::UnexpectedToken {
                            expected: vec![Semicolon],
                            found: token,
                            reason: "Tree is not finished, missing semicolon".to_string(),
                        });
                    }
                    return Ok(None);
                }
            }
        }
    }

    /// Consume a node label or support value if present, a branch length if present, and a trailing comma if present.
    /// If a comma is consumed, the parser expects a sibling node next, ensuring that a following
    /// closing parenthesis implicitly adds an anonymous node.
    #[inline]
    fn consume_named_node_info(&mut self) -> Result<(Option<String>, Option<f64>, Option<f64>), ParseError> {
        let mut node_label = None;
        let mut node_support = None;

        // parse node label or support
        let token = self.tokenizer.peek().context(InputSnafu {})?;
        if let Name(_) = token {
            let Name(label) = self.tokenizer.next_token().context(InputSnafu {})? else {
                unreachable!()
            };
            node_label = Some(label);
        } else if let Float(_) = token {
            let Float(support) = self.tokenizer.next_token().context(InputSnafu {})? else {
                unreachable!()
            };
            node_support = Some(support);
        }

        let node_branch_length = self.consume_branch_length()?;

        Ok((node_label, node_support, node_branch_length))
    }

    /// Consume a branch length if present.
    /// Regardless of whether a branch length is present or not, the parser calls
    /// [`consume_trailing_comma`] afterward.
    #[inline]
    fn consume_branch_length(&mut self) -> Result<Option<f64>, ParseError> {
        let mut node_branch_length = None;

        if matches!(self.tokenizer.peek(), Ok(Colon)) {
            // Ignore colon
            self.tokenizer.next_token().context(InputSnafu {})?;

            let branch_length_token = self.tokenizer.next_token().context(InputSnafu {})?;
            if let Float(branch_length) = branch_length_token {
                node_branch_length = Some(branch_length);
            } else {
                return Err(ParseError::UnexpectedToken {
                    expected: vec![Float(0.0)],
                    found: branch_length_token,
                    reason: "Expected a branch length after colon".to_string(),
                });
            }
        }

        // read away a trailing comma if present
        self.consume_trailing_comma()?;

        Ok(node_branch_length)
    }

    /// Consume a trailing comma if present.
    /// If a comma is consumed, the parser expects a sibling node next, ensuring that a following
    /// closing parenthesis implicitly adds an anonymous node.
    #[inline]
    fn consume_trailing_comma(&mut self) -> Result<(), ParseError> {
        let token = self.tokenizer.peek().context(InputSnafu {})?;
        if matches!(token, Comma) {
            self.tokenizer.next_token().context(InputSnafu {})?;
            self.expect_sibling = true;
            Ok(())
        } else if matches!(token, CloseParen | Semicolon) {
            self.expect_sibling = false;
            Ok(())
        } else {
            Err(ParseError::UnexpectedToken {
                expected: vec![Comma, CloseParen, Semicolon],
                found: token.clone(),
                reason: "Expected a comma, closing parenthesis, or semicolon after a node definition".to_string(),
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;
    use std::fs::File;
    use std::path::PathBuf;

    struct MockTreeBuilder;

    impl TreeBuilder for MockTreeBuilder {
        type Tree = ();
        type NodeId = ();

        fn build(&mut self) -> Self::Tree {}

        fn add_node(&mut self, _label: Option<String>, _edge_hint: usize) -> Self::NodeId {}

        fn add_edge(
            &mut self,
            _parent: Self::NodeId,
            _child: Self::NodeId,
            _support: Option<f64>,
            _branch_length: Option<f64>,
        ) {
        }

        fn set_virtual_root(&mut self, _node: Self::NodeId, _support: Option<f64>, _branch_length: Option<f64>) {}
    }

    struct OutputTreeBuilder {
        tree: String,
    }

    impl TreeBuilder for OutputTreeBuilder {
        type Tree = String;
        type NodeId = ();

        fn build(&mut self) -> Self::Tree {
            let mut next_tree = String::new();
            std::mem::swap(&mut self.tree, &mut next_tree);
            next_tree
        }

        fn add_node(&mut self, label: Option<String>, _edge_hint: usize) -> Self::NodeId {
            self.tree
                .push_str(&label.unwrap_or_else(|| String::from("<anonymous>")));
        }

        fn add_edge(
            &mut self,
            _parent: Self::NodeId,
            _child: Self::NodeId,
            _support: Option<f64>,
            _branch_length: Option<f64>,
        ) {
        }

        fn set_virtual_root(&mut self, _node: Self::NodeId, _support: Option<f64>, _branch_length: Option<f64>) {}
    }

    #[rstest]
    fn expect_working(#[files("tests/resources/parser/accept/*.nw")] path: PathBuf) {
        // output the file name for easy identification in log files
        println!("Testing file: {:?}", path.file_name().unwrap());

        let stream = File::open(path).expect("Could not open file");
        let builder = MockTreeBuilder {};
        let mut parser = Parser::new(stream, builder);

        // read all trees from the file
        loop {
            let res = parser.parse().expect("Failed to parse file");
            if res.is_none() {
                break;
            }
        }
    }

    #[rstest]
    fn reject_failing(#[files("tests/resources/parser/reject/*.nw")] path: PathBuf) {
        // output the file name for easy identification in log files
        println!("Testing file: {:?}", path.file_name().unwrap());

        let stream = File::open(&path).expect("Could not open file");
        let builder = MockTreeBuilder {};
        let mut parser = Parser::new(stream, builder);

        assert!(parser.parse().is_err(), "Expected parse to fail for file: {:?}", path);
    }

    #[rstest]
    fn verify_postorder(#[files("tests/resources/parser/postorder/*.nw")] path: PathBuf) {
        // output the file name for easy identification in log files
        println!("Testing file: {:?}", path.file_name().unwrap());

        let mut expected_output = path.clone();
        expected_output.set_extension("out");

        let stream = File::open(&path).expect("Could not open file");
        let builder = OutputTreeBuilder { tree: String::new() };
        let mut parser = Parser::new(stream, builder);

        let mut expected_stream = File::open(expected_output).expect("Could not open expected output file");
        let mut expected = String::new();
        expected_stream
            .read_to_string(&mut expected)
            .expect("Could not read expected output file");

        assert_eq!(parser.parse().expect("Failed to parse file"), Some(expected));
    }
}

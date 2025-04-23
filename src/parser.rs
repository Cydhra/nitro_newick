use std::io::Read;
use snafu::prelude::*;
use crate::tokenizer::{Token, Tokenizer, TokenizerError};
use crate::tokenizer::Token::*;
use crate::TreeBuilder;

/// Error type for the parser
#[derive(Debug, Snafu)]
pub enum ParseError {
    /// Error while reading from the input stream
    #[snafu(display("Error while reading from input stream: {source}"))]
    InputError { source: TokenizerError },

    /// Unexpected token in the input stream
     UnexpectedToken {
        expected: Vec<Token>,
        found: Token
    },
}

pub struct Parser<R: Read, B: TreeBuilder> {
    tokenizer: Tokenizer<R>,
    builder: B,
}

impl<R: Read, B: TreeBuilder> Parser<R, B> {
    pub fn new(reader: R, builder: B) -> Self {
        Parser {
            tokenizer: Tokenizer::new(reader),
            builder,
        }
    }

    pub fn parse(&mut self) -> Result<Option<B::Tree>, ParseError> {
        let mut stack = vec![];
        loop {
            let token = self.tokenizer.next_token().context(InputSnafu {})?;
            match token {
                OpenParen => {
                    // push a new node to the stack
                    stack.push(vec![]);
                }
                CloseParen => {
                    if stack.is_empty() {
                        return Err(ParseError::UnexpectedToken {
                            expected: vec![OpenParen, Semicolon],
                            found: token,
                        });
                    }

                    let has_info = matches!(self.tokenizer.peek(), Ok(Colon) | Ok(Name(_)) | Ok(Float(_)));
                    let mut node_label = None;
                    let mut node_support = None;
                    let mut node_branch_length = None;

                    // check if the next token is a colon, name, or float, and if so, parse the node
                    // label, support, and branch length
                    if has_info {
                        // parse node label, support, and branch length
                        let mut token = self.tokenizer.next_token().context(InputSnafu {})?;
                        if let Name(label) = token {
                            node_label = Some(label);
                            token = self.tokenizer.next_token().context(InputSnafu {})?;
                        } else if let Float(support) = token {
                            node_support = Some(support);
                            token = self.tokenizer.next_token().context(InputSnafu {})?;
                        }

                        if let Colon = token {
                            // Ignore colon
                            let branch_length_token = self.tokenizer.next_token().context(InputSnafu {})?;
                            if let Float(branch_length) = branch_length_token {
                                node_branch_length = Some(branch_length);
                            } else {
                                return Err(ParseError::UnexpectedToken {
                                    expected: vec![Float(0.0)],
                                    found: branch_length_token,
                                });
                            }
                        }
                    }

                    // pop children from the stack and append to the current node
                    let children = stack.pop().unwrap();
                    let node_id = self.builder.add_node(node_label);
                    for (child, branch_support, branch_length) in children {
                        self.builder.add_edge(node_id.clone(), child, branch_support, branch_length);
                    }

                    // push current edge to the parent children
                    if let Some(children) = stack.last_mut() {
                        children.push((node_id, node_support, node_branch_length));
                    }

                    // read away a trailing comma if present
                    let token = self.tokenizer.peek().context(InputSnafu {})?;
                    if matches!(token, Comma) {
                        self.tokenizer.next_token().context(InputSnafu {})?;
                    }
                }
                Comma => {
                    // if we encounter a comma, it means there is a leaf node, because otherwise
                    // we would have encountered a close parenthesis first, and consumed the
                    // comma

                    // add a leaf node
                    let node_id = self.builder.add_node(None);

                    // push current edge to the parent children
                    if let Some(children) = stack.last_mut() {
                        children.push((node_id, None, None));
                    } else {
                        return Err(ParseError::UnexpectedToken {
                            expected: vec![OpenParen, Semicolon],
                            found: token,
                        });
                    }
                }
                Semicolon => { return Ok(Some(self.builder.build())); }
                End => { return Ok(None) }
                _ => {
                    return Err(ParseError::UnexpectedToken {
                        expected: vec![OpenParen, CloseParen, Semicolon, Comma],
                        found: token,
                    });
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs::File;
    use std::path::PathBuf;
    use super::*;
    use rstest::rstest;

    struct MockTreeBuilder;

    impl TreeBuilder for MockTreeBuilder {
        type Tree = ();
        type NodeId = ();

        fn build(&mut self) -> Self::Tree {}

        fn add_node(&mut self, _label: Option<String>) -> Self::NodeId {}

        fn add_edge(&mut self, _parent: Self::NodeId, _child: Self::NodeId, _support: Option<f64>, _branch_length: Option<f64>) {}
    }

    #[rstest]
    fn expect_working(#[files("tests/resources/parser/accept/*.nw")] path: PathBuf) {
        // output the file name for easy identification in log files
        println!("Testing file: {:?}", path.file_name().unwrap());

        let stream = File::open(path).expect("Could not open file");
        let builder = MockTreeBuilder {};
        let mut parser = Parser::new(stream, builder);

        parser.parse().expect("Failed to parse file");
    }
}
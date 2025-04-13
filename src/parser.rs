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
    #[snafu(display("Unexpected token: found {found} but expected {expected}"))]
    UnexpectedToken {
        expected: Token,
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

    pub fn parse(&mut self) -> Result<B::Tree, ParseError> {
        self.parse_tree()
    }

    fn parse_tree(&mut self) -> Result<B::Tree, ParseError> {
        let token = self.tokenizer.next_token().context(InputSnafu {})?;
        
        if matches!(token, Semicolon) {
            Ok(self.builder.build_empty_tree())
        } else if matches!(token, OpenParen) {
            // parse loop
            Ok(self.builder.build_empty_tree())
        } else {
            Err(ParseError::UnexpectedToken {
                expected: OpenParen,
                found: token,
            })
        }
    }
}
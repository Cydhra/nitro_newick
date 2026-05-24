use crate::config::Settings;
use snafu::{ResultExt, Snafu, ensure};
use std::borrow::Cow;
use std::fmt::Display;
use std::io::Read;

const BUFFER_SIZE: usize = 16 * 1024;

/// Error type for the tokenizer.
#[derive(Debug, Snafu)]
pub enum TokenizerError {
    /// Error while reading from the input reader
    #[snafu(display("Could not read input stream"))]
    InputError {
        /// IO Error which produced this error in the tokenizer.
        source: std::io::Error,
    },

    /// Error while parsing a float
    #[snafu(display("Invalid float value"))]
    FloatError {
        /// Parser error in the [`FromStr`] implementation of `f64` which caused this error in the
        /// tokenizer.
        ///
        /// [`FromStr`]: std::str::FromStr
        source: std::num::ParseFloatError,
    },

    /// General error while trying to read structured input (but not a float),
    /// not caused by an underlying component.
    #[snafu(display("Cannot parse input: {reason}"))]
    ParseError {
        /// Human-readable reason for the tokenizer error.
        reason: String,
    },
}

/// Newick token types, produced by the [`Tokenizer`]
///
/// [`Tokenizer`]: Tokenizer
#[derive(Clone, Debug, PartialEq)]
pub enum Token {
    /// Floating point number.
    /// Not the full range of possible floating point serializations is supported;
    /// only floats built from digits (`0-9`), minus (`-`), and dots (`.`) can be parsed.
    Float(f64),

    /// Any string, already decoded according to the parser settings (i.e., the string in this
    /// variant is already the correct string with quotation marks removed and underscores replaced).
    Name(String),

    /// A comma (not part of a quoted string).
    Comma,

    /// An opening parenthesis (not part of a quoted string).
    OpenParen,

    /// A closing parenthesis (not part of a quoted string).
    CloseParen,

    /// A colon (not part of a quoted string).
    Colon,

    /// A semicolon (not part of a quoted string).
    Semicolon,

    /// End of the input stream.
    End,
}

impl Display for Token {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

/// Reads a [readable] stream and converts it into Newick tokens.
/// Tokens are read from the instance with [`next_token`].
///
/// To generate the tokens, the `Tokenizer` calls an underlying parser for floating point numbers.
/// Newick strings are also already decoded; the generated tokens already contain clear-text strings
/// with the Newick encoding reversed.
///
/// ```
/// use nitro_newick::tokenizer::{Tokenizer, Token};
/// use nitro_newick::tokenizer::Token::*;
///
/// let newick = "(A, B);";
/// let mut tokenizer = Tokenizer::new(newick.as_bytes());
/// let tokens = vec![
///     tokenizer.next_token().unwrap(), tokenizer.next_token().unwrap(), tokenizer.next_token().unwrap(),
///     tokenizer.next_token().unwrap(), tokenizer.next_token().unwrap(), tokenizer.next_token().unwrap(),
///     tokenizer.next_token().unwrap(),
/// ];
///
/// assert_eq!(tokens, vec![OpenParen, Name("A".into()), Comma, Name("B".into()), CloseParen, Semicolon, End]);
/// ```
///
/// [readable]: Read
/// [`next_token`]: Tokenizer::next_token
#[derive(Clone, Debug)]
pub struct Tokenizer<R: Read> {
    settings: Settings,
    reader: R,
    buffer: Box<[u8; BUFFER_SIZE]>,
    position: usize,
    length: usize,
    lookahead: Option<Token>,
}

impl<R: Read> Tokenizer<R> {
    /// Create a tokenizer from a [readable] source with default settings.
    ///
    /// # Example
    /// ```
    /// # use nitro_newick::tokenizer::{Tokenizer, Token};
    ///
    /// let newick = "(((A1,A2)10,(B1,B2)20)30,(C1,C2)30,(D1,D2)40);";
    /// let mut tokenizer = Tokenizer::new(newick.as_bytes());
    ///
    /// assert_eq!(tokenizer.next_token().unwrap(), Token::OpenParen);
    /// ```
    ///
    /// [readable]: Read
    pub fn new(reader: R) -> Self {
        Self::with_settings(reader, Settings::default())
    }

    /// Create a tokenizer from a [readable] source with custom [`Settings`].
    ///
    /// # Example
    /// ```
    /// # use nitro_newick::config::Settings;
    /// # use nitro_newick::tokenizer::{Tokenizer, Token};
    ///
    /// let newick = "A_B_C";
    /// let mut tokenizer =
    ///     Tokenizer::with_settings(newick.as_bytes(), Settings::default().translate_underscores(false));
    ///
    /// assert_eq!(tokenizer.next_token().unwrap(), Token::Name("A_B_C".to_owned()));
    /// ```
    ///
    /// [readable]: Read
    /// [`Settings`]: Settings
    pub fn with_settings(reader: R, settings: Settings) -> Self {
        Tokenizer {
            reader,
            settings,
            buffer: Box::new([0; BUFFER_SIZE]),
            position: 0,
            length: 0,
            lookahead: None,
        }
    }

    /// Get the next token in the token stream without consuming it.
    ///
    /// # Example
    /// ```
    /// # use nitro_newick::config::Settings;
    /// # use nitro_newick::tokenizer::{Tokenizer, Token};
    ///
    /// let newick = "A_B_C";
    /// let mut tokenizer = Tokenizer::new(newick.as_bytes());
    ///
    /// assert_eq!(*tokenizer.peek().unwrap(), Token::Name("A B C".to_owned()));
    /// assert_eq!(*tokenizer.peek().unwrap(), Token::Name("A B C".to_owned()));
    /// ```
    pub fn peek(&mut self) -> Result<&Token, TokenizerError> {
        if !(self.lookahead.is_some()) {
            let token = self.next_token()?;
            self.lookahead = Some(token);
        }

        Ok(self.lookahead.as_ref().unwrap())
    }

    /// Get the next token in the token stream and advance the stream beyond it.
    ///
    /// # Example
    /// ```
    /// # use nitro_newick::config::Settings;
    /// # use nitro_newick::tokenizer::{Tokenizer, Token};
    ///
    /// let newick = "A_B_C";
    /// let mut tokenizer = Tokenizer::new(newick.as_bytes());
    ///
    /// assert_eq!(tokenizer.next_token().unwrap(), Token::Name("A B C".to_owned()));
    /// assert_eq!(tokenizer.next_token().unwrap(), Token::End);
    /// ```
    pub fn next_token(&mut self) -> Result<Token, TokenizerError> {
        if let Some(token) = self.lookahead.take() {
            return Ok(token);
        }

        if self.position >= self.length {
            self.fill_buffer()?;

            if self.length == 0 {
                return Ok(Token::End);
            }
        }

        let byte = self.buffer[self.position];
        match byte {
            b'-' | b'.' | b'0'..=b'9' => self.read_numeral_or_string(),
            b',' => {
                self.position += 1;
                Ok(Token::Comma)
            }
            b'(' => {
                self.position += 1;
                Ok(Token::OpenParen)
            }
            b')' => {
                self.position += 1;
                Ok(Token::CloseParen)
            }
            b':' => {
                self.position += 1;
                Ok(Token::Colon)
            }
            b';' => {
                self.position += 1;
                Ok(Token::Semicolon)
            }
            b'\'' => self.read_quoted_string(),
            b' ' | b'\r' | b'\n' | b'\t' => {
                self.position += 1;
                self.next_token()
            }
            _ => self.read_string(),
        }
    }

    #[inline]
    fn find_token_end(buffer: &[u8], predicate: fn(&u8) -> bool) -> usize {
        buffer.iter().position(predicate).unwrap_or(buffer.len())
    }

    #[inline]
    fn read_token(&mut self, predicate: fn(&u8) -> bool) -> Result<Cow<'_, [u8]>, TokenizerError> {
        let start = self.position;
        let end = Self::find_token_end(&self.buffer[self.position..self.length], predicate);
        self.position += end;

        // panic mode: if we reach the end of the buffer, we need to explicitly copy data,
        // and then refill the buffer
        if self.position == self.length {
            let mut literal = Vec::with_capacity(32);
            literal.extend_from_slice(&self.buffer[start..self.length]);

            self.fill_buffer()?;

            // if we aren't at the end of the stream, we continue to read the float
            if self.length > 0 {
                let start = self.position;
                let end = Self::find_token_end(&self.buffer[self.position..self.length], predicate);
                self.position += end;

                ensure!(
                    self.position < self.length,
                    ParseSnafu {
                        reason: format!("literal exceeds {} KiB", BUFFER_SIZE / 1024)
                    }
                );
                literal.extend_from_slice(&self.buffer[start..self.position]);
            }

            return Ok(Cow::Owned(literal));
        }

        // otherwise, parse and return
        Ok(Cow::Borrowed(&self.buffer[start..self.position]))
    }

    /// Reads a numeric-leading literal from the input stream, starting at the current position.
    /// Returns a `Token::Float` for pure numeric literals and `Token::Name` otherwise.
    ///
    /// If the buffer is exhausted while reading, it will attempt to fill the buffer
    /// and continue reading. If the end of the stream is reached, it will return a literal containing
    /// the data read so far.
    ///
    /// If the literal is larger than the buffer size, it will panic.
    fn read_numeral_or_string(&mut self) -> Result<Token, TokenizerError> {
        let translate_underscores = self.settings.translate_underscores;
        let token = self.read_token(|&b| {
            b.is_ascii_whitespace() || b == b',' || b == b';' || b == b':' || b == b'(' || b == b')'
        })?;

        if token.iter().all(|b| b.is_ascii_digit() || *b == b'.' || *b == b'-') {
            return Ok(Token::Float(
                String::from_utf8_lossy(&token).parse().context(FloatSnafu {})?,
            ));
        }

        Ok(Token::Name(if translate_underscores {
            String::from_utf8_lossy(&token).replace('_', " ")
        } else {
            String::from_utf8_lossy(&token).to_string()
        }))
    }

    fn read_string(&mut self) -> Result<Token, TokenizerError> {
        let translate_underscores = self.settings.translate_underscores;
        let token = self.read_token(|&b| {
            b.is_ascii_whitespace() || b == b',' || b == b';' || b == b':' || b == b'(' || b == b')'
        })?;
        Ok(Token::Name(if translate_underscores {
            String::from_utf8_lossy(&token).replace('_', " ")
        } else {
            String::from_utf8_lossy(&token).to_string()
        }))
    }

    fn read_quoted_string(&mut self) -> Result<Token, TokenizerError> {
        self.position += 1;
        let token = self.read_token(|&b| b == b'\'' || b == b'\n')?;
        let mut token_string = String::from_utf8_lossy(&token).into_owned();

        loop {
            if self.position >= self.length || self.buffer[self.position] != b'\'' {
                return Err(TokenizerError::ParseError {
                    reason: "Unterminated quoted string".to_string(),
                });
            }
            self.position += 1;

            if self.position < self.length && self.buffer[self.position] == b'\'' {
                self.position += 1; // consume the escaped character, so the predicate works
                token_string.push('\'');

                // read until we find a newline or another quote
                let continued = self.read_token(|&b| b == b'\'' || b == b'\n')?;
                token_string.push_str(&String::from_utf8_lossy(&continued));
            } else {
                break;
            }
        }
        Ok(Token::Name(token_string))
    }

    /// Reads from the input stream into the buffer.
    /// Sets `position` to 0 and `length` to the number of bytes read.
    /// If the end of the stream is reached, `length` will be 0.
    /// This needs to be handled by the caller.
    fn fill_buffer(&mut self) -> Result<(), TokenizerError> {
        let bytes_read = self.reader.read(&mut *self.buffer).context(InputSnafu {})?;
        self.position = 0;
        self.length = bytes_read;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;
    use std::fs::File;
    use std::io::{BufRead, BufReader};
    use std::path::PathBuf;

    #[rstest]
    fn expect_working(#[files("tests/resources/tokenizer/accept/*.nw")] path: PathBuf) {
        // output the file name for easy identification in log files
        println!("Testing file: {:?}", path.file_name().unwrap());

        let mut expected_output = path.clone();
        expected_output.set_extension("out");

        let stream = File::open(path).expect("Could not open file");
        let expected_stream = File::open(expected_output).expect("Could not open expected output file");

        let mut tokenizer = Tokenizer::new(stream);
        let mut expected_reader = BufReader::new(expected_stream);
        let mut expected = String::with_capacity(64);

        loop {
            let result = tokenizer.next_token();
            if let Ok(result) = result {
                expected.clear();
                expected_reader
                    .read_line(&mut expected)
                    .expect("Could not read expected output");

                if expected.is_empty() {
                    assert!(false, "Expected output is empty, but got {:?}", result);
                }

                if expected.ends_with('\n') {
                    expected.pop();
                    if expected.ends_with('\r') {
                        expected.pop();
                    }
                }

                assert_eq!(format!("{:?}", result), expected);

                if let Token::End = result {
                    break;
                }
            } else {
                assert!(false, "Error while reading token: {:?}", result);
            }
        }
    }

    #[rstest]
    fn reject_failing(#[files("tests/resources/tokenizer/reject/*.nw")] path: PathBuf) {
        // output the file name for easy identification in log files
        println!("Testing file: {:?}", path.file_name().unwrap());

        let stream = File::open(path).expect("Could not open file");
        let mut tokenizer = Tokenizer::new(stream);

        loop {
            let result = tokenizer.next_token();
            if let Ok(result) = result {
                if let Token::End = result {
                    // if we reach this point, the test has failed
                    assert!(false, "Expected failure, but no error was raised");
                }
            } else {
                // if we reach this point, the test has passed
                break;
            }
        }
    }

    #[test]
    fn test_reading_past_buffer() {
        let mut input = String::from("()");
        let mut name = String::new();
        for _ in 0..BUFFER_SIZE {
            name.push('A');
        }
        input.push_str(&name);
        input.push(';');

        let mut tokenizer = Tokenizer::new(input.as_bytes());
        let Ok(paren_left) = tokenizer.next_token() else {
            panic!("did not find open parenthesis")
        };
        assert_eq!(paren_left, Token::OpenParen);
        let Ok(paren_right) = tokenizer.next_token() else {
            panic!("did not find closing parenthesis")
        };
        assert_eq!(paren_right, Token::CloseParen);
        let Ok(name_token) = tokenizer.next_token() else {
            panic!("did not find name")
        };
        assert_eq!(name_token, Token::Name(name));
        let Ok(semicolon) = tokenizer.next_token() else {
            panic!("did not find semicolon")
        };
        assert_eq!(semicolon, Token::Semicolon);
    }

    #[test]
    fn test_no_replacement() {
        // test whether not replacing underscores in strings works
        let input = String::from("AB_C,'XY_Z'");
        let mut tokenizer =
            Tokenizer::with_settings(input.as_bytes(), Settings::default().translate_underscores(false));

        if let Ok(Token::Name(name)) = tokenizer.next_token() {
            assert_eq!("AB_C", name, "incorrectly parsed unquoted string");
        };

        let Ok(Token::Comma) = tokenizer.next_token() else {
            panic!("missing comma");
        };

        if let Ok(Token::Name(name)) = tokenizer.next_token() {
            assert_eq!("XY_Z", name, "incorrectly parsed quoted string");
        };
    }
}

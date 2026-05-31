use crate::TreeSerialize;
use crate::config::{QuotationMode, Settings};
use std::iter::Peekable;
use std::marker::PhantomData;

/// Characters not allowed in unquoted Newick strings since they have meaning in Newick itself.
/// Strings containing only whitespace may be encoded by replacing white space with underscores.
/// Strings containing any of the other characters must be quoted.
///
/// The reserved characters are: `_`, `'`, `;`, `,`, `(`, `)`, `:`, `[`, `]`, and space.
/// Space can be handled differently than the other characters, see [`QuotationMode`].
///
/// [`QuotationMode`]: QuotationMode
pub const NEWICK_RESERVED_CHARACTERS: [char; 10] = [' ', '_', '\'', ';', ',', '(', ')', ':', '[', ']'];

/// Characters not allowed in unquoted Newick strings since they have meaning in Newick itself, other than whitespace.
/// Spaces have no meaning outside of strings other than to split tokens.
/// They can be handled differently than the other characters, see [`QuotationMode`].
///
/// The reserved characters are: `_`, `'`, `;`, `,`, `(`, `)`, `:`, `[`, `]`.
///
/// [`QuotationMode`]: QuotationMode
pub const NEWICK_RESERVED_CHARACTERS_NO_SPACE: [char; 9] = ['_', '\'', ';', ',', '(', ')', ':', '[', ']'];

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
#[derive(Default)]
pub struct Serializer<T: TreeSerialize> {
    tree_type: PhantomData<T>,
    settings: Settings,
}

impl<T: TreeSerialize> Serializer<T> {
    /// Creates a new instance of the `Serializer` with default [settings].
    ///
    /// [settings]: Settings
    pub fn new() -> Self {
        Self::with_settings(Settings::default())
    }

    /// Creates a new instance of the `Serializer` with custom [`Settings`].
    ///
    /// [`Settings`]: Settings
    pub fn with_settings(settings: Settings) -> Self {
        Serializer {
            tree_type: PhantomData,
            settings,
        }
    }

    /// Helper function to push node data into the result string.
    fn push_node_data(
        settings: &Settings,
        result: &mut String,
        label: Option<&String>,
        support: Option<f64>,
        branch_length: Option<f64>,
    ) {
        if let Some(label) = label {
            // todo sanitize input (remove illegal new-lines, etc)
            match settings.use_quoted_strings {
                QuotationMode::Always => result.push_str(&format!("'{}'", label.replace('\'', "''"))),
                QuotationMode::PreferUnquoted => {
                    if label.contains(NEWICK_RESERVED_CHARACTERS_NO_SPACE) {
                        result.push_str(&format!("'{}'", label.replace('\'', "''")))
                    } else {
                        result.push_str(&label.replace(' ', "_"))
                    }
                }
                QuotationMode::Dynamic => {
                    if label.contains(NEWICK_RESERVED_CHARACTERS) {
                        result.push_str(&format!("'{}'", label.replace('\'', "''")))
                    } else {
                        result.push_str(&label)
                    }
                }
                QuotationMode::Never => result.push_str(&label.replace(NEWICK_RESERVED_CHARACTERS, "_")),
            }
        } else if let Some(support) = support {
            result.push_str(&format!("{}", support));
        }

        if let Some(branch_length) = branch_length {
            result.push_str(&format!(":{}", branch_length));
        }
    }

    /// Serializes a tree into a newick format string according to the settings of this serializer.
    ///
    /// # Example
    /// ```
    /// # use nitro_newick::config::QuotationMode::Always;
    /// # use nitro_newick::config::Settings;
    /// # use nitro_newick::parser::Parser;
    /// # use nitro_newick::serializer::Serializer;
    /// # use nitro_newick::tree::SimpleTreeBuilder;
    /// let newick = "((A_A));";
    /// let tree = Parser::new(newick.as_bytes(), SimpleTreeBuilder::new()).parse().unwrap().unwrap();
    /// let serializer = Serializer::with_settings(Settings::default().use_quoted_strings(Always));
    /// let converted = serializer.serialize(&tree);
    /// assert_eq!(converted, "(('A A'));");
    /// ```
    pub fn serialize(&self, tree: &T) -> String {
        let root = tree.get_virtual_root();
        if root.is_none() {
            return String::from(';');
        }

        let mut result = String::new();
        let mut stack = Vec::new();
        let mut children = tree.get_children(root.unwrap(), root.unwrap()).peekable();

        if children.peek().is_none() {
            Self::push_node_data(
                &self.settings,
                &mut result,
                tree.get_label(root.as_ref().unwrap()),
                tree.get_tree_support(),
                tree.get_tree_branch_length(),
            );
            result.push(';');
            return result;
        } else {
            result.push('(');
            stack.push(Node {
                id: root.as_ref().unwrap(),
                label: tree.get_label(root.as_ref().unwrap()),
                support: tree.get_tree_support(),
                branch_length: tree.get_tree_branch_length(),
                children,
            });
        }

        loop {
            let node = stack.last_mut().unwrap();
            if let Some((child_id, support, branch_length)) = node.children.next() {
                let mut children = tree.get_children(*node.id, *child_id).peekable();
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
                    Self::push_node_data(
                        &self.settings,
                        &mut result,
                        tree.get_label(child_id),
                        support,
                        branch_length,
                    );
                }
            } else {
                let node = stack.pop().unwrap();
                result.push(')');
                Self::push_node_data(
                    &self.settings,
                    &mut result,
                    node.label,
                    node.support,
                    node.branch_length,
                );
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
    use crate::config::QuotationMode::{Always, Dynamic, Never, PreferUnquoted};
    use crate::parser::Parser;
    use crate::tree::{NTree, SimpleTreeBuilder};
    use rstest::rstest;
    use std::fs::File;
    use std::io::Read;
    use std::path::PathBuf;

    #[test]
    fn test_serialize() {
        let newick = "(A:0.1,B:0.2,(C:0.3,D:0.4):0.5);";
        let mut parser = Parser::new(newick.as_bytes(), SimpleTreeBuilder::new());
        let tree = parser.parse().unwrap().expect("Parse Error");
        let serializer = Serializer::<NTree>::new();
        let serialized = serializer.serialize(&tree);
        assert_eq!(serialized, newick);
    }

    #[test]
    fn test_serialize_anonymous() {
        let newick = "(:0.1,:0.2,(,D:0.4)F);";
        let mut parser = Parser::new(newick.as_bytes(), SimpleTreeBuilder::new());
        let tree = parser.parse().unwrap().expect("Parse Error");
        let serializer = Serializer::<NTree>::new();
        let serialized = serializer.serialize(&tree);
        assert_eq!(serialized, newick);
    }

    #[rstest]
    fn expect_working(#[files("tests/resources/serializer/*.nw")] path: PathBuf) {
        // output the file name for easy identification in log files
        println!("Testing file: {:?}", path.file_name().unwrap());

        let stream = File::open(path).expect("Could not open file");
        let mut newick = String::new();
        let mut reader = std::io::BufReader::new(stream);
        reader.read_to_string(&mut newick).expect("Could not read file");
        let mut parser = Parser::new(newick.as_bytes(), SimpleTreeBuilder::new());
        let tree = parser.parse().unwrap().expect("Parse Error");
        let serializer = Serializer::<NTree>::new();
        let serialized = serializer.serialize(&tree);
        assert_eq!(serialized, newick);
    }

    #[test]
    fn test_use_quotes() {
        for (setting, expected) in [(Always, "'A_B C'"), (Never, "A_B_C"), (Dynamic, "'A_B C'")] {
            let settings = Settings::default().use_quoted_strings(setting);

            let mut result = String::new();
            Serializer::<NTree>::push_node_data(&settings, &mut result, Some(&"A_B C".to_string()), None, None);

            assert_eq!(
                expected, result,
                "{setting:?} generates unexpected result \"{result}\" instead of \"{expected}\""
            );
        }
    }

    #[test]
    fn test_double_quotes() {
        // test whether escpaping works
        let settings = Settings::default().use_quoted_strings(Always);

        let mut result = String::new();
        Serializer::<NTree>::push_node_data(&settings, &mut result, Some(&"A'B".to_string()), None, None);
        assert_eq!("'A''B'", result);

        let settings = Settings::default().use_quoted_strings(Dynamic);
        result.clear();
        Serializer::<NTree>::push_node_data(&settings, &mut result, Some(&"A'B_".to_string()), None, None);
        assert_eq!("'A''B_'", result);

        result.clear();
        Serializer::<NTree>::push_node_data(&settings, &mut result, Some(&"A'B".to_string()), None, None);
        assert_eq!("'A''B'", result);
    }

    #[test]
    fn test_dynamic_quotes() {
        // test whether Dynamic mode doesnt use quotes if unnecessary
        let settings = Settings::default().use_quoted_strings(Dynamic);

        let mut result = String::new();
        Serializer::<NTree>::push_node_data(&settings, &mut result, Some(&"AB".to_string()), None, None);

        assert_eq!("AB", result);
    }

    // verify that prefer-unquoted quotation escapes all illegal characters.
    fn test_auto_quotation(mode: QuotationMode, char: &str) {
        let settings = Settings::default().use_quoted_strings(mode);
        let mut result = String::new();
        Serializer::<NTree>::push_node_data(&settings, &mut result, Some(&char.to_string()), None, None);
        assert_eq!(format!("'{char}'"), result);
    }

    #[test]
    fn test_illegal_characters_prefer_unquoted() {
        test_auto_quotation(PreferUnquoted, "[");
        test_auto_quotation(PreferUnquoted, "]");
        test_auto_quotation(PreferUnquoted, "(");
        test_auto_quotation(PreferUnquoted, ")");
        test_auto_quotation(PreferUnquoted, ";");
        test_auto_quotation(PreferUnquoted, ":");
        test_auto_quotation(PreferUnquoted, ",");
        test_auto_quotation(PreferUnquoted, "_");

        test_auto_quotation(PreferUnquoted, "A_A");
        test_auto_quotation(PreferUnquoted, "AAAA;");
        test_auto_quotation(PreferUnquoted, "[ ]");

        let settings = Settings::default().use_quoted_strings(PreferUnquoted);
        let mut result = String::new();
        Serializer::<NTree>::push_node_data(&settings, &mut result, Some(&" ".to_string()), None, None);
        assert_eq!("_".to_string(), result);
    }

    #[test]
    fn test_illegal_characters_prefer_quoted() {
        test_auto_quotation(Dynamic, "[");
        test_auto_quotation(Dynamic, "]");
        test_auto_quotation(Dynamic, "(");
        test_auto_quotation(Dynamic, ")");
        test_auto_quotation(Dynamic, ";");
        test_auto_quotation(Dynamic, ":");
        test_auto_quotation(Dynamic, ",");
        test_auto_quotation(Dynamic, "_");
        test_auto_quotation(Dynamic, " ");

        test_auto_quotation(Dynamic, "A_A");
        test_auto_quotation(Dynamic, "AAAA;");
        test_auto_quotation(Dynamic, "[ ]");
    }
}

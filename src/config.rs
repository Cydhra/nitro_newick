//! Configuration for the [`Parser`] and the [`Serializer`].
//! The [`Settings`] struct can be configured with a builder pattern and given to
//! parser and serializer.
//!
//! [`Parser`]: crate::parser::Parser
//! [`Serializer`]: crate::serializer::Serializer
//! [`Settings`]: Settings

use crate::config::QuotationMode::*;

/// Serializer behavior for Newick strings.
/// Since there is ambiguity on how to encode strings, the behavior can be changed.
/// Note that some behaviors violate the Newick standard.
///
/// Strings are written verbatim into the Newick representation, unless the string contains a space, or any character
/// reserved by the Newick standard. Spaces can be replaced with underscores, unless other [reserved characters] or
/// underscores are present. If reserved characters are present, a string must be enclosed in single quotes.
/// Single quotes in the string are escaped by doubling them (`''`).
///
/// [reserved characters]: crate::serializer::NEWICK_RESERVED_CHARACTERS
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum QuotationMode {
    /// Always use quoted strings.
    Always,

    /// Use unquoted strings unless the string contains a space, or another [reserved character].
    /// Since labels containing spaces are quoted, spaces are not replaced by underscores.
    ///
    /// [reserved character]: crate::serializer::NEWICK_RESERVED_CHARACTERS
    Dynamic,

    /// Use unquoted strings unless a [reserved character] (other than space) is present.
    /// Spaces are replaced with underscores.
    ///
    /// [reserved character]: crate::serializer::NEWICK_RESERVED_CHARACTERS_NO_SPACE
    PreferUnquoted,

    /// Never use quoted strings, replace all [reserved characters] with underscores.
    /// This does not adhere to the Newick standard and will irrecoverably change labels that contain [reserved
    /// characters] other than space.
    ///
    /// [reserved characters]: crate::serializer::NEWICK_RESERVED_CHARACTERS
    Never,
}

/// Settings for serialization and deserialization. This mostly affects string (de-)serialization
/// as it is defined ambiguously in the standard.
#[derive(Debug, Clone, PartialEq)]
pub struct Settings {
    pub(crate) translate_underscores: bool,
    pub(crate) use_quoted_strings: QuotationMode,
    pub(crate) prefer_labels: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            translate_underscores: true,
            use_quoted_strings: Never,
            prefer_labels: false,
        }
    }
}

impl Settings {
    /// If true, translates underscores in unquoted strings into spaces on deserialization, as
    /// the Newick standard implies. This is the default setting.
    /// Note that the serialization behavior is unaffected by this setting, since spaces are illegal
    /// in unquoted strings and have to be replaced with underscores.
    /// See [use_quoted_strings] for controlling serialization behavior.
    #[inline]
    pub fn translate_underscores(mut self, translate_underscores: bool) -> Self {
        self.translate_underscores = translate_underscores;
        self
    }

    /// Controls which type of Newick string is used during serialization.
    /// See [QuotationMode].
    #[inline]
    pub fn use_quoted_strings(mut self, use_quoted_strings: QuotationMode) -> Self {
        self.use_quoted_strings = use_quoted_strings;
        self
    }

    /// Controls whether the serializer prefers labels or support values if both are present
    /// in a node.
    #[inline]
    pub fn prefer_labels(mut self, prefer_labels: bool) -> Self {
        self.prefer_labels = prefer_labels;
        self
    }
}

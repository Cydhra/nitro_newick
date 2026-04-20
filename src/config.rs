use crate::config::QuotationMode::*;

/// Serializer behavior for Newick strings.
#[derive(Debug, Copy, Clone)]
pub enum QuotationMode {
    /// Always use quoted strings.
    Always,

    /// Use unquoted strings except if the string contains a space or an underscore.
    Dynamic,

    /// Never use quoted strings, replace all spaces with underscores.
    Never,
}

/// Settings for serialization and deserialization. This mostly affects string (de-)serialization
/// as it is defined ambiguously in the standard.
#[derive(Debug, Clone)]
pub struct Settings {
    pub(crate) translate_underscores: bool,
    pub(crate) use_quoted_strings: QuotationMode,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            translate_underscores: true,
            use_quoted_strings: Never,
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
}

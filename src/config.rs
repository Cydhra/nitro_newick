use crate::config::QuotationMode::Always;

/// Serializer behavior for Newick strings.
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
pub struct Settings {
    translate_underscores: bool,
    use_quoted_strings: QuotationMode,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            translate_underscores: true,
            use_quoted_strings: Always,
        }
    }
}

impl Settings {
    /// If true, translates underscores in unquoted strings into spaces on deserialization, as
    /// the Newick standard implies. This is the default setting.
    /// Note that the serialization behavior is unaffected by this setting, since spaces are illegal
    /// in unquoted strings and have to be replaced with underscores.
    /// See [use_quoted_strings] for controlling serialization behavior.
    pub fn translate_underscores(&mut self, translate_underscores: bool) -> &mut Self {
        self.translate_underscores = translate_underscores;
        self
    }

    /// Control which type of Newick string is used during serialization.
    /// See [QuotationMode].
    pub fn use_quoted_strings(&mut self, use_quoted_strings: QuotationMode) -> &mut Self {
        self.use_quoted_strings = use_quoted_strings;
        self
    }
}

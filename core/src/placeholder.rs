use core::fmt;

/// Various styles of SQL parameter placeholders.
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PlaceholderStyle {
    /// Colon style placeholders (:param)
    Colon,
    /// At-sign style placeholders (@param)
    AtSign,
    /// Dollar style placeholders ($param)
    Dollar,
    #[default]
    Positional,
}

/// A SQL parameter placeholder.
#[derive(Default, Debug, Clone, Hash, Copy, PartialEq, Eq)]
pub struct Placeholder {
    /// The name of the parameter.
    pub name: Option<&'static str>,
    /// The style of the placeholder.
    pub style: PlaceholderStyle,
}

impl Placeholder {
    /// Creates a new placeholder with the given name and style.
    pub const fn with_style(name: &'static str, style: PlaceholderStyle) -> Self {
        Placeholder {
            name: Some(name),
            style,
        }
    }

    /// Creates a new colon-style placeholder.
    pub const fn colon(name: &'static str) -> Self {
        Self::with_style(name, PlaceholderStyle::Colon)
    }

    /// Creates a new at-sign-style placeholder.
    pub const fn at(name: &'static str) -> Self {
        Self::with_style(name, PlaceholderStyle::AtSign)
    }

    /// Creates a new dollar-style placeholder.
    pub const fn dollar(name: &'static str) -> Self {
        Self::with_style(name, PlaceholderStyle::Dollar)
    }

    /// Creates a positional placeholder ('?').
    pub const fn positional() -> Self {
        Placeholder {
            name: None,
            style: PlaceholderStyle::Positional,
        }
    }
}

impl fmt::Display for Placeholder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.style {
            PlaceholderStyle::Colon => write!(f, ":{}", self.name.unwrap_or_default()),
            PlaceholderStyle::AtSign => write!(f, "@{}", self.name.unwrap_or_default()),
            PlaceholderStyle::Dollar => write!(f, "${}", self.name.unwrap_or_default()),
            PlaceholderStyle::Positional => write!(f, "?"),
        }
    }
}

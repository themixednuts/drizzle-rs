use std::{borrow::Cow, fmt};

use crate::traits::SQLParam;

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

/// A SQL parameter that associates a value with a placeholder.
/// Designed to be const-friendly and zero-cost when possible.
#[derive(Debug, Clone)]
pub struct Param<'a, V: SQLParam> {
    /// The placeholder to use in the SQL
    pub placeholder: Placeholder,
    /// The value to bind
    pub value: Option<Cow<'a, V>>,
}

impl<'a, V: SQLParam> Param<'a, V> {
    pub fn new(placeholder: Placeholder, value: Option<Cow<'a, V>>) -> Self {
        Self { placeholder, value }
    }
}

impl<'a, V: SQLParam> From<OwnedParam<V>> for Param<'a, V> {
    fn from(value: OwnedParam<V>) -> Self {
        Self {
            placeholder: value.placeholder,
            value: value.value.map(|v| Cow::Owned(v)),
        }
    }
}

impl<'a, V: SQLParam> From<&'a OwnedParam<V>> for Param<'a, V> {
    fn from(value: &'a OwnedParam<V>) -> Self {
        Self {
            placeholder: value.placeholder,
            value: value.value.as_ref().map(|v| Cow::Borrowed(v)),
        }
    }
}

#[derive(Debug, Clone)]
pub struct OwnedParam<V: SQLParam> {
    /// The placeholder to use in the SQL
    pub placeholder: Placeholder,
    /// The value to bind
    pub value: Option<V>,
}

impl<'a, V: SQLParam> From<Param<'a, V>> for OwnedParam<V> {
    fn from(value: Param<'a, V>) -> Self {
        Self {
            placeholder: value.placeholder,
            value: value.value.map(|v| v.into_owned()),
        }
    }
}

impl<'a, V: SQLParam> From<&Param<'a, V>> for OwnedParam<V> {
    fn from(value: &Param<'a, V>) -> Self {
        Self {
            placeholder: value.placeholder,
            value: value.value.clone().map(|v| v.into_owned()),
        }
    }
}

impl<'a, T: SQLParam> Param<'a, T> {
    /// Creates a new parameter with a positional placeholder
    pub const fn positional(value: T) -> Self {
        Self {
            placeholder: Placeholder::positional(),
            value: Some(Cow::Owned(value)),
        }
    }

    /// Creates a new parameter with a specific placeholder and no value
    pub const fn from_placeholder(placeholder: Placeholder) -> Self {
        Self {
            placeholder,
            value: None,
        }
    }

    /// Creates a new parameter with a named placeholder (colon style)
    pub const fn named(name: &'static str, value: T) -> Self {
        Self {
            placeholder: Placeholder::colon(name),
            value: Some(Cow::Owned(value)),
        }
    }

    /// Creates a new parameter with a specific placeholder
    pub const fn with_placeholder(placeholder: Placeholder, value: T) -> Self {
        Self {
            placeholder,
            value: Some(Cow::Owned(value)),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ParamBind<'a, V: SQLParam> {
    pub name: &'a str,
    pub value: V,
}

impl<'a, V: SQLParam> ParamBind<'a, V> {
    pub const fn new(name: &'a str, value: V) -> Self {
        Self { name, value }
    }
}

/// Utility functions for creating placeholders with different styles
pub mod placeholders {
    use super::{Placeholder, PlaceholderStyle};

    pub const fn colon(name: &'static str) -> Placeholder {
        Placeholder::with_style(name, PlaceholderStyle::Colon)
    }

    pub const fn at(name: &'static str) -> Placeholder {
        Placeholder::with_style(name, PlaceholderStyle::AtSign)
    }

    pub const fn dollar(name: &'static str) -> Placeholder {
        Placeholder::with_style(name, PlaceholderStyle::Dollar)
    }
}

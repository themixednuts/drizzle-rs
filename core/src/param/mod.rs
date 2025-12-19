mod owned;
pub use owned::*;

#[cfg(all(feature = "alloc", not(feature = "std")))]
use alloc::borrow::Cow;
#[cfg(feature = "std")]
use std::borrow::Cow;

use crate::{placeholder::Placeholder, traits::SQLParam};

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

impl<'a, V: SQLParam> From<Placeholder> for Param<'a, V> {
    fn from(value: Placeholder) -> Self {
        Self {
            placeholder: value,
            value: None,
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

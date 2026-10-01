mod owned;
pub use owned::*;

use crate::prelude::*;
use crate::{placeholder::Placeholder, traits::SQLParam};

/// A placeholder in a SQL fragment, with its value once bound.
#[derive(Debug, Clone)]
pub struct Param<'a, V: SQLParam> {
    /// The placeholder written into the SQL text.
    pub placeholder: Placeholder,
    /// The bound value, or `None` until one is bound.
    pub value: Option<Cow<'a, V>>,
}

impl<'a, V: SQLParam> Param<'a, V> {
    /// Creates a parameter from a placeholder and an optional value.
    pub const fn new(placeholder: Placeholder, value: Option<Cow<'a, V>>) -> Self {
        Self { placeholder, value }
    }
}

impl<V: SQLParam> From<OwnedParam<V>> for Param<'_, V> {
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

impl<V: SQLParam> From<Placeholder> for Param<'_, V> {
    fn from(value: Placeholder) -> Self {
        Self {
            placeholder: value,
            value: None,
        }
    }
}

impl<T: SQLParam> Param<'_, T> {
    /// Creates an anonymous (positional) parameter holding `value`.
    pub const fn positional(value: T) -> Self {
        Self {
            placeholder: Placeholder::anonymous(),
            value: Some(Cow::Owned(value)),
        }
    }

    /// Creates a parameter with no value yet.
    #[must_use]
    pub const fn from_placeholder(placeholder: Placeholder) -> Self {
        Self {
            placeholder,
            value: None,
        }
    }

    /// Creates a named parameter holding `value`.
    pub const fn named(name: &'static str, value: T) -> Self {
        Self {
            placeholder: Placeholder::named(name),
            value: Some(Cow::Owned(value)),
        }
    }

    /// Creates a parameter with the given placeholder, holding `value`.
    pub const fn with_placeholder(placeholder: Placeholder, value: T) -> Self {
        Self {
            placeholder,
            value: Some(Cow::Owned(value)),
        }
    }
}

/// A value to bind to the placeholder called `name`.
///
/// Pass these to [`SQL::bind`](crate::SQL::bind) or to a prepared
/// statement. [`TypedPlaceholder::bind`](crate::TypedPlaceholder::bind)
/// creates one with a type check.
///
/// # Examples
///
/// ```
/// use drizzle_core::{ParamBind, Placeholder, SQL, ToSQL};
/// # use drizzle_core::{Dialect, SQLParam, SQLiteDialect};
/// # use std::borrow::Cow;
/// # #[derive(Debug, Clone, PartialEq)]
/// # struct Value(i64);
/// # impl SQLParam for Value {
/// #     const DIALECT: Dialect = Dialect::SQLite;
/// #     type DialectMarker = SQLiteDialect;
/// # }
/// # impl From<Value> for Cow<'_, Value> {
/// #     fn from(value: Value) -> Self { Cow::Owned(value) }
/// # }
///
/// let sql: SQL<'_, Value> = Placeholder::named("limit").to_sql();
/// let bound = sql.bind([ParamBind::new("limit", Value(20))]);
/// assert_eq!(bound.params().collect::<Vec<_>>(), [&Value(20)]);
/// ```
#[derive(Debug, Clone)]
pub struct ParamBind<'a, V: SQLParam> {
    /// The placeholder name. Empty for a positional binding.
    pub name: &'a str,
    /// The value to bind.
    pub value: V,
}

impl<'a, V: SQLParam> ParamBind<'a, V> {
    /// Creates a binding for the placeholder `name`.
    pub const fn new(name: &'a str, value: V) -> Self {
        Self { name, value }
    }

    /// Creates a binding with no name, matched by position.
    pub const fn positional(value: V) -> Self {
        Self { name: "", value }
    }
}

/// A fixed-size set of [`ParamBind`]s, iterated in order.
#[derive(Debug, Clone)]
pub struct ParamSet<'a, V: SQLParam, const N: usize> {
    binds: [ParamBind<'a, V>; N],
}

impl<'a, V: SQLParam, const N: usize> ParamSet<'a, V, N> {
    /// Creates a set from an array of bindings.
    pub const fn new(binds: [ParamBind<'a, V>; N]) -> Self {
        Self { binds }
    }
}

impl<'a, V: SQLParam, const N: usize> From<[ParamBind<'a, V>; N]> for ParamSet<'a, V, N> {
    fn from(value: [ParamBind<'a, V>; N]) -> Self {
        Self::new(value)
    }
}

impl<'a, V: SQLParam, const N: usize> IntoIterator for ParamSet<'a, V, N> {
    type Item = ParamBind<'a, V>;
    type IntoIter = core::array::IntoIter<ParamBind<'a, V>, N>;

    fn into_iter(self) -> Self::IntoIter {
        self.binds.into_iter()
    }
}

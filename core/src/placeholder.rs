use crate::bind::BindValue;
use crate::expr::{Expr, NonNull, Scalar};
use crate::param::ParamBind;
use crate::traits::{SQLParam, ToSQL};
use crate::types::DataType;
use crate::{Param, SQL};
use core::fmt;
use core::marker::PhantomData;

/// A SQL parameter placeholder.
///
/// Placeholders store a semantic name for parameter binding. The actual SQL syntax
/// (`$1`, `?`, `:name`) is determined by the `Dialect` at render time.
#[derive(Default, Debug, Clone, Hash, Copy, PartialEq, Eq)]
pub struct Placeholder {
    /// The semantic name of the parameter (used for binding by name).
    pub name: Option<&'static str>,
}

/// A placeholder that carries the expected SQL type at compile time.
#[derive(Default, Debug, Clone, Hash, Copy, PartialEq, Eq)]
pub struct TypedPlaceholder<T: DataType> {
    inner: Placeholder,
    _ty: PhantomData<fn() -> T>,
}

impl Placeholder {
    /// Creates a named placeholder.
    ///
    /// The name is used for binding; rendering is dialect-specific:
    /// - PostgreSQL: `$1`, `$2`, ... (positional, name ignored in SQL)
    /// - SQLite: `:name` for named placeholders
    /// - MySQL: `?` (positional, name ignored in SQL)
    pub const fn named(name: &'static str) -> Self {
        Self { name: Some(name) }
    }

    /// Creates an anonymous placeholder (no name).
    pub const fn anonymous() -> Self {
        Self { name: None }
    }

    /// Creates a typed named placeholder.
    pub const fn typed<T: DataType>(name: &'static str) -> TypedPlaceholder<T> {
        TypedPlaceholder {
            inner: Self::named(name),
            _ty: PhantomData,
        }
    }
}

impl<T: DataType> TypedPlaceholder<T> {
    /// Creates a typed named placeholder.
    pub const fn named(name: &'static str) -> Self {
        Placeholder::typed::<T>(name)
    }

    /// Binds a value to this placeholder with compile-time SQL type checking.
    pub fn bind<'a, V, R>(self, value: R) -> ParamBind<'a, V>
    where
        V: SQLParam,
        R: BindValue<'a, V, T>,
    {
        ParamBind {
            name: self.inner.name.unwrap_or(""),
            value: value.into_bind_value(),
        }
    }

    /// Returns the placeholder name if present.
    pub const fn name(self) -> Option<&'static str> {
        self.inner.name
    }

    /// Returns this typed placeholder as an untyped placeholder.
    pub const fn into_placeholder(self) -> Placeholder {
        self.inner
    }
}

impl<T: DataType> From<TypedPlaceholder<T>> for Placeholder {
    fn from(value: TypedPlaceholder<T>) -> Self {
        value.inner
    }
}

impl<'a, V: SQLParam + 'a> ToSQL<'a, V> for Placeholder {
    fn to_sql(&self) -> SQL<'a, V> {
        SQL {
            chunks: smallvec::smallvec![crate::SQLChunk::Param(Param {
                value: None,
                placeholder: *self,
            })],
        }
    }
}

impl<'a, V: SQLParam + 'a> Expr<'a, V> for Placeholder {
    type SQLType = crate::types::Placeholder;
    type Nullable = NonNull;
    type Aggregate = Scalar;
}

impl<'a, V: SQLParam + 'a, T: DataType> ToSQL<'a, V> for TypedPlaceholder<T> {
    fn to_sql(&self) -> SQL<'a, V> {
        self.inner.to_sql()
    }
}

impl<'a, V: SQLParam + 'a, T: DataType> Expr<'a, V> for TypedPlaceholder<T> {
    type SQLType = T;
    type Nullable = NonNull;
    type Aggregate = Scalar;
}

impl fmt::Display for Placeholder {
    /// Debug display: `?` for anonymous or `:name` for named.
    /// Note: actual SQL rendering uses dialect-specific placeholders via `SQL::write_to`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.name {
            Some(name) => write!(f, ":{}", name),
            None => write!(f, "?"),
        }
    }
}

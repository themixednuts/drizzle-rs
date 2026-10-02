use crate::bind::{BindValue, NullableBindValue};
use crate::expr::{Expr, NonNull, Null, Nullability, Scalar};
use crate::param::ParamBind;
use crate::traits::{SQLParam, ToSQL};
use crate::types::DataType;
use crate::{Param, SQL};
use core::fmt;
use core::marker::PhantomData;

/// A parameter placeholder whose value is supplied later, by name.
///
/// Use placeholders in prepared statements: build the query once, then bind
/// values each time it runs. The SQL text depends on the dialect: `:name`
/// for a named placeholder on SQLite, `$1, $2, ...` on PostgreSQL, `?` on
/// MySQL. Prefer [`Placeholder::typed`], which also checks the bound value's
/// type.
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
/// let sql: SQL<'_, Value> = SQL::raw("SELECT * FROM users WHERE id =")
///     .append(Placeholder::named("id").to_sql());
/// assert_eq!(sql.sql(), "SELECT * FROM users WHERE id = :id");
///
/// let bound = sql.bind([ParamBind::new("id", Value(1))]);
/// assert_eq!(bound.params().count(), 1);
/// ```
#[derive(Default, Debug, Clone, Hash, Copy, PartialEq, Eq)]
pub struct Placeholder {
    /// The name values are bound by; `None` for an anonymous placeholder.
    pub name: Option<&'static str>,
}

/// A named placeholder that knows its SQL type `T` and nullability `N`.
///
/// Create one with [`Placeholder::typed`] or [`Placeholder::typed_nullable`].
/// It works as a typed expression, and [`bind`](Self::bind) only accepts
/// values that can be stored in `T`.
///
/// # Examples
///
/// ```
/// use drizzle_core::{ParamBind, Placeholder, SQL, ToSQL};
/// use drizzle_types::sqlite::types::Integer;
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
/// # impl From<i64> for Value {
/// #     fn from(value: i64) -> Self { Value(value) }
/// # }
/// # impl From<&str> for Value {
/// #     fn from(_: &str) -> Self { Value(0) }
/// # }
///
/// let id = Placeholder::typed::<Integer>("id");
/// let sql: SQL<'_, Value> = SQL::raw("SELECT * FROM users WHERE id =").append(id.to_sql());
///
/// let binding: ParamBind<'_, Value> = id.bind(7_i64);
/// let bound = sql.bind([binding]);
/// assert_eq!(bound.params().collect::<Vec<_>>(), [&Value(7)]);
/// ```
///
/// # Compile-time checks
///
/// Binding a value of the wrong SQL type does not compile:
///
/// ```compile_fail
/// use drizzle_core::{ParamBind, Placeholder};
/// use drizzle_types::sqlite::types::Integer;
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
/// # impl From<i64> for Value {
/// #     fn from(value: i64) -> Self { Value(value) }
/// # }
/// # impl From<&str> for Value {
/// #     fn from(_: &str) -> Self { Value(0) }
/// # }
///
/// let id = Placeholder::typed::<Integer>("id");
/// let _: ParamBind<'_, Value> = id.bind("seven"); // TEXT into INTEGER
/// ```
#[derive(Default, Debug, Clone, Hash, Copy, PartialEq, Eq)]
pub struct TypedPlaceholder<T: DataType, N: Nullability = NonNull> {
    inner: Placeholder,
    _marker: PhantomData<fn() -> (T, N)>,
}

impl Placeholder {
    /// Creates a named placeholder.
    ///
    /// Values are bound by `name`. The SQL text depends on the dialect:
    /// - PostgreSQL: `$1`, `$2`, ... (the name does not appear);
    /// - SQLite: `:name`;
    /// - MySQL: `?` (the name does not appear).
    #[must_use]
    pub const fn named(name: &'static str) -> Self {
        Self { name: Some(name) }
    }

    /// Creates an anonymous placeholder. It cannot be bound by name.
    #[must_use]
    pub const fn anonymous() -> Self {
        Self { name: None }
    }

    /// Creates a named placeholder for a non-null value of SQL type `T`.
    #[must_use]
    pub const fn typed<T: DataType>(name: &'static str) -> TypedPlaceholder<T, NonNull> {
        TypedPlaceholder {
            inner: Self::named(name),
            _marker: PhantomData,
        }
    }

    /// Creates a named placeholder for a nullable value of SQL type `T`.
    /// Bind it with [`TypedPlaceholder::bind_opt`].
    #[must_use]
    pub const fn typed_nullable<T: DataType>(name: &'static str) -> TypedPlaceholder<T, Null> {
        TypedPlaceholder {
            inner: Self::named(name),
            _marker: PhantomData,
        }
    }
}

impl<T: DataType, N: Nullability> TypedPlaceholder<T, N> {
    /// Creates a named placeholder of this type.
    #[must_use]
    pub const fn named(name: &'static str) -> Self {
        Self {
            inner: Placeholder::named(name),
            _marker: PhantomData,
        }
    }

    /// Pairs this placeholder's name with `value`, for
    /// [`SQL::bind`](crate::SQL::bind) or a prepared statement.
    ///
    /// Only values whose SQL type can be stored in `T` are accepted.
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

    /// Returns the placeholder name.
    #[must_use]
    pub const fn name(self) -> Option<&'static str> {
        self.inner.name
    }

    /// Drops the type, returning a plain [`Placeholder`].
    #[must_use]
    pub const fn into_placeholder(self) -> Placeholder {
        self.inner
    }
}

impl<T: DataType> TypedPlaceholder<T, Null> {
    /// Pairs this placeholder's name with an optional value; `None` binds
    /// NULL.
    pub fn bind_opt<'a, V, R>(self, value: Option<R>) -> ParamBind<'a, V>
    where
        V: SQLParam,
        Option<R>: NullableBindValue<'a, V, T>,
    {
        ParamBind {
            name: self.inner.name.unwrap_or(""),
            value: value.into_nullable_bind_value(),
        }
    }
}

impl<T: DataType, N: Nullability> From<TypedPlaceholder<T, N>> for Placeholder {
    fn from(value: TypedPlaceholder<T, N>) -> Self {
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

impl crate::expr::ExprSources for Placeholder {
    type Sources = ();
}

impl<T: DataType, N: Nullability> crate::expr::ExprSources for TypedPlaceholder<T, N> {
    type Sources = ();
}

impl<'a, V: SQLParam + 'a> Expr<'a, V> for Placeholder {
    type SQLType = crate::types::Placeholder;
    type Nullable = NonNull;
    type Aggregate = Scalar;
}

impl<'a, V: SQLParam + 'a, T: DataType, N: Nullability> ToSQL<'a, V> for TypedPlaceholder<T, N> {
    fn to_sql(&self) -> SQL<'a, V> {
        self.inner.to_sql()
    }
}

impl<'a, V: SQLParam + 'a, T: DataType, N: Nullability> Expr<'a, V> for TypedPlaceholder<T, N> {
    type SQLType = T;
    type Nullable = N;
    type Aggregate = Scalar;
}

impl fmt::Display for Placeholder {
    /// Shows `:name`, or `?` when anonymous. SQL rendering uses the
    /// dialect's syntax instead (see [`SQL::write_to`](crate::SQL::write_to)).
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.name {
            Some(name) => write!(f, ":{name}"),
            None => write!(f, "?"),
        }
    }
}

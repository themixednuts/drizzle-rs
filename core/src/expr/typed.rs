//! [`SQLExpr`]: SQL text paired with its SQL type, nullability and aggregate kind.

use core::fmt::{self, Display};
use core::marker::PhantomData;
use core::ops::Deref;

use crate::sql::SQL;
use crate::traits::{SQLParam, ToSQL};
use crate::types::DataType;

use super::{Agg, AggregateKind, Expr, NonNull, Null, Nullability, Scalar};

/// A SQL fragment together with its type information.
///
/// Every function in [`crate::expr`] returns an `SQLExpr`. The type
/// parameters record what the fragment produces:
///
/// - `'a`: lifetime of borrowed values inside it;
/// - `V`: the dialect's value type (`SQLiteValue`, `PostgresValue`, ...);
/// - `T`: the SQL data type marker;
/// - `N`: [`NonNull`] or [`Null`];
/// - `A`: [`Scalar`] or [`Agg`];
/// - `S`: the tables it reads (see [`crate::scope`]).
///
/// `SQLExpr` dereferences to [`SQL`], so methods such as [`SQL::sql`] work on
/// it directly. The Rust operators `+ - * / %` and unary `-` do arithmetic on
/// numeric expressions, and `& | !` combine boolean ones.
///
/// # Examples
///
/// ```rust
/// # use drizzle_core::dialect::{Dialect, DialectTypes, SQLiteDialect as D};
/// # use drizzle_core::{ColumnRef, SQL, SQLParam, expr::*};
/// # #[derive(Clone, Debug)] struct Value(String);
/// # impl SQLParam for Value { const DIALECT: Dialect = Dialect::SQLite; type DialectMarker = D; }
/// # impl<X: ToString> From<X> for Value { fn from(v: X) -> Self { Value(v.to_string()) } }
/// # impl From<Value> for std::borrow::Cow<'_, Value> { fn from(v: Value) -> Self { Self::Owned(v) } }
/// # type C<X, N = NonNull> = &'static SQLExpr<'static, Value, X, N>;
/// # fn col<X: drizzle_core::types::DataType, N: Nullability>(c: &'static str) -> C<X, N> { Box::leak(Box::new(SQLExpr::new(SQL::column(ColumnRef::sql("users", c))))) }
/// # type Int = <D as DialectTypes>::Int; type Text = <D as DialectTypes>::Text; type Real = <D as DialectTypes>::Double;
/// # struct Users { id: C<Int>, age: C<Int>, name: C<Text>, email: C<Text, Null>, score: C<Real, Null>, active: C<<D as DialectTypes>::Bool>, created_at: C<<D as DialectTypes>::Timestamp> }
/// # let users = Users { id: col("id"), age: col("age"), name: col("name"), email: col("email"), score: col("score"), active: col("active"), created_at: col("created_at") };
/// let total = users.age.clone() * 2 + 1;
/// assert_eq!(total.sql(), r#""users"."age" * ? + ?"#);
/// ```

#[derive(Debug, Clone)]
pub struct SQLExpr<
    'a,
    V: SQLParam,
    T: DataType,
    N: Nullability = NonNull,
    A: AggregateKind = Scalar,
    S = (),
> {
    sql: SQL<'a, V>,
    _ty: super::TypeMarker<(T, N, A, S)>,
}

impl<'a, V: SQLParam, T: DataType, N: Nullability, A: AggregateKind, S> SQLExpr<'a, V, T, N, A, S> {
    /// Wraps a SQL fragment, declaring its type.
    ///
    /// The type parameters are trusted, not checked against the SQL.
    #[inline]
    pub const fn new(sql: SQL<'a, V>) -> Self {
        Self {
            sql,
            _ty: PhantomData,
        }
    }

    /// Returns the SQL fragment, dropping the type information.
    #[inline]
    pub fn into_sql(self) -> SQL<'a, V> {
        self.sql
    }

    /// Changes the nullability marker.
    #[inline]
    #[allow(dead_code)]
    pub(crate) fn with_nullability<N2: Nullability>(self) -> SQLExpr<'a, V, T, N2, A, S> {
        SQLExpr {
            sql: self.sql,
            _ty: PhantomData,
        }
    }

    /// Marks this expression as nullable, keeping its SQL type and aggregate
    /// kind.
    #[inline]
    #[must_use]
    pub fn nullable(self) -> SQLExpr<'a, V, T, Null, A, S> {
        self.with_nullability::<Null>()
    }

    /// Changes the aggregate marker.
    #[inline]
    #[allow(dead_code)]
    pub(crate) fn with_aggregation<A2: AggregateKind>(self) -> SQLExpr<'a, V, T, N, A2, S> {
        SQLExpr {
            sql: self.sql,
            _ty: PhantomData,
        }
    }

    /// Changes the SQL type marker.
    #[inline]
    #[allow(dead_code)]
    pub(crate) fn with_type<T2: DataType>(self) -> SQLExpr<'a, V, T2, N, A, S> {
        SQLExpr {
            sql: self.sql,
            _ty: PhantomData,
        }
    }

    /// Replaces the recorded sources.
    #[inline]
    pub(crate) fn with_sources<S2>(self) -> SQLExpr<'a, V, T, N, A, S2> {
        SQLExpr {
            sql: self.sql,
            _ty: PhantomData,
        }
    }

    /// Forgets which tables this expression reads.
    ///
    /// The result passes every scope check. Use it only for SQL that is valid
    /// in a scope the type system cannot see, such as a correlated subquery
    /// built separately from its outer query.
    #[inline]
    #[must_use]
    pub fn unscoped(self) -> SQLExpr<'a, V, T, N, A> {
        self.with_sources()
    }
}

// =============================================================================
// ToSQL Implementation
// =============================================================================

impl<'a, V: SQLParam, T: DataType, N: Nullability, A: AggregateKind, S> ToSQL<'a, V>
    for SQLExpr<'a, V, T, N, A, S>
{
    fn to_sql(&self) -> SQL<'a, V> {
        self.sql.clone()
    }

    fn into_sql(self) -> SQL<'a, V> {
        self.sql
    }
}

// =============================================================================
// Into<SQL> Implementation - For builder compatibility
// =============================================================================

impl<'a, V: SQLParam, T: DataType, N: Nullability, A: AggregateKind, S>
    From<SQLExpr<'a, V, T, N, A, S>> for SQL<'a, V>
{
    fn from(expr: SQLExpr<'a, V, T, N, A, S>) -> Self {
        expr.sql
    }
}

// =============================================================================
// Expr Implementation
// =============================================================================

impl<'a, V: SQLParam, T: DataType, N: Nullability, A: AggregateKind, S> Expr<'a, V>
    for SQLExpr<'a, V, T, N, A, S>
{
    type SQLType = T;
    type Nullable = N;
    type Aggregate = A;
}

impl<V: SQLParam, T: DataType, N: Nullability, A: AggregateKind, S> super::ExprSources
    for SQLExpr<'_, V, T, N, A, S>
{
    type Sources = S;
}

// =============================================================================
// Convenience Type Aliases
// =============================================================================

/// A non-null, scalar [`SQLExpr`].
pub type ScalarExpr<'a, V, T> = SQLExpr<'a, V, T, NonNull, Scalar>;

/// A nullable, scalar [`SQLExpr`].
pub type NullableExpr<'a, V, T> = SQLExpr<'a, V, T, Null, Scalar>;

/// A non-null, aggregate [`SQLExpr`].
pub type AggExpr<'a, V, T> = SQLExpr<'a, V, T, NonNull, Agg>;

/// A nullable, aggregate [`SQLExpr`].
pub type NullableAggExpr<'a, V, T> = SQLExpr<'a, V, T, Null, Agg>;

// =============================================================================
// Display Implementation
// =============================================================================

/// Formats the expression like its inner [`SQL`].
impl<V, T, N, A, S> Display for SQLExpr<'_, V, T, N, A, S>
where
    V: SQLParam + Display,
    T: DataType,
    N: Nullability,
    A: AggregateKind,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Display::fmt(&self.sql, f)
    }
}

impl<V, T, N, A, S> super::HasAggStatus for SQLExpr<'_, V, T, N, A, S>
where
    V: SQLParam,
    T: DataType,
    N: Nullability,
    A: AggregateKind,
{
    type Status = A::Status;
}

// =============================================================================
// Deref Implementation
// =============================================================================

/// Gives direct access to the inner [`SQL`] and its methods, such as
/// [`SQL::sql`].
impl<'a, V, T, N, A, S> Deref for SQLExpr<'a, V, T, N, A, S>
where
    V: SQLParam,
    T: DataType,
    N: Nullability,
    A: AggregateKind,
{
    type Target = SQL<'a, V>;

    fn deref(&self) -> &Self::Target {
        &self.sql
    }
}

// =============================================================================
// AsRef Implementation
// =============================================================================

/// Borrows the inner [`SQL`].
impl<'a, V, T, N, A, S> AsRef<SQL<'a, V>> for SQLExpr<'a, V, T, N, A, S>
where
    V: SQLParam,
    T: DataType,
    N: Nullability,
    A: AggregateKind,
{
    fn as_ref(&self) -> &SQL<'a, V> {
        &self.sql
    }
}

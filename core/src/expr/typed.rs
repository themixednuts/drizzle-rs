//! `SQLExpr` - A typed SQL expression wrapper.

use core::fmt::{self, Display};
use core::marker::PhantomData;
use core::ops::Deref;

use crate::sql::SQL;
use crate::traits::{SQLParam, ToSQL};
use crate::types::DataType;

use super::{Agg, AggToStatus, AggregateKind, Expr, NonNull, Null, Nullability, Scalar};

/// A SQL expression that carries type information.
///
/// This wrapper preserves the SQL type through operations, enabling
/// compile-time type checking of SQL expressions.
///
/// # Type Parameters
///
/// - `'a`: Lifetime of borrowed data
/// - `V`: The dialect's value type (`SQLiteValue`, `PostgresValue`)
/// - `T`: The SQL data type marker (Int, Text, etc.)
/// - `N`: The nullability marker (`NonNull` or Null)
/// - `A`: The aggregation marker (Scalar or Agg)
/// - `S`: The sources the expression reads (see [`crate::scope`])
///
/// # Example
///
/// ```rust
/// # let _ = r####"
/// use drizzle_core::expr::{SQLExpr, NonNull, Scalar};
/// use drizzle_core::types::Int;
///
/// let expr: SQLExpr<'_, SQLiteValue, Int, NonNull, Scalar> = ...;
/// # "####;
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
    _ty: PhantomData<fn() -> (T, N, A, S)>,
}

impl<'a, V: SQLParam, T: DataType, N: Nullability, A: AggregateKind, S> SQLExpr<'a, V, T, N, A, S> {
    /// Create a new typed expression from raw SQL.
    #[inline]
    pub const fn new(sql: SQL<'a, V>) -> Self {
        Self {
            sql,
            _ty: PhantomData,
        }
    }

    /// Consume the wrapper and return the inner SQL.
    #[inline]
    pub fn into_sql(self) -> SQL<'a, V> {
        self.sql
    }

    /// Change the nullability marker (internal use only).
    #[inline]
    #[allow(dead_code)]
    pub(crate) fn with_nullability<N2: Nullability>(self) -> SQLExpr<'a, V, T, N2, A, S> {
        SQLExpr {
            sql: self.sql,
            _ty: PhantomData,
        }
    }

    /// Mark this expression as nullable while preserving its SQL type and aggregate kind.
    #[inline]
    #[must_use]
    pub fn nullable(self) -> SQLExpr<'a, V, T, Null, A, S> {
        self.with_nullability::<Null>()
    }

    /// Change the aggregation marker (internal use only).
    #[inline]
    #[allow(dead_code)]
    pub(crate) fn with_aggregation<A2: AggregateKind>(self) -> SQLExpr<'a, V, T, N, A2, S> {
        SQLExpr {
            sql: self.sql,
            _ty: PhantomData,
        }
    }

    /// Change the data type marker (internal use only).
    #[inline]
    #[allow(dead_code)]
    pub(crate) fn with_type<T2: DataType>(self) -> SQLExpr<'a, V, T2, N, A, S> {
        SQLExpr {
            sql: self.sql,
            _ty: PhantomData,
        }
    }

    /// Replace the recorded sources (internal use only).
    #[inline]
    pub(crate) fn with_sources<S2>(self) -> SQLExpr<'a, V, T, N, A, S2> {
        SQLExpr {
            sql: self.sql,
            _ty: PhantomData,
        }
    }

    /// Forget which sources this expression reads.
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

impl<'a, V: SQLParam, T: DataType, N: Nullability, A: AggregateKind, S> From<SQLExpr<'a, V, T, N, A, S>>
    for SQL<'a, V>
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

/// A scalar, non-null expression.
pub type ScalarExpr<'a, V, T> = SQLExpr<'a, V, T, NonNull, Scalar>;

/// A scalar, nullable expression.
pub type NullableExpr<'a, V, T> = SQLExpr<'a, V, T, Null, Scalar>;

/// An aggregate, non-null expression.
pub type AggExpr<'a, V, T> = SQLExpr<'a, V, T, NonNull, Agg>;

/// An aggregate, nullable expression.
pub type NullableAggExpr<'a, V, T> = SQLExpr<'a, V, T, Null, Agg>;

// =============================================================================
// Display Implementation
// =============================================================================

/// Display the SQL expression as a string.
///
/// Delegates to the inner `SQL` type's Display implementation.
///
/// # Example
///
/// ```rust
/// # let _ = r####"
/// let expr = eq(users.id, 42);
/// println!("{}", expr);  // "users"."id" = 42
/// # "####;
/// ```
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
    A: AggToStatus,
{
    type Status = A::Status;
}

// =============================================================================
// Deref Implementation
// =============================================================================

/// Provides transparent access to inner SQL methods via Deref coercion.
///
/// # Example
///
/// ```rust
/// # let _ = r####"
/// let expr = eq(users.id, 42);
/// // Access SQL methods directly:
/// let sql_str = expr.to_string();
/// # "####;
/// ```
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

/// Provides reference conversion to inner SQL.
///
/// # Example
///
/// ```rust
/// # let _ = r####"
/// fn takes_sql_ref<'a, V>(sql: &SQL<'a, V>) { ... }
/// let expr = eq(users.id, 42);
/// takes_sql_ref(expr.as_ref());
/// # "####;
/// ```
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

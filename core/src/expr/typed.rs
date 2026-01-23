//! SQLExpr - A typed SQL expression wrapper.

use core::fmt::{self, Display};
use core::marker::PhantomData;
use core::ops::Deref;

use crate::sql::SQL;
use crate::traits::{SQLParam, ToSQL};
use crate::types::DataType;

use super::{Agg, AggregateKind, Expr, NonNull, Null, Nullability, Scalar};

/// A SQL expression that carries type information.
///
/// This wrapper preserves the SQL type through operations, enabling
/// compile-time type checking of SQL expressions.
///
/// # Type Parameters
///
/// - `'a`: Lifetime of borrowed data
/// - `V`: The dialect's value type (SQLiteValue, PostgresValue)
/// - `T`: The SQL data type marker (Int, Text, etc.)
/// - `N`: The nullability marker (NonNull or Null)
/// - `A`: The aggregation marker (Scalar or Agg)
///
/// # Example
///
/// ```ignore
/// use drizzle_core::expr::{SQLExpr, NonNull, Scalar};
/// use drizzle_core::types::Int;
///
/// let expr: SQLExpr<'_, SQLiteValue, Int, NonNull, Scalar> = ...;
/// ```
#[derive(Debug, Clone)]
pub struct SQLExpr<
    'a,
    V: SQLParam,
    T: DataType,
    N: Nullability = NonNull,
    A: AggregateKind = Scalar,
> {
    sql: SQL<'a, V>,
    _ty: PhantomData<(T, N, A)>,
}

impl<'a, V: SQLParam, T: DataType, N: Nullability, A: AggregateKind> SQLExpr<'a, V, T, N, A> {
    /// Create a new typed expression from raw SQL.
    #[inline]
    pub fn new(sql: SQL<'a, V>) -> Self {
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

    /// Get a reference to the inner SQL.
    #[inline]
    pub fn as_sql(&self) -> &SQL<'a, V> {
        &self.sql
    }

    /// Change the nullability marker (internal use only).
    #[inline]
    #[allow(dead_code)]
    pub(crate) fn with_nullability<N2: Nullability>(self) -> SQLExpr<'a, V, T, N2, A> {
        SQLExpr {
            sql: self.sql,
            _ty: PhantomData,
        }
    }

    /// Change the aggregation marker (internal use only).
    #[inline]
    #[allow(dead_code)]
    pub(crate) fn with_aggregation<A2: AggregateKind>(self) -> SQLExpr<'a, V, T, N, A2> {
        SQLExpr {
            sql: self.sql,
            _ty: PhantomData,
        }
    }

    /// Change the data type marker (internal use only).
    #[inline]
    #[allow(dead_code)]
    pub(crate) fn with_type<T2: DataType>(self) -> SQLExpr<'a, V, T2, N, A> {
        SQLExpr {
            sql: self.sql,
            _ty: PhantomData,
        }
    }
}

// =============================================================================
// ToSQL Implementation
// =============================================================================

impl<'a, V: SQLParam, T: DataType, N: Nullability, A: AggregateKind> ToSQL<'a, V>
    for SQLExpr<'a, V, T, N, A>
{
    fn to_sql(&self) -> SQL<'a, V> {
        self.sql.clone()
    }
}

// =============================================================================
// Into<SQL> Implementation - For builder compatibility
// =============================================================================

impl<'a, V: SQLParam, T: DataType, N: Nullability, A: AggregateKind> From<SQLExpr<'a, V, T, N, A>>
    for SQL<'a, V>
{
    fn from(expr: SQLExpr<'a, V, T, N, A>) -> Self {
        expr.sql
    }
}

// =============================================================================
// Expr Implementation
// =============================================================================

impl<'a, V: SQLParam, T: DataType, N: Nullability, A: AggregateKind> Expr<'a, V>
    for SQLExpr<'a, V, T, N, A>
{
    type SQLType = T;
    type Nullable = N;
    type Aggregate = A;
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
/// ```ignore
/// let expr = eq(users.id, 42);
/// println!("{}", expr);  // "users"."id" = 42
/// ```
impl<'a, V, T, N, A> Display for SQLExpr<'a, V, T, N, A>
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

// =============================================================================
// Deref Implementation
// =============================================================================

/// Provides transparent access to inner SQL methods via Deref coercion.
///
/// # Example
///
/// ```ignore
/// let expr = eq(users.id, 42);
/// // Access SQL methods directly:
/// let sql_str = expr.to_string();
/// ```
impl<'a, V, T, N, A> Deref for SQLExpr<'a, V, T, N, A>
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
/// ```ignore
/// fn takes_sql_ref<'a, V>(sql: &SQL<'a, V>) { ... }
/// let expr = eq(users.id, 42);
/// takes_sql_ref(expr.as_ref());
/// ```
impl<'a, V, T, N, A> AsRef<SQL<'a, V>> for SQLExpr<'a, V, T, N, A>
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

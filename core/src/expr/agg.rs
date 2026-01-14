//! Type-safe aggregate functions.
//!
//! These functions return expressions marked as aggregates, which can be used
//! to enforce GROUP BY rules at compile time.
//!
//! # Type Safety
//!
//! - `sum`, `avg`: Require `Numeric` types (Int, BigInt, Float, Double)
//! - `count`: Works with any type
//! - `min`, `max`: Work with any type (ordered types in SQL)

use crate::sql::SQL;
use crate::traits::SQLParam;
use crate::types::{Any, BigInt, Double, Numeric};

use super::{Agg, Expr, NonNull, Null, Scalar, SQLExpr};

// =============================================================================
// COUNT
// =============================================================================

/// COUNT(*) - counts all rows.
///
/// Returns a BigInt, NonNull (count is never NULL), Aggregate expression.
///
/// # Example
///
/// ```ignore
/// use drizzle_core::expr::count_all;
///
/// let total = count_all();
/// // Generates: COUNT(*)
/// ```
pub fn count_all<'a, V>() -> SQLExpr<'a, V, BigInt, NonNull, Agg>
where
    V: SQLParam + 'a,
{
    SQLExpr::new(SQL::raw("COUNT(*)"))
}

/// COUNT(expr) - counts non-null values.
///
/// Returns a BigInt, NonNull (count is never NULL), Aggregate expression.
/// Works with any expression type.
///
/// # Example
///
/// ```ignore
/// use drizzle_core::expr::count;
///
/// let count = count(users.email);
/// // Generates: COUNT("users"."email")
/// ```
pub fn count<'a, V, E>(expr: E) -> SQLExpr<'a, V, BigInt, NonNull, Agg>
where
    V: SQLParam + 'a,
    E: Expr<'a, V>,
{
    SQLExpr::new(SQL::func("COUNT", expr.to_sql()))
}

/// COUNT(DISTINCT expr) - counts distinct non-null values.
///
/// Returns a BigInt, NonNull, Aggregate expression.
/// Works with any expression type.
pub fn count_distinct<'a, V, E>(expr: E) -> SQLExpr<'a, V, BigInt, NonNull, Agg>
where
    V: SQLParam + 'a,
    E: Expr<'a, V>,
{
    SQLExpr::new(SQL::func("COUNT", SQL::raw("DISTINCT").append(expr.to_sql())))
}

// =============================================================================
// SUM
// =============================================================================

/// SUM(expr) - sums numeric values.
///
/// Requires the expression to be `Numeric` (Int, BigInt, Float, Double).
/// Returns a nullable expression (empty set returns NULL).
///
/// # Type Safety
///
/// ```ignore
/// // ✅ OK: Numeric column
/// sum(orders.amount);
///
/// // ❌ Compile error: Text is not Numeric
/// sum(users.name);
/// ```
pub fn sum<'a, V, E>(expr: E) -> SQLExpr<'a, V, Any, Null, Agg>
where
    V: SQLParam + 'a,
    E: Expr<'a, V>,
    E::SQLType: Numeric,
{
    SQLExpr::new(SQL::func("SUM", expr.to_sql()))
}

/// SUM(DISTINCT expr) - sums distinct numeric values.
///
/// Requires the expression to be `Numeric`.
pub fn sum_distinct<'a, V, E>(expr: E) -> SQLExpr<'a, V, Any, Null, Agg>
where
    V: SQLParam + 'a,
    E: Expr<'a, V>,
    E::SQLType: Numeric,
{
    SQLExpr::new(SQL::func("SUM", SQL::raw("DISTINCT").append(expr.to_sql())))
}

// =============================================================================
// AVG
// =============================================================================

/// AVG(expr) - calculates average of numeric values.
///
/// Requires the expression to be `Numeric`.
/// Always returns Double (SQL standard behavior), nullable.
///
/// # Type Safety
///
/// ```ignore
/// // ✅ OK: Numeric column
/// avg(products.price);
///
/// // ❌ Compile error: Text is not Numeric
/// avg(users.name);
/// ```
pub fn avg<'a, V, E>(expr: E) -> SQLExpr<'a, V, Double, Null, Agg>
where
    V: SQLParam + 'a,
    E: Expr<'a, V>,
    E::SQLType: Numeric,
{
    SQLExpr::new(SQL::func("AVG", expr.to_sql()))
}

/// AVG(DISTINCT expr) - calculates average of distinct numeric values.
///
/// Requires the expression to be `Numeric`.
pub fn avg_distinct<'a, V, E>(expr: E) -> SQLExpr<'a, V, Double, Null, Agg>
where
    V: SQLParam + 'a,
    E: Expr<'a, V>,
    E::SQLType: Numeric,
{
    SQLExpr::new(SQL::func("AVG", SQL::raw("DISTINCT").append(expr.to_sql())))
}

// =============================================================================
// MIN / MAX
// =============================================================================

/// MIN(expr) - finds minimum value.
///
/// Works with any expression type (ordered types in SQL).
/// Result is nullable (empty set returns NULL).
///
/// # Example
///
/// ```ignore
/// use drizzle_core::expr::min;
///
/// let cheapest = min(products.price);
/// // Generates: MIN("products"."price")
/// ```
pub fn min<'a, V, E>(expr: E) -> SQLExpr<'a, V, Any, Null, Agg>
where
    V: SQLParam + 'a,
    E: Expr<'a, V>,
{
    SQLExpr::new(SQL::func("MIN", expr.to_sql()))
}

/// MAX(expr) - finds maximum value.
///
/// Works with any expression type (ordered types in SQL).
/// Result is nullable (empty set returns NULL).
///
/// # Example
///
/// ```ignore
/// use drizzle_core::expr::max;
///
/// let most_expensive = max(products.price);
/// // Generates: MAX("products"."price")
/// ```
pub fn max<'a, V, E>(expr: E) -> SQLExpr<'a, V, Any, Null, Agg>
where
    V: SQLParam + 'a,
    E: Expr<'a, V>,
{
    SQLExpr::new(SQL::func("MAX", expr.to_sql()))
}

// =============================================================================
// GROUP_CONCAT / STRING_AGG
// =============================================================================

/// GROUP_CONCAT / STRING_AGG - concatenates values into a string.
///
/// Note: This is dialect-specific (GROUP_CONCAT in SQLite/MySQL, STRING_AGG in PostgreSQL).
/// Returns Text type, nullable.
pub fn group_concat<'a, V, E>(expr: E) -> SQLExpr<'a, V, crate::types::Text, Null, Agg>
where
    V: SQLParam + 'a,
    E: Expr<'a, V>,
{
    SQLExpr::new(SQL::func("GROUP_CONCAT", expr.to_sql()))
}

// =============================================================================
// Distinct Wrapper
// =============================================================================

/// DISTINCT - marks an expression as DISTINCT.
///
/// Typically used inside aggregate functions.
pub fn distinct<'a, V, E>(expr: E) -> SQLExpr<'a, V, Any, Null, Scalar>
where
    V: SQLParam + 'a,
    E: Expr<'a, V>,
{
    SQLExpr::new(SQL::raw("DISTINCT").append(expr.to_sql()))
}

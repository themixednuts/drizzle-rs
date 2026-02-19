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
use crate::types::{Any, BigInt, Double, Float, Int, Numeric, SmallInt};
use crate::{PostgresDialect, SQLiteDialect};

use super::{Agg, Expr, NonNull, Null, SQLExpr, Scalar};

// =============================================================================
// Dialect Aggregate Policy
// =============================================================================

/// Dialect-specific aggregate output mapping.
///
/// Keeps aggregate output typing in one place so all aggregate functions
/// follow the same per-dialect policy.
#[diagnostic::on_unimplemented(
    message = "no aggregate policy for `{Self}` on this dialect",
    label = "aggregate result type is not defined for this SQL type/dialect"
)]
pub trait AggregatePolicy<D>: Numeric {
    type Sum: crate::types::DataType;
    type Avg: crate::types::DataType;
}

impl AggregatePolicy<SQLiteDialect> for SmallInt {
    type Sum = SmallInt;
    type Avg = Double;
}
impl AggregatePolicy<SQLiteDialect> for Int {
    type Sum = Int;
    type Avg = Double;
}
impl AggregatePolicy<SQLiteDialect> for BigInt {
    type Sum = BigInt;
    type Avg = Double;
}
impl AggregatePolicy<SQLiteDialect> for Float {
    type Sum = Float;
    type Avg = Double;
}
impl AggregatePolicy<SQLiteDialect> for Double {
    type Sum = Double;
    type Avg = Double;
}
impl AggregatePolicy<SQLiteDialect> for Any {
    type Sum = Any;
    type Avg = Double;
}

impl AggregatePolicy<PostgresDialect> for SmallInt {
    type Sum = BigInt;
    type Avg = Double;
}
impl AggregatePolicy<PostgresDialect> for Int {
    type Sum = BigInt;
    type Avg = Double;
}
impl AggregatePolicy<PostgresDialect> for BigInt {
    type Sum = BigInt;
    type Avg = Double;
}
impl AggregatePolicy<PostgresDialect> for Float {
    type Sum = Double;
    type Avg = Double;
}
impl AggregatePolicy<PostgresDialect> for Double {
    type Sum = Double;
    type Avg = Double;
}
impl AggregatePolicy<PostgresDialect> for Any {
    type Sum = Any;
    type Avg = Double;
}

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
    SQLExpr::new(SQL::func("COUNT", expr.into_sql()))
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
    SQLExpr::new(SQL::func(
        "COUNT",
        SQL::raw("DISTINCT").append(expr.into_sql()),
    ))
}

// =============================================================================
// SUM
// =============================================================================

/// SUM(expr) - sums numeric values.
///
/// Requires the expression to be `Numeric` (Int, BigInt, Float, Double).
/// Preserves the input expression's SQL type.
/// Returns a nullable expression (empty set returns NULL).
///
/// # Type Safety
///
/// ```ignore
/// // ✅ OK: Numeric column
/// sum(orders.amount);
/// // Returns the same SQL type as orders.amount
///
/// // ❌ Compile error: Text is not Numeric
/// sum(users.name);
/// ```
pub fn sum<'a, V, E>(
    expr: E,
) -> SQLExpr<'a, V, <E::SQLType as AggregatePolicy<V::DialectMarker>>::Sum, Null, Agg>
where
    V: SQLParam + 'a,
    E: Expr<'a, V>,
    E::SQLType: AggregatePolicy<V::DialectMarker>,
{
    SQLExpr::new(SQL::func("SUM", expr.into_sql()))
}

/// SUM(DISTINCT expr) - sums distinct numeric values.
///
/// Requires the expression to be `Numeric`.
/// Preserves the input expression's SQL type.
pub fn sum_distinct<'a, V, E>(
    expr: E,
) -> SQLExpr<'a, V, <E::SQLType as AggregatePolicy<V::DialectMarker>>::Sum, Null, Agg>
where
    V: SQLParam + 'a,
    E: Expr<'a, V>,
    E::SQLType: AggregatePolicy<V::DialectMarker>,
{
    SQLExpr::new(SQL::func(
        "SUM",
        SQL::raw("DISTINCT").append(expr.into_sql()),
    ))
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
pub fn avg<'a, V, E>(
    expr: E,
) -> SQLExpr<'a, V, <E::SQLType as AggregatePolicy<V::DialectMarker>>::Avg, Null, Agg>
where
    V: SQLParam + 'a,
    E: Expr<'a, V>,
    E::SQLType: AggregatePolicy<V::DialectMarker>,
{
    SQLExpr::new(SQL::func("AVG", expr.into_sql()))
}

/// AVG(DISTINCT expr) - calculates average of distinct numeric values.
///
/// Requires the expression to be `Numeric`.
pub fn avg_distinct<'a, V, E>(
    expr: E,
) -> SQLExpr<'a, V, <E::SQLType as AggregatePolicy<V::DialectMarker>>::Avg, Null, Agg>
where
    V: SQLParam + 'a,
    E: Expr<'a, V>,
    E::SQLType: AggregatePolicy<V::DialectMarker>,
{
    SQLExpr::new(SQL::func(
        "AVG",
        SQL::raw("DISTINCT").append(expr.into_sql()),
    ))
}

// =============================================================================
// MIN / MAX
// =============================================================================

/// MIN(expr) - finds minimum value.
///
/// Works with any expression type (ordered types in SQL).
/// Preserves the input expression's SQL type.
/// Result is nullable (empty set returns NULL).
///
/// # Example
///
/// ```ignore
/// use drizzle_core::expr::min;
///
/// let cheapest = min(products.price);
/// // Generates: MIN("products"."price")
/// // Returns the same SQL type as products.price
/// ```
pub fn min<'a, V, E>(expr: E) -> SQLExpr<'a, V, E::SQLType, Null, Agg>
where
    V: SQLParam + 'a,
    E: Expr<'a, V>,
{
    SQLExpr::new(SQL::func("MIN", expr.into_sql()))
}

/// MAX(expr) - finds maximum value.
///
/// Works with any expression type (ordered types in SQL).
/// Preserves the input expression's SQL type.
/// Result is nullable (empty set returns NULL).
///
/// # Example
///
/// ```ignore
/// use drizzle_core::expr::max;
///
/// let most_expensive = max(products.price);
/// // Generates: MAX("products"."price")
/// // Returns the same SQL type as products.price
/// ```
pub fn max<'a, V, E>(expr: E) -> SQLExpr<'a, V, E::SQLType, Null, Agg>
where
    V: SQLParam + 'a,
    E: Expr<'a, V>,
{
    SQLExpr::new(SQL::func("MAX", expr.into_sql()))
}

// =============================================================================
// STATISTICAL FUNCTIONS
// =============================================================================

/// STDDEV_POP - population standard deviation.
///
/// Calculates the population standard deviation of numeric values.
/// Requires the expression to be `Numeric`.
/// Returns Double, nullable (empty set returns NULL).
///
/// Note: This function is available in PostgreSQL. SQLite does not have it built-in.
///
/// # Example
///
/// ```ignore
/// use drizzle_core::expr::stddev_pop;
///
/// let deviation = stddev_pop(measurements.value);
/// // Generates: STDDEV_POP("measurements"."value")
/// ```
pub fn stddev_pop<'a, V, E>(expr: E) -> SQLExpr<'a, V, Double, Null, Agg>
where
    V: SQLParam + 'a,
    E: Expr<'a, V>,
    E::SQLType: Numeric,
{
    SQLExpr::new(SQL::func("STDDEV_POP", expr.into_sql()))
}

/// STDDEV_SAMP / STDDEV - sample standard deviation.
///
/// Calculates the sample standard deviation of numeric values.
/// Requires the expression to be `Numeric`.
/// Returns Double, nullable (empty set returns NULL).
///
/// Note: This function is available in PostgreSQL. SQLite does not have it built-in.
///
/// # Example
///
/// ```ignore
/// use drizzle_core::expr::stddev_samp;
///
/// let deviation = stddev_samp(measurements.value);
/// // Generates: STDDEV_SAMP("measurements"."value")
/// ```
pub fn stddev_samp<'a, V, E>(expr: E) -> SQLExpr<'a, V, Double, Null, Agg>
where
    V: SQLParam + 'a,
    E: Expr<'a, V>,
    E::SQLType: Numeric,
{
    SQLExpr::new(SQL::func("STDDEV_SAMP", expr.into_sql()))
}

/// VAR_POP - population variance.
///
/// Calculates the population variance of numeric values.
/// Requires the expression to be `Numeric`.
/// Returns Double, nullable (empty set returns NULL).
///
/// Note: This function is available in PostgreSQL. SQLite does not have it built-in.
///
/// # Example
///
/// ```ignore
/// use drizzle_core::expr::var_pop;
///
/// let variance = var_pop(measurements.value);
/// // Generates: VAR_POP("measurements"."value")
/// ```
pub fn var_pop<'a, V, E>(expr: E) -> SQLExpr<'a, V, Double, Null, Agg>
where
    V: SQLParam + 'a,
    E: Expr<'a, V>,
    E::SQLType: Numeric,
{
    SQLExpr::new(SQL::func("VAR_POP", expr.into_sql()))
}

/// VAR_SAMP / VARIANCE - sample variance.
///
/// Calculates the sample variance of numeric values.
/// Requires the expression to be `Numeric`.
/// Returns Double, nullable (empty set returns NULL).
///
/// Note: This function is available in PostgreSQL. SQLite does not have it built-in.
///
/// # Example
///
/// ```ignore
/// use drizzle_core::expr::var_samp;
///
/// let variance = var_samp(measurements.value);
/// // Generates: VAR_SAMP("measurements"."value")
/// ```
pub fn var_samp<'a, V, E>(expr: E) -> SQLExpr<'a, V, Double, Null, Agg>
where
    V: SQLParam + 'a,
    E: Expr<'a, V>,
    E::SQLType: Numeric,
{
    SQLExpr::new(SQL::func("VAR_SAMP", expr.into_sql()))
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
    SQLExpr::new(SQL::func("GROUP_CONCAT", expr.into_sql()))
}

// =============================================================================
// Distinct Wrapper
// =============================================================================

/// DISTINCT - marks an expression as DISTINCT.
///
/// Typically used inside aggregate functions.
pub fn distinct<'a, V, E>(expr: E) -> SQLExpr<'a, V, E::SQLType, E::Nullable, Scalar>
where
    V: SQLParam + 'a,
    E: Expr<'a, V>,
{
    SQLExpr::new(SQL::raw("DISTINCT").append(expr.into_sql()))
}

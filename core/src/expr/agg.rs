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
use crate::types::{
    Any, Array, BigInt, Bool, Double, Float, Int, Json, Jsonb, Numeric, SmallInt, Text,
};
use crate::{PostgresDialect, SQLiteDialect};
use drizzle_types::postgres::types::{Boolean as PgBoolean, Float4, Float8, Int2, Int4, Int8};
use drizzle_types::sqlite::types::{Integer as SqliteInteger, Real as SqliteReal};

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

#[diagnostic::on_unimplemented(
    message = "no statistical aggregate policy for `{Self}` on this dialect",
    label = "stddev/variance result type is not defined for this SQL type/dialect"
)]
pub trait StatisticalAggregatePolicy<D>: Numeric {
    type StddevPop: crate::types::DataType;
    type StddevSamp: crate::types::DataType;
    type VarPop: crate::types::DataType;
    type VarSamp: crate::types::DataType;
}

#[diagnostic::on_unimplemented(
    message = "boolean aggregates are not supported for `{Self}` on this dialect",
    label = "use a boolean expression with a dialect that supports BOOL_AND/BOOL_OR"
)]
pub trait BooleanAggregatePolicy<D>: crate::types::DataType {}

#[diagnostic::on_unimplemented(
    message = "this aggregate is not available for this dialect",
    label = "use a dialect-specific alternative"
)]
pub trait PostgresAggregateSupport {}

#[diagnostic::on_unimplemented(
    message = "this aggregate is not available for this dialect",
    label = "use a dialect-specific alternative"
)]
pub trait SQLiteAggregateSupport {}

pub trait CountPolicy {
    type Count: crate::types::DataType;
}

impl CountPolicy for SQLiteDialect {
    type Count = drizzle_types::sqlite::types::Integer;
}

impl CountPolicy for PostgresDialect {
    type Count = drizzle_types::postgres::types::Int8;
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
impl AggregatePolicy<SQLiteDialect> for SqliteInteger {
    type Sum = SqliteInteger;
    type Avg = Double;
}
impl AggregatePolicy<SQLiteDialect> for SqliteReal {
    type Sum = SqliteReal;
    type Avg = Double;
}

impl StatisticalAggregatePolicy<PostgresDialect> for SmallInt {
    type StddevPop = Double;
    type StddevSamp = Double;
    type VarPop = Double;
    type VarSamp = Double;
}
impl StatisticalAggregatePolicy<PostgresDialect> for Int {
    type StddevPop = Double;
    type StddevSamp = Double;
    type VarPop = Double;
    type VarSamp = Double;
}
impl StatisticalAggregatePolicy<PostgresDialect> for BigInt {
    type StddevPop = Double;
    type StddevSamp = Double;
    type VarPop = Double;
    type VarSamp = Double;
}
impl StatisticalAggregatePolicy<PostgresDialect> for Float {
    type StddevPop = Double;
    type StddevSamp = Double;
    type VarPop = Double;
    type VarSamp = Double;
}
impl StatisticalAggregatePolicy<PostgresDialect> for Double {
    type StddevPop = Double;
    type StddevSamp = Double;
    type VarPop = Double;
    type VarSamp = Double;
}
impl StatisticalAggregatePolicy<PostgresDialect> for Int2 {
    type StddevPop = Double;
    type StddevSamp = Double;
    type VarPop = Double;
    type VarSamp = Double;
}
impl StatisticalAggregatePolicy<PostgresDialect> for Int4 {
    type StddevPop = Double;
    type StddevSamp = Double;
    type VarPop = Double;
    type VarSamp = Double;
}
impl StatisticalAggregatePolicy<PostgresDialect> for Int8 {
    type StddevPop = Double;
    type StddevSamp = Double;
    type VarPop = Double;
    type VarSamp = Double;
}
impl StatisticalAggregatePolicy<PostgresDialect> for Float4 {
    type StddevPop = Double;
    type StddevSamp = Double;
    type VarPop = Double;
    type VarSamp = Double;
}
impl StatisticalAggregatePolicy<PostgresDialect> for Float8 {
    type StddevPop = Double;
    type StddevSamp = Double;
    type VarPop = Double;
    type VarSamp = Double;
}

impl BooleanAggregatePolicy<PostgresDialect> for Bool {}
impl BooleanAggregatePolicy<PostgresDialect> for PgBoolean {}

impl PostgresAggregateSupport for PostgresDialect {}
impl SQLiteAggregateSupport for SQLiteDialect {}

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
impl AggregatePolicy<PostgresDialect> for Int2 {
    type Sum = Int8;
    type Avg = Double;
}
impl AggregatePolicy<PostgresDialect> for Int4 {
    type Sum = Int8;
    type Avg = Double;
}
impl AggregatePolicy<PostgresDialect> for Int8 {
    type Sum = Int8;
    type Avg = Double;
}
impl AggregatePolicy<PostgresDialect> for Float4 {
    type Sum = Float8;
    type Avg = Float8;
}
impl AggregatePolicy<PostgresDialect> for Float8 {
    type Sum = Float8;
    type Avg = Float8;
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
pub fn count_all<'a, V>() -> SQLExpr<'a, V, <V::DialectMarker as CountPolicy>::Count, NonNull, Agg>
where
    V: SQLParam + 'a,
    V::DialectMarker: CountPolicy,
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
pub fn count<'a, V, E>(
    expr: E,
) -> SQLExpr<'a, V, <V::DialectMarker as CountPolicy>::Count, NonNull, Agg>
where
    V: SQLParam + 'a,
    V::DialectMarker: CountPolicy,
    E: Expr<'a, V>,
{
    SQLExpr::new(SQL::func("COUNT", expr.into_sql()))
}

/// COUNT(DISTINCT expr) - counts distinct non-null values.
///
/// Returns a BigInt, NonNull, Aggregate expression.
/// Works with any expression type.
pub fn count_distinct<'a, V, E>(
    expr: E,
) -> SQLExpr<'a, V, <V::DialectMarker as CountPolicy>::Count, NonNull, Agg>
where
    V: SQLParam + 'a,
    V::DialectMarker: CountPolicy,
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
/// Result type is dialect-aware.
/// Returns a nullable expression (empty set returns NULL).
///
/// # Type Safety
///
/// ```ignore
/// // ✅ OK: Numeric column
/// sum(orders.amount);
/// // SQLite: same width for integer sums
/// // PostgreSQL: Int/SmallInt promote to BigInt
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
/// Result type is dialect-aware.
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
pub fn stddev_pop<'a, V, E>(
    expr: E,
) -> SQLExpr<
    'a,
    V,
    <E::SQLType as StatisticalAggregatePolicy<V::DialectMarker>>::StddevPop,
    Null,
    Agg,
>
where
    V: SQLParam + 'a,
    E: Expr<'a, V>,
    E::SQLType: StatisticalAggregatePolicy<V::DialectMarker>,
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
pub fn stddev_samp<'a, V, E>(
    expr: E,
) -> SQLExpr<
    'a,
    V,
    <E::SQLType as StatisticalAggregatePolicy<V::DialectMarker>>::StddevSamp,
    Null,
    Agg,
>
where
    V: SQLParam + 'a,
    E: Expr<'a, V>,
    E::SQLType: StatisticalAggregatePolicy<V::DialectMarker>,
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
pub fn var_pop<'a, V, E>(
    expr: E,
) -> SQLExpr<'a, V, <E::SQLType as StatisticalAggregatePolicy<V::DialectMarker>>::VarPop, Null, Agg>
where
    V: SQLParam + 'a,
    E: Expr<'a, V>,
    E::SQLType: StatisticalAggregatePolicy<V::DialectMarker>,
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
pub fn var_samp<'a, V, E>(
    expr: E,
) -> SQLExpr<'a, V, <E::SQLType as StatisticalAggregatePolicy<V::DialectMarker>>::VarSamp, Null, Agg>
where
    V: SQLParam + 'a,
    E: Expr<'a, V>,
    E::SQLType: StatisticalAggregatePolicy<V::DialectMarker>,
{
    SQLExpr::new(SQL::func("VAR_SAMP", expr.into_sql()))
}

/// VARIANCE - PostgreSQL alias for sample variance (VAR_SAMP).
pub fn variance<'a, V, E>(
    expr: E,
) -> SQLExpr<'a, V, <E::SQLType as StatisticalAggregatePolicy<V::DialectMarker>>::VarSamp, Null, Agg>
where
    V: SQLParam + 'a,
    E: Expr<'a, V>,
    E::SQLType: StatisticalAggregatePolicy<V::DialectMarker>,
{
    SQLExpr::new(SQL::func("VARIANCE", expr.into_sql()))
}

/// BOOL_AND - true if all non-null inputs are true (PostgreSQL).
pub fn bool_and<'a, V, E>(expr: E) -> SQLExpr<'a, V, Bool, Null, Agg>
where
    V: SQLParam + 'a,
    V::DialectMarker: PostgresAggregateSupport,
    E: Expr<'a, V>,
    E::SQLType: BooleanAggregatePolicy<V::DialectMarker>,
{
    SQLExpr::new(SQL::func("BOOL_AND", expr.into_sql()))
}

/// BOOL_OR - true if any non-null input is true (PostgreSQL).
pub fn bool_or<'a, V, E>(expr: E) -> SQLExpr<'a, V, Bool, Null, Agg>
where
    V: SQLParam + 'a,
    V::DialectMarker: PostgresAggregateSupport,
    E: Expr<'a, V>,
    E::SQLType: BooleanAggregatePolicy<V::DialectMarker>,
{
    SQLExpr::new(SQL::func("BOOL_OR", expr.into_sql()))
}

/// JSON_AGG - aggregates values into a JSON array (PostgreSQL).
pub fn json_agg<'a, V, E>(expr: E) -> SQLExpr<'a, V, Json, Null, Agg>
where
    V: SQLParam + 'a,
    V::DialectMarker: PostgresAggregateSupport,
    E: Expr<'a, V>,
{
    SQLExpr::new(SQL::func("JSON_AGG", expr.into_sql()))
}

/// JSONB_AGG - aggregates values into a JSONB array (PostgreSQL).
pub fn jsonb_agg<'a, V, E>(expr: E) -> SQLExpr<'a, V, Jsonb, Null, Agg>
where
    V: SQLParam + 'a,
    V::DialectMarker: PostgresAggregateSupport,
    E: Expr<'a, V>,
{
    SQLExpr::new(SQL::func("JSONB_AGG", expr.into_sql()))
}

/// ARRAY_AGG - aggregates values into a SQL array (PostgreSQL).
pub fn array_agg<'a, V, E>(expr: E) -> SQLExpr<'a, V, Array<E::SQLType>, Null, Agg>
where
    V: SQLParam + 'a,
    V::DialectMarker: PostgresAggregateSupport,
    E: Expr<'a, V>,
{
    SQLExpr::new(SQL::func("ARRAY_AGG", expr.into_sql()))
}

// =============================================================================
// GROUP_CONCAT / STRING_AGG
// =============================================================================

/// GROUP_CONCAT - concatenates values into a string (SQLite).
///
/// Returns Text type, nullable.
pub fn group_concat<'a, V, E>(expr: E) -> SQLExpr<'a, V, crate::types::Text, Null, Agg>
where
    V: SQLParam + 'a,
    V::DialectMarker: SQLiteAggregateSupport,
    E: Expr<'a, V>,
{
    SQLExpr::new(SQL::func("GROUP_CONCAT", expr.into_sql()))
}

/// STRING_AGG - concatenates text values using a delimiter (PostgreSQL).
pub fn string_agg<'a, V, E, D>(expr: E, delimiter: D) -> SQLExpr<'a, V, Text, Null, Agg>
where
    V: SQLParam + 'a,
    V::DialectMarker: PostgresAggregateSupport,
    E: Expr<'a, V>,
    E::SQLType: crate::types::Textual,
    D: Expr<'a, V>,
    D::SQLType: crate::types::Textual,
{
    SQLExpr::new(SQL::func(
        "STRING_AGG",
        expr.into_sql()
            .push(crate::Token::COMMA)
            .append(delimiter.into_sql()),
    ))
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

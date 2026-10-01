//! Aggregate functions: `COUNT`, `SUM`, `AVG`, `MIN`, `MAX` and friends.
//!
//! Every function here returns an aggregate expression ([`Agg`]). Query
//! builders use that to reject a SELECT list that mixes aggregates and plain
//! columns without a matching `GROUP BY`. Calling `.over(...)` on an aggregate
//! turns it into a window function (see [`window`](super::window)).
//!
//! # Type safety
//!
//! - [`count`], [`min`] and [`max`] accept any expression.
//! - [`sum`], [`avg`] and the statistical functions need a numeric argument.
//!   Their result type depends on the dialect (see [`AggregatePolicy`]).
//! - Functions that exist on only some databases (`TOTAL`, `GROUP_CONCAT`,
//!   `STRING_AGG`, `BOOL_AND`, ...) do not compile for the others.
//!
//! Except for `COUNT` and `TOTAL`, aggregates are nullable: they return NULL
//! for an empty group.

use crate::dialect::{Dialect, DialectTypes};
use crate::dialect::{DialectSupports, feature};
use crate::sql::SQL;
use crate::traits::SQLParam;
use crate::types::{Array, Numeric};
use crate::{MySQLDialect, PostgresDialect, SQLiteDialect};
use drizzle_types::mysql::types::{
    BigInt as MyBigInt, BigIntUnsigned as MyBigIntUnsigned, Decimal as MyDecimal,
    Double as MyDouble, Float as MyFloat, Int as MyInt, IntUnsigned as MyIntUnsigned,
    MediumInt as MyMediumInt, MediumIntUnsigned as MyMediumIntUnsigned, SmallInt as MySmallInt,
    SmallIntUnsigned as MySmallIntUnsigned, TinyInt as MyTinyInt,
    TinyIntUnsigned as MyTinyIntUnsigned, Year as MyYear,
};
use drizzle_types::postgres::types::{
    Boolean as PgBoolean, Float4, Float8, Int2, Int4, Int8, Numeric as PgNumeric,
};
use drizzle_types::sqlite::types::{
    Integer as SqliteInteger, Numeric as SqliteNumeric, Real as SqliteReal,
};

use super::ExprSources;
use super::math::pg_double;
use super::{Agg, Expr, NonNull, Null, SQLExpr, Scalar};
use crate::scope::ScopeOnly;

// =============================================================================
// Dialect Aggregate Policy
// =============================================================================

/// Result types of [`sum`] and [`avg`] for a numeric SQL type on dialect `D`.
///
/// | Dialect | Input | `SUM` | `AVG` |
/// |---|---|---|---|
/// | SQLite | `INTEGER` | `INTEGER` | `REAL` |
/// | SQLite | `REAL` | `REAL` | `REAL` |
/// | PostgreSQL | `int2`, `int4` | `int8` | `float8` |
/// | PostgreSQL | `int8` | `int8` | `float8` |
/// | PostgreSQL | `float4`, `float8` | `float8` | `float8` |
/// | PostgreSQL | `numeric` | `numeric` | `numeric` |
/// | MySQL | integer types, `DECIMAL` | `DECIMAL` | `DECIMAL` |
/// | MySQL | `FLOAT`, `DOUBLE` | `DOUBLE` | `DOUBLE` |
#[diagnostic::on_unimplemented(
    message = "no aggregate policy for `{Self}` on this dialect",
    label = "aggregate result type is not defined for this SQL type/dialect"
)]
pub trait AggregatePolicy<D>: Numeric {
    /// Result type of `SUM`.
    type Sum: crate::types::DataType;
    /// Result type of `AVG`.
    type Avg: crate::types::DataType;
}

/// Result types of the standard deviation and variance aggregates for a
/// numeric SQL type on dialect `D`.
///
/// On PostgreSQL every result is `float8`; on MySQL it is `DOUBLE`. SQLite
/// has no built-in statistical aggregates, so it does not implement this
/// trait.
#[diagnostic::on_unimplemented(
    message = "no statistical aggregate policy for `{Self}` on this dialect",
    label = "stddev/variance result type is not defined for this SQL type/dialect"
)]
pub trait StatisticalAggregatePolicy<D>: Numeric {
    /// Result type of `STDDEV_POP`.
    type StddevPop: crate::types::DataType;
    /// Result type of `STDDEV_SAMP`.
    type StddevSamp: crate::types::DataType;
    /// Result type of `VAR_POP`.
    type VarPop: crate::types::DataType;
    /// Result type of `VAR_SAMP` / `VARIANCE`.
    type VarSamp: crate::types::DataType;
}

/// SQL types that `BOOL_AND`, `BOOL_OR` and `EVERY` accept on dialect `D`.
///
/// Only PostgreSQL's `boolean` implements it.
#[diagnostic::on_unimplemented(
    message = "boolean aggregates are not supported for `{Self}` on this dialect",
    label = "use a boolean expression with a dialect that supports BOOL_AND/BOOL_OR"
)]
pub trait BooleanAggregatePolicy<D>: crate::types::DataType {}

mod count_arg_private {
    use super::SQLParam;

    pub trait Sealed<'a, V: SQLParam> {}

    impl<'a, V: SQLParam> Sealed<'a, V> for () {}

    impl<'a, V, E> Sealed<'a, V> for E
    where
        V: SQLParam + 'a,
        E: crate::traits::ToSQL<'a, V> + crate::row::ExprValueType,
    {
    }
}

/// Argument accepted by [`count`]: `()` for `COUNT(*)`, or an expression.
///
/// Sealed. It lets `count(())` work without making `()` a general SQL
/// expression.
#[doc(hidden)]
pub trait CountArg<'a, V: SQLParam>: count_arg_private::Sealed<'a, V> + ExprSources {
    /// Renders the `COUNT(...)` call.
    fn count_sql(self) -> SQL<'a, V>;
}

impl<'a, V: SQLParam + 'a> CountArg<'a, V> for () {
    fn count_sql(self) -> SQL<'a, V> {
        SQL::raw("COUNT(*)")
    }
}

impl<'a, V, E> CountArg<'a, V> for E
where
    V: SQLParam + 'a,
    E: crate::traits::ToSQL<'a, V> + crate::row::ExprValueType + ExprSources,
{
    fn count_sql(self) -> SQL<'a, V> {
        SQL::func("COUNT", self.into_sql().parens_if_subquery())
    }
}

macro_rules! mysql_aggregate_policy {
    ($output:ty; $($ty:ty),+ $(,)?) => {
        $(
            impl AggregatePolicy<MySQLDialect> for $ty {
                type Sum = $output;
                type Avg = $output;
            }
        )+
    };
}

mysql_aggregate_policy!(MyDecimal;
    MyTinyInt,
    MyTinyIntUnsigned,
    MySmallInt,
    MySmallIntUnsigned,
    MyMediumInt,
    MyMediumIntUnsigned,
    MyInt,
    MyIntUnsigned,
    MyBigInt,
    MyBigIntUnsigned,
    MyYear,
    MyDecimal,
);

mysql_aggregate_policy!(MyDouble; MyFloat, MyDouble);

macro_rules! mysql_statistical_aggregate_policy {
    ($($ty:ty),+ $(,)?) => {
        $(
            impl StatisticalAggregatePolicy<MySQLDialect> for $ty {
                type StddevPop = MyDouble;
                type StddevSamp = MyDouble;
                type VarPop = MyDouble;
                type VarSamp = MyDouble;
            }
        )+
    };
}

mysql_statistical_aggregate_policy!(
    MyTinyInt,
    MyTinyIntUnsigned,
    MySmallInt,
    MySmallIntUnsigned,
    MyMediumInt,
    MyMediumIntUnsigned,
    MyInt,
    MyIntUnsigned,
    MyBigInt,
    MyBigIntUnsigned,
    MyYear,
    MyDecimal,
    MyFloat,
    MyDouble,
);

impl AggregatePolicy<SQLiteDialect> for SqliteInteger {
    type Sum = Self;
    type Avg = SqliteReal;
}
impl AggregatePolicy<SQLiteDialect> for SqliteReal {
    type Sum = Self;
    type Avg = Self;
}
impl AggregatePolicy<SQLiteDialect> for SqliteNumeric {
    type Sum = Self;
    type Avg = SqliteReal;
}
impl AggregatePolicy<SQLiteDialect> for drizzle_types::sqlite::types::Any {
    type Sum = Self;
    type Avg = SqliteReal;
}

impl StatisticalAggregatePolicy<PostgresDialect> for Int2 {
    type StddevPop = Float8;
    type StddevSamp = Float8;
    type VarPop = Float8;
    type VarSamp = Float8;
}
impl StatisticalAggregatePolicy<PostgresDialect> for Int4 {
    type StddevPop = Float8;
    type StddevSamp = Float8;
    type VarPop = Float8;
    type VarSamp = Float8;
}
impl StatisticalAggregatePolicy<PostgresDialect> for Int8 {
    type StddevPop = Float8;
    type StddevSamp = Float8;
    type VarPop = Float8;
    type VarSamp = Float8;
}
impl StatisticalAggregatePolicy<PostgresDialect> for Float4 {
    type StddevPop = Float8;
    type StddevSamp = Float8;
    type VarPop = Float8;
    type VarSamp = Float8;
}
impl StatisticalAggregatePolicy<PostgresDialect> for Float8 {
    type StddevPop = Self;
    type StddevSamp = Self;
    type VarPop = Self;
    type VarSamp = Self;
}
impl StatisticalAggregatePolicy<PostgresDialect> for PgNumeric {
    type StddevPop = Float8;
    type StddevSamp = Float8;
    type VarPop = Float8;
    type VarSamp = Float8;
}

impl BooleanAggregatePolicy<PostgresDialect> for PgBoolean {}

impl DialectSupports<feature::PostgresAggregate> for PostgresDialect {}
impl DialectSupports<feature::SQLiteAggregate> for SQLiteDialect {}
impl DialectSupports<feature::GroupConcat> for SQLiteDialect {}
impl DialectSupports<feature::GroupConcat> for MySQLDialect {}

impl AggregatePolicy<PostgresDialect> for Int2 {
    type Sum = Int8;
    type Avg = Float8;
}
impl AggregatePolicy<PostgresDialect> for Int4 {
    type Sum = Int8;
    type Avg = Float8;
}
impl AggregatePolicy<PostgresDialect> for Int8 {
    type Sum = Self;
    type Avg = Float8;
}
impl AggregatePolicy<PostgresDialect> for Float4 {
    type Sum = Float8;
    type Avg = Float8;
}
impl AggregatePolicy<PostgresDialect> for Float8 {
    type Sum = Self;
    type Avg = Self;
}
impl AggregatePolicy<PostgresDialect> for PgNumeric {
    type Sum = Self;
    type Avg = Self;
}

// =============================================================================
// COUNT
// =============================================================================

/// Row or value count (`COUNT`).
///
/// `count(())` renders `COUNT(*)` and counts rows. `count(expr)` renders
/// `COUNT(expr)` and counts non-NULL values. The result is the dialect's
/// big-integer type, never NULL, and an aggregate.
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
/// assert_eq!(count::<Value, _>(()).sql(), "COUNT(*)");
/// assert_eq!(count(users.email).sql(), r#"COUNT ("users"."email")"#);
/// ```
pub fn count<'a, V, A>(
    arg: A,
) -> SQLExpr<'a, V, <V::DialectMarker as DialectTypes>::BigInt, NonNull, Agg, ScopeOnly<A::Sources>>
where
    V: SQLParam + 'a,
    A: CountArg<'a, V>,
{
    SQLExpr::new(arg.count_sql())
}

/// Count of distinct non-NULL values (`COUNT(DISTINCT expr)`).
///
/// Accepts any expression. The result is the dialect's big-integer type,
/// never NULL, and an aggregate.
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
/// let names = count_distinct(users.name);
/// assert_eq!(names.sql(), r#"COUNT (DISTINCT "users"."name")"#);
/// ```
pub fn count_distinct<'a, V, E>(
    expr: E,
) -> SQLExpr<'a, V, <V::DialectMarker as DialectTypes>::BigInt, NonNull, Agg, ScopeOnly<E::Sources>>
where
    V: SQLParam + 'a,
    E: Expr<'a, V>,
{
    SQLExpr::new(SQL::func(
        "COUNT",
        SQL::raw("DISTINCT").append(expr.into_expr_sql()),
    ))
}

// =============================================================================
// SUM
// =============================================================================

/// Sum of numeric values (`SUM`).
///
/// The argument must be numeric. The result type depends on the dialect (see
/// [`AggregatePolicy`]): for example, PostgreSQL widens `int4` to `int8`. The
/// result is nullable (`SUM` of no rows is NULL) and an aggregate.
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
/// assert_eq!(sum(users.age).sql(), r#"SUM ("users"."age")"#);
/// ```
///
/// # Type safety
///
/// Summing a text column does not compile:
///
/// ```rust,compile_fail
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
/// let wrong = sum(users.name);
/// ```
#[allow(clippy::type_complexity)]
pub fn sum<'a, V, E>(
    expr: E,
) -> SQLExpr<'a, V, <E::SQLType as AggregatePolicy<V::DialectMarker>>::Sum, Null, Agg, E::Sources>
where
    V: SQLParam + 'a,
    E: Expr<'a, V>,
    E::SQLType: AggregatePolicy<V::DialectMarker>,
{
    SQLExpr::new(SQL::func("SUM", expr.into_expr_sql()))
}

/// Sum of distinct numeric values (`SUM(DISTINCT expr)`).
///
/// Same typing as [`sum`].
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
/// assert_eq!(sum_distinct(users.age).sql(), r#"SUM (DISTINCT "users"."age")"#);
/// ```
#[allow(clippy::type_complexity)]
pub fn sum_distinct<'a, V, E>(
    expr: E,
) -> SQLExpr<'a, V, <E::SQLType as AggregatePolicy<V::DialectMarker>>::Sum, Null, Agg, E::Sources>
where
    V: SQLParam + 'a,
    E: Expr<'a, V>,
    E::SQLType: AggregatePolicy<V::DialectMarker>,
{
    SQLExpr::new(SQL::func(
        "SUM",
        SQL::raw("DISTINCT").append(expr.into_expr_sql()),
    ))
}

// =============================================================================
// AVG
// =============================================================================

/// Average of numeric values (`AVG`).
///
/// The argument must be numeric. The result type depends on the dialect (see
/// [`AggregatePolicy`]): SQLite returns `REAL`, PostgreSQL `float8` (or
/// `numeric` for `numeric` input), MySQL `DECIMAL` for integers. The result
/// is nullable (`AVG` of no rows is NULL) and an aggregate.
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
/// assert_eq!(avg(users.age).sql(), r#"AVG ("users"."age")"#);
/// ```
///
/// # Type safety
///
/// Averaging a text column does not compile:
///
/// ```rust,compile_fail
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
/// let wrong = avg(users.name);
/// ```
#[allow(clippy::type_complexity)]
pub fn avg<'a, V, E>(
    expr: E,
) -> SQLExpr<'a, V, <E::SQLType as AggregatePolicy<V::DialectMarker>>::Avg, Null, Agg, E::Sources>
where
    V: SQLParam + 'a,
    E: Expr<'a, V>,
    E::SQLType: AggregatePolicy<V::DialectMarker>,
{
    SQLExpr::new(SQL::func("AVG", expr.into_expr_sql()))
}

/// Average of distinct numeric values (`AVG(DISTINCT expr)`).
///
/// Same typing as [`avg`].
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
/// assert_eq!(avg_distinct(users.age).sql(), r#"AVG (DISTINCT "users"."age")"#);
/// ```
#[allow(clippy::type_complexity)]
pub fn avg_distinct<'a, V, E>(
    expr: E,
) -> SQLExpr<'a, V, <E::SQLType as AggregatePolicy<V::DialectMarker>>::Avg, Null, Agg, E::Sources>
where
    V: SQLParam + 'a,
    E: Expr<'a, V>,
    E::SQLType: AggregatePolicy<V::DialectMarker>,
{
    SQLExpr::new(SQL::func(
        "AVG",
        SQL::raw("DISTINCT").append(expr.into_expr_sql()),
    ))
}

// =============================================================================
// MIN / MAX
// =============================================================================

/// Smallest value (`MIN`).
///
/// Accepts any expression. The result has the argument's SQL type, is
/// nullable (`MIN` of no rows is NULL), and is an aggregate.
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
/// assert_eq!(min(users.age).sql(), r#"MIN ("users"."age")"#);
/// ```
pub fn min<'a, V, E>(expr: E) -> SQLExpr<'a, V, E::SQLType, Null, Agg, E::Sources>
where
    V: SQLParam + 'a,
    E: Expr<'a, V>,
{
    SQLExpr::new(SQL::func("MIN", expr.into_expr_sql()))
}

/// Largest value (`MAX`).
///
/// Accepts any expression. The result has the argument's SQL type, is
/// nullable (`MAX` of no rows is NULL), and is an aggregate.
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
/// assert_eq!(max(users.age).sql(), r#"MAX ("users"."age")"#);
/// ```
pub fn max<'a, V, E>(expr: E) -> SQLExpr<'a, V, E::SQLType, Null, Agg, E::Sources>
where
    V: SQLParam + 'a,
    E: Expr<'a, V>,
{
    SQLExpr::new(SQL::func("MAX", expr.into_expr_sql()))
}

// =============================================================================
// STATISTICAL FUNCTIONS
// =============================================================================

/// Population standard deviation (`STDDEV_POP`), on PostgreSQL and MySQL.
///
/// The argument must be numeric. The result type comes from
/// [`StatisticalAggregatePolicy`] (`float8` on PostgreSQL, `DOUBLE` on
/// MySQL); on PostgreSQL the call is wrapped in
/// `CAST(... AS DOUBLE PRECISION)`. The result is nullable and an aggregate.
/// SQLite has no `STDDEV_POP`, so this does not compile for SQLite.
///
/// # Examples
///
/// ```rust
/// # use drizzle_core::dialect::{Dialect, DialectTypes, PostgresDialect as D};
/// # use drizzle_core::{ColumnRef, SQL, SQLParam, expr::*};
/// # #[derive(Clone, Debug)] struct Value(String);
/// # impl SQLParam for Value { const DIALECT: Dialect = Dialect::PostgreSQL; type DialectMarker = D; }
/// # impl<X: ToString> From<X> for Value { fn from(v: X) -> Self { Value(v.to_string()) } }
/// # impl From<Value> for std::borrow::Cow<'_, Value> { fn from(v: Value) -> Self { Self::Owned(v) } }
/// # type C<X, N = NonNull> = &'static SQLExpr<'static, Value, X, N>;
/// # fn col<X: drizzle_core::types::DataType, N: Nullability>(c: &'static str) -> C<X, N> { Box::leak(Box::new(SQLExpr::new(SQL::column(ColumnRef::sql("users", c))))) }
/// # type Int = <D as DialectTypes>::Int; type Text = <D as DialectTypes>::Text; type Real = <D as DialectTypes>::Double;
/// # struct Users { id: C<Int>, age: C<Int>, name: C<Text>, email: C<Text, Null>, score: C<Real, Null>, active: C<<D as DialectTypes>::Bool>, created_at: C<<D as DialectTypes>::Timestamp> }
/// # let users = Users { id: col("id"), age: col("age"), name: col("name"), email: col("email"), score: col("score"), active: col("active"), created_at: col("created_at") };
/// assert_eq!(
///     stddev_pop(users.age).sql(),
///     r#"CAST (STDDEV_POP ("users"."age") AS DOUBLE PRECISION)"#
/// );
/// ```
#[allow(clippy::type_complexity)]
pub fn stddev_pop<'a, V, E>(
    expr: E,
) -> SQLExpr<
    'a,
    V,
    <E::SQLType as StatisticalAggregatePolicy<V::DialectMarker>>::StddevPop,
    Null,
    Agg,
    E::Sources,
>
where
    V: SQLParam + 'a,
    E: Expr<'a, V>,
    E::SQLType: StatisticalAggregatePolicy<V::DialectMarker>,
{
    SQLExpr::new(pg_double(SQL::func("STDDEV_POP", expr.into_expr_sql())))
}

/// Sample standard deviation (`STDDEV_SAMP`), on PostgreSQL and MySQL.
///
/// The argument must be numeric. The result type comes from
/// [`StatisticalAggregatePolicy`] (`float8` on PostgreSQL, `DOUBLE` on
/// MySQL); on PostgreSQL the call is wrapped in
/// `CAST(... AS DOUBLE PRECISION)`. The result is nullable and an aggregate.
/// SQLite has no `STDDEV_SAMP`, so this does not compile for SQLite.
///
/// # Examples
///
/// ```rust
/// # use drizzle_core::dialect::{Dialect, DialectTypes, PostgresDialect as D};
/// # use drizzle_core::{ColumnRef, SQL, SQLParam, expr::*};
/// # #[derive(Clone, Debug)] struct Value(String);
/// # impl SQLParam for Value { const DIALECT: Dialect = Dialect::PostgreSQL; type DialectMarker = D; }
/// # impl<X: ToString> From<X> for Value { fn from(v: X) -> Self { Value(v.to_string()) } }
/// # impl From<Value> for std::borrow::Cow<'_, Value> { fn from(v: Value) -> Self { Self::Owned(v) } }
/// # type C<X, N = NonNull> = &'static SQLExpr<'static, Value, X, N>;
/// # fn col<X: drizzle_core::types::DataType, N: Nullability>(c: &'static str) -> C<X, N> { Box::leak(Box::new(SQLExpr::new(SQL::column(ColumnRef::sql("users", c))))) }
/// # type Int = <D as DialectTypes>::Int; type Text = <D as DialectTypes>::Text; type Real = <D as DialectTypes>::Double;
/// # struct Users { id: C<Int>, age: C<Int>, name: C<Text>, email: C<Text, Null>, score: C<Real, Null>, active: C<<D as DialectTypes>::Bool>, created_at: C<<D as DialectTypes>::Timestamp> }
/// # let users = Users { id: col("id"), age: col("age"), name: col("name"), email: col("email"), score: col("score"), active: col("active"), created_at: col("created_at") };
/// assert_eq!(
///     stddev_samp(users.age).sql(),
///     r#"CAST (STDDEV_SAMP ("users"."age") AS DOUBLE PRECISION)"#
/// );
/// ```
#[allow(clippy::type_complexity)]
pub fn stddev_samp<'a, V, E>(
    expr: E,
) -> SQLExpr<
    'a,
    V,
    <E::SQLType as StatisticalAggregatePolicy<V::DialectMarker>>::StddevSamp,
    Null,
    Agg,
    E::Sources,
>
where
    V: SQLParam + 'a,
    E: Expr<'a, V>,
    E::SQLType: StatisticalAggregatePolicy<V::DialectMarker>,
{
    SQLExpr::new(pg_double(SQL::func("STDDEV_SAMP", expr.into_expr_sql())))
}

/// Population variance (`VAR_POP`), on PostgreSQL and MySQL.
///
/// The argument must be numeric. The result type comes from
/// [`StatisticalAggregatePolicy`] (`float8` on PostgreSQL, `DOUBLE` on
/// MySQL); on PostgreSQL the call is wrapped in
/// `CAST(... AS DOUBLE PRECISION)`. The result is nullable and an aggregate.
/// SQLite has no `VAR_POP`, so this does not compile for SQLite.
///
/// # Examples
///
/// ```rust
/// # use drizzle_core::dialect::{Dialect, DialectTypes, PostgresDialect as D};
/// # use drizzle_core::{ColumnRef, SQL, SQLParam, expr::*};
/// # #[derive(Clone, Debug)] struct Value(String);
/// # impl SQLParam for Value { const DIALECT: Dialect = Dialect::PostgreSQL; type DialectMarker = D; }
/// # impl<X: ToString> From<X> for Value { fn from(v: X) -> Self { Value(v.to_string()) } }
/// # impl From<Value> for std::borrow::Cow<'_, Value> { fn from(v: Value) -> Self { Self::Owned(v) } }
/// # type C<X, N = NonNull> = &'static SQLExpr<'static, Value, X, N>;
/// # fn col<X: drizzle_core::types::DataType, N: Nullability>(c: &'static str) -> C<X, N> { Box::leak(Box::new(SQLExpr::new(SQL::column(ColumnRef::sql("users", c))))) }
/// # type Int = <D as DialectTypes>::Int; type Text = <D as DialectTypes>::Text; type Real = <D as DialectTypes>::Double;
/// # struct Users { id: C<Int>, age: C<Int>, name: C<Text>, email: C<Text, Null>, score: C<Real, Null>, active: C<<D as DialectTypes>::Bool>, created_at: C<<D as DialectTypes>::Timestamp> }
/// # let users = Users { id: col("id"), age: col("age"), name: col("name"), email: col("email"), score: col("score"), active: col("active"), created_at: col("created_at") };
/// assert_eq!(
///     var_pop(users.age).sql(),
///     r#"CAST (VAR_POP ("users"."age") AS DOUBLE PRECISION)"#
/// );
/// ```
#[allow(clippy::type_complexity)]
pub fn var_pop<'a, V, E>(
    expr: E,
) -> SQLExpr<
    'a,
    V,
    <E::SQLType as StatisticalAggregatePolicy<V::DialectMarker>>::VarPop,
    Null,
    Agg,
    E::Sources,
>
where
    V: SQLParam + 'a,
    E: Expr<'a, V>,
    E::SQLType: StatisticalAggregatePolicy<V::DialectMarker>,
{
    SQLExpr::new(pg_double(SQL::func("VAR_POP", expr.into_expr_sql())))
}

/// Sample variance (`VAR_SAMP`), on PostgreSQL and MySQL.
///
/// The argument must be numeric. The result type comes from
/// [`StatisticalAggregatePolicy`] (`float8` on PostgreSQL, `DOUBLE` on
/// MySQL); on PostgreSQL the call is wrapped in
/// `CAST(... AS DOUBLE PRECISION)`. The result is nullable and an aggregate.
/// SQLite has no `VAR_SAMP`, so this does not compile for SQLite.
///
/// # Examples
///
/// ```rust
/// # use drizzle_core::dialect::{Dialect, DialectTypes, PostgresDialect as D};
/// # use drizzle_core::{ColumnRef, SQL, SQLParam, expr::*};
/// # #[derive(Clone, Debug)] struct Value(String);
/// # impl SQLParam for Value { const DIALECT: Dialect = Dialect::PostgreSQL; type DialectMarker = D; }
/// # impl<X: ToString> From<X> for Value { fn from(v: X) -> Self { Value(v.to_string()) } }
/// # impl From<Value> for std::borrow::Cow<'_, Value> { fn from(v: Value) -> Self { Self::Owned(v) } }
/// # type C<X, N = NonNull> = &'static SQLExpr<'static, Value, X, N>;
/// # fn col<X: drizzle_core::types::DataType, N: Nullability>(c: &'static str) -> C<X, N> { Box::leak(Box::new(SQLExpr::new(SQL::column(ColumnRef::sql("users", c))))) }
/// # type Int = <D as DialectTypes>::Int; type Text = <D as DialectTypes>::Text; type Real = <D as DialectTypes>::Double;
/// # struct Users { id: C<Int>, age: C<Int>, name: C<Text>, email: C<Text, Null>, score: C<Real, Null>, active: C<<D as DialectTypes>::Bool>, created_at: C<<D as DialectTypes>::Timestamp> }
/// # let users = Users { id: col("id"), age: col("age"), name: col("name"), email: col("email"), score: col("score"), active: col("active"), created_at: col("created_at") };
/// assert_eq!(
///     var_samp(users.age).sql(),
///     r#"CAST (VAR_SAMP ("users"."age") AS DOUBLE PRECISION)"#
/// );
/// ```
#[allow(clippy::type_complexity)]
pub fn var_samp<'a, V, E>(
    expr: E,
) -> SQLExpr<
    'a,
    V,
    <E::SQLType as StatisticalAggregatePolicy<V::DialectMarker>>::VarSamp,
    Null,
    Agg,
    E::Sources,
>
where
    V: SQLParam + 'a,
    E: Expr<'a, V>,
    E::SQLType: StatisticalAggregatePolicy<V::DialectMarker>,
{
    SQLExpr::new(pg_double(SQL::func("VAR_SAMP", expr.into_expr_sql())))
}

/// Sample variance: `VARIANCE` on PostgreSQL, `VAR_SAMP` on MySQL.
///
/// Same typing as [`var_samp`].
///
/// # Examples
///
/// ```rust
/// # use drizzle_core::dialect::{Dialect, DialectTypes, PostgresDialect as D};
/// # use drizzle_core::{ColumnRef, SQL, SQLParam, expr::*};
/// # #[derive(Clone, Debug)] struct Value(String);
/// # impl SQLParam for Value { const DIALECT: Dialect = Dialect::PostgreSQL; type DialectMarker = D; }
/// # impl<X: ToString> From<X> for Value { fn from(v: X) -> Self { Value(v.to_string()) } }
/// # impl From<Value> for std::borrow::Cow<'_, Value> { fn from(v: Value) -> Self { Self::Owned(v) } }
/// # type C<X, N = NonNull> = &'static SQLExpr<'static, Value, X, N>;
/// # fn col<X: drizzle_core::types::DataType, N: Nullability>(c: &'static str) -> C<X, N> { Box::leak(Box::new(SQLExpr::new(SQL::column(ColumnRef::sql("users", c))))) }
/// # type Int = <D as DialectTypes>::Int; type Text = <D as DialectTypes>::Text; type Real = <D as DialectTypes>::Double;
/// # struct Users { id: C<Int>, age: C<Int>, name: C<Text>, email: C<Text, Null>, score: C<Real, Null>, active: C<<D as DialectTypes>::Bool>, created_at: C<<D as DialectTypes>::Timestamp> }
/// # let users = Users { id: col("id"), age: col("age"), name: col("name"), email: col("email"), score: col("score"), active: col("active"), created_at: col("created_at") };
/// assert_eq!(
///     variance(users.age).sql(),
///     r#"CAST (VARIANCE ("users"."age") AS DOUBLE PRECISION)"#
/// );
/// ```
#[allow(clippy::type_complexity)]
pub fn variance<'a, V, E>(
    expr: E,
) -> SQLExpr<
    'a,
    V,
    <E::SQLType as StatisticalAggregatePolicy<V::DialectMarker>>::VarSamp,
    Null,
    Agg,
    E::Sources,
>
where
    V: SQLParam + 'a,
    E: Expr<'a, V>,
    E::SQLType: StatisticalAggregatePolicy<V::DialectMarker>,
{
    SQLExpr::new(pg_double(SQL::func(
        match V::DIALECT {
            Dialect::MySQL => "VAR_SAMP",
            Dialect::SQLite | Dialect::PostgreSQL => "VARIANCE",
        },
        expr.into_expr_sql(),
    )))
}

/// True when every non-NULL input is true (`BOOL_AND`), on PostgreSQL.
///
/// The argument must be `boolean`. The result is the dialect's boolean,
/// nullable, and an aggregate.
///
/// # Examples
///
/// ```rust
/// # use drizzle_core::dialect::{Dialect, DialectTypes, PostgresDialect as D};
/// # use drizzle_core::{ColumnRef, SQL, SQLParam, expr::*};
/// # #[derive(Clone, Debug)] struct Value(String);
/// # impl SQLParam for Value { const DIALECT: Dialect = Dialect::PostgreSQL; type DialectMarker = D; }
/// # impl<X: ToString> From<X> for Value { fn from(v: X) -> Self { Value(v.to_string()) } }
/// # impl From<Value> for std::borrow::Cow<'_, Value> { fn from(v: Value) -> Self { Self::Owned(v) } }
/// # type C<X, N = NonNull> = &'static SQLExpr<'static, Value, X, N>;
/// # fn col<X: drizzle_core::types::DataType, N: Nullability>(c: &'static str) -> C<X, N> { Box::leak(Box::new(SQLExpr::new(SQL::column(ColumnRef::sql("users", c))))) }
/// # type Int = <D as DialectTypes>::Int; type Text = <D as DialectTypes>::Text; type Real = <D as DialectTypes>::Double;
/// # struct Users { id: C<Int>, age: C<Int>, name: C<Text>, email: C<Text, Null>, score: C<Real, Null>, active: C<<D as DialectTypes>::Bool>, created_at: C<<D as DialectTypes>::Timestamp> }
/// # let users = Users { id: col("id"), age: col("age"), name: col("name"), email: col("email"), score: col("score"), active: col("active"), created_at: col("created_at") };
/// assert_eq!(bool_and(users.active).sql(), r#"BOOL_AND ("users"."active")"#);
/// ```
pub fn bool_and<'a, V, E>(
    expr: E,
) -> SQLExpr<'a, V, <V::DialectMarker as DialectTypes>::Bool, Null, Agg, E::Sources>
where
    V: SQLParam + 'a,
    V::DialectMarker: DialectSupports<feature::PostgresAggregate>,
    E: Expr<'a, V>,
    E::SQLType: BooleanAggregatePolicy<V::DialectMarker>,
{
    SQLExpr::new(SQL::func("BOOL_AND", expr.into_expr_sql()))
}

/// True when any non-NULL input is true (`BOOL_OR`), on PostgreSQL.
///
/// The argument must be `boolean`. The result is the dialect's boolean,
/// nullable, and an aggregate.
///
/// # Examples
///
/// ```rust
/// # use drizzle_core::dialect::{Dialect, DialectTypes, PostgresDialect as D};
/// # use drizzle_core::{ColumnRef, SQL, SQLParam, expr::*};
/// # #[derive(Clone, Debug)] struct Value(String);
/// # impl SQLParam for Value { const DIALECT: Dialect = Dialect::PostgreSQL; type DialectMarker = D; }
/// # impl<X: ToString> From<X> for Value { fn from(v: X) -> Self { Value(v.to_string()) } }
/// # impl From<Value> for std::borrow::Cow<'_, Value> { fn from(v: Value) -> Self { Self::Owned(v) } }
/// # type C<X, N = NonNull> = &'static SQLExpr<'static, Value, X, N>;
/// # fn col<X: drizzle_core::types::DataType, N: Nullability>(c: &'static str) -> C<X, N> { Box::leak(Box::new(SQLExpr::new(SQL::column(ColumnRef::sql("users", c))))) }
/// # type Int = <D as DialectTypes>::Int; type Text = <D as DialectTypes>::Text; type Real = <D as DialectTypes>::Double;
/// # struct Users { id: C<Int>, age: C<Int>, name: C<Text>, email: C<Text, Null>, score: C<Real, Null>, active: C<<D as DialectTypes>::Bool>, created_at: C<<D as DialectTypes>::Timestamp> }
/// # let users = Users { id: col("id"), age: col("age"), name: col("name"), email: col("email"), score: col("score"), active: col("active"), created_at: col("created_at") };
/// assert_eq!(bool_or(users.active).sql(), r#"BOOL_OR ("users"."active")"#);
/// ```
pub fn bool_or<'a, V, E>(
    expr: E,
) -> SQLExpr<'a, V, <V::DialectMarker as DialectTypes>::Bool, Null, Agg, E::Sources>
where
    V: SQLParam + 'a,
    V::DialectMarker: DialectSupports<feature::PostgresAggregate>,
    E: Expr<'a, V>,
    E::SQLType: BooleanAggregatePolicy<V::DialectMarker>,
{
    SQLExpr::new(SQL::func("BOOL_OR", expr.into_expr_sql()))
}

/// Collects values into a JSON array (`JSON_AGG`), on PostgreSQL.
///
/// Accepts any expression. The result is `json`, nullable, and an aggregate.
///
/// # Examples
///
/// ```rust
/// # use drizzle_core::dialect::{Dialect, DialectTypes, PostgresDialect as D};
/// # use drizzle_core::{ColumnRef, SQL, SQLParam, expr::*};
/// # #[derive(Clone, Debug)] struct Value(String);
/// # impl SQLParam for Value { const DIALECT: Dialect = Dialect::PostgreSQL; type DialectMarker = D; }
/// # impl<X: ToString> From<X> for Value { fn from(v: X) -> Self { Value(v.to_string()) } }
/// # impl From<Value> for std::borrow::Cow<'_, Value> { fn from(v: Value) -> Self { Self::Owned(v) } }
/// # type C<X, N = NonNull> = &'static SQLExpr<'static, Value, X, N>;
/// # fn col<X: drizzle_core::types::DataType, N: Nullability>(c: &'static str) -> C<X, N> { Box::leak(Box::new(SQLExpr::new(SQL::column(ColumnRef::sql("users", c))))) }
/// # type Int = <D as DialectTypes>::Int; type Text = <D as DialectTypes>::Text; type Real = <D as DialectTypes>::Double;
/// # struct Users { id: C<Int>, age: C<Int>, name: C<Text>, email: C<Text, Null>, score: C<Real, Null>, active: C<<D as DialectTypes>::Bool>, created_at: C<<D as DialectTypes>::Timestamp> }
/// # let users = Users { id: col("id"), age: col("age"), name: col("name"), email: col("email"), score: col("score"), active: col("active"), created_at: col("created_at") };
/// assert_eq!(json_agg(users.name).sql(), r#"JSON_AGG ("users"."name")"#);
/// ```
pub fn json_agg<'a, V, E>(
    expr: E,
) -> SQLExpr<'a, V, <V::DialectMarker as DialectTypes>::Json, Null, Agg, E::Sources>
where
    V: SQLParam + 'a,
    V::DialectMarker: DialectSupports<feature::PostgresAggregate>,
    E: Expr<'a, V>,
{
    SQLExpr::new(SQL::func("JSON_AGG", expr.into_expr_sql()))
}

/// Collects values into a JSONB array (`JSONB_AGG`), on PostgreSQL.
///
/// Accepts any expression. The result is `jsonb`, nullable, and an aggregate.
///
/// # Examples
///
/// ```rust
/// # use drizzle_core::dialect::{Dialect, DialectTypes, PostgresDialect as D};
/// # use drizzle_core::{ColumnRef, SQL, SQLParam, expr::*};
/// # #[derive(Clone, Debug)] struct Value(String);
/// # impl SQLParam for Value { const DIALECT: Dialect = Dialect::PostgreSQL; type DialectMarker = D; }
/// # impl<X: ToString> From<X> for Value { fn from(v: X) -> Self { Value(v.to_string()) } }
/// # impl From<Value> for std::borrow::Cow<'_, Value> { fn from(v: Value) -> Self { Self::Owned(v) } }
/// # type C<X, N = NonNull> = &'static SQLExpr<'static, Value, X, N>;
/// # fn col<X: drizzle_core::types::DataType, N: Nullability>(c: &'static str) -> C<X, N> { Box::leak(Box::new(SQLExpr::new(SQL::column(ColumnRef::sql("users", c))))) }
/// # type Int = <D as DialectTypes>::Int; type Text = <D as DialectTypes>::Text; type Real = <D as DialectTypes>::Double;
/// # struct Users { id: C<Int>, age: C<Int>, name: C<Text>, email: C<Text, Null>, score: C<Real, Null>, active: C<<D as DialectTypes>::Bool>, created_at: C<<D as DialectTypes>::Timestamp> }
/// # let users = Users { id: col("id"), age: col("age"), name: col("name"), email: col("email"), score: col("score"), active: col("active"), created_at: col("created_at") };
/// assert_eq!(jsonb_agg(users.name).sql(), r#"JSONB_AGG ("users"."name")"#);
/// ```
pub fn jsonb_agg<'a, V, E>(
    expr: E,
) -> SQLExpr<'a, V, <V::DialectMarker as DialectTypes>::Jsonb, Null, Agg, E::Sources>
where
    V: SQLParam + 'a,
    V::DialectMarker: DialectSupports<feature::PostgresAggregate>,
    E: Expr<'a, V>,
{
    SQLExpr::new(SQL::func("JSONB_AGG", expr.into_expr_sql()))
}

/// Collects values into a SQL array (`ARRAY_AGG`), on PostgreSQL.
///
/// Accepts any expression. The result is an array of the argument's SQL type,
/// nullable, and an aggregate.
///
/// # Examples
///
/// ```rust
/// # use drizzle_core::dialect::{Dialect, DialectTypes, PostgresDialect as D};
/// # use drizzle_core::{ColumnRef, SQL, SQLParam, expr::*};
/// # #[derive(Clone, Debug)] struct Value(String);
/// # impl SQLParam for Value { const DIALECT: Dialect = Dialect::PostgreSQL; type DialectMarker = D; }
/// # impl<X: ToString> From<X> for Value { fn from(v: X) -> Self { Value(v.to_string()) } }
/// # impl From<Value> for std::borrow::Cow<'_, Value> { fn from(v: Value) -> Self { Self::Owned(v) } }
/// # type C<X, N = NonNull> = &'static SQLExpr<'static, Value, X, N>;
/// # fn col<X: drizzle_core::types::DataType, N: Nullability>(c: &'static str) -> C<X, N> { Box::leak(Box::new(SQLExpr::new(SQL::column(ColumnRef::sql("users", c))))) }
/// # type Int = <D as DialectTypes>::Int; type Text = <D as DialectTypes>::Text; type Real = <D as DialectTypes>::Double;
/// # struct Users { id: C<Int>, age: C<Int>, name: C<Text>, email: C<Text, Null>, score: C<Real, Null>, active: C<<D as DialectTypes>::Bool>, created_at: C<<D as DialectTypes>::Timestamp> }
/// # let users = Users { id: col("id"), age: col("age"), name: col("name"), email: col("email"), score: col("score"), active: col("active"), created_at: col("created_at") };
/// assert_eq!(array_agg(users.id).sql(), r#"ARRAY_AGG ("users"."id")"#);
/// ```
pub fn array_agg<'a, V, E>(expr: E) -> SQLExpr<'a, V, Array<E::SQLType>, Null, Agg, E::Sources>
where
    V: SQLParam + 'a,
    V::DialectMarker: DialectSupports<feature::PostgresAggregate>,
    E: Expr<'a, V>,
{
    SQLExpr::new(SQL::func("ARRAY_AGG", expr.into_expr_sql()))
}

// =============================================================================
// TOTAL (SQLite)
// =============================================================================

/// Floating-point sum that is never NULL (`TOTAL`), on SQLite.
///
/// The argument must be numeric. Unlike [`sum`], `TOTAL` of no rows is `0.0`,
/// so the result is non-null. It is the dialect's double type and an
/// aggregate.
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
/// assert_eq!(total(users.age).sql(), r#"TOTAL ("users"."age")"#);
/// ```
pub fn total<'a, V, E>(
    expr: E,
) -> SQLExpr<'a, V, <V::DialectMarker as DialectTypes>::Double, NonNull, Agg, ScopeOnly<E::Sources>>
where
    V: SQLParam + 'a,
    V::DialectMarker: DialectSupports<feature::SQLiteAggregate>,
    E: Expr<'a, V>,
    E::SQLType: Numeric,
{
    SQLExpr::new(SQL::func("TOTAL", expr.into_expr_sql()))
}

// =============================================================================
// GROUP_CONCAT / STRING_AGG
// =============================================================================

/// Joins text values with commas (`GROUP_CONCAT`), on SQLite and MySQL.
///
/// The argument must be text. The result is text, nullable, and an aggregate.
/// On PostgreSQL, use [`string_agg`].
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
/// assert_eq!(group_concat(users.name).sql(), r#"GROUP_CONCAT ("users"."name")"#);
/// ```
pub fn group_concat<'a, V, E>(
    expr: E,
) -> SQLExpr<'a, V, <V::DialectMarker as DialectTypes>::Text, Null, Agg, E::Sources>
where
    V: SQLParam + 'a,
    V::DialectMarker: DialectSupports<feature::GroupConcat>,
    E: Expr<'a, V>,
    E::SQLType: crate::types::Textual,
{
    SQLExpr::new(SQL::func("GROUP_CONCAT", expr.into_expr_sql()))
}

/// Joins text values with a delimiter (`STRING_AGG`), on PostgreSQL.
///
/// Both arguments must be text. The result is text, nullable, and an
/// aggregate. On SQLite and MySQL, use [`group_concat`].
///
/// # Examples
///
/// ```rust
/// # use drizzle_core::dialect::{Dialect, DialectTypes, PostgresDialect as D};
/// # use drizzle_core::{ColumnRef, SQL, SQLParam, expr::*};
/// # #[derive(Clone, Debug)] struct Value(String);
/// # impl SQLParam for Value { const DIALECT: Dialect = Dialect::PostgreSQL; type DialectMarker = D; }
/// # impl<X: ToString> From<X> for Value { fn from(v: X) -> Self { Value(v.to_string()) } }
/// # impl From<Value> for std::borrow::Cow<'_, Value> { fn from(v: Value) -> Self { Self::Owned(v) } }
/// # type C<X, N = NonNull> = &'static SQLExpr<'static, Value, X, N>;
/// # fn col<X: drizzle_core::types::DataType, N: Nullability>(c: &'static str) -> C<X, N> { Box::leak(Box::new(SQLExpr::new(SQL::column(ColumnRef::sql("users", c))))) }
/// # type Int = <D as DialectTypes>::Int; type Text = <D as DialectTypes>::Text; type Real = <D as DialectTypes>::Double;
/// # struct Users { id: C<Int>, age: C<Int>, name: C<Text>, email: C<Text, Null>, score: C<Real, Null>, active: C<<D as DialectTypes>::Bool>, created_at: C<<D as DialectTypes>::Timestamp> }
/// # let users = Users { id: col("id"), age: col("age"), name: col("name"), email: col("email"), score: col("score"), active: col("active"), created_at: col("created_at") };
/// let names = string_agg(users.name, ", ");
/// assert_eq!(names.sql(), r#"STRING_AGG ("users"."name", $1)"#);
/// ```
#[allow(clippy::type_complexity)]
pub fn string_agg<'a, V, E, D>(
    expr: E,
    delimiter: D,
) -> SQLExpr<'a, V, <V::DialectMarker as DialectTypes>::Text, Null, Agg, (E::Sources, D::Sources)>
where
    V: SQLParam + 'a,
    V::DialectMarker: DialectSupports<feature::PostgresAggregate>,
    E: Expr<'a, V>,
    E::SQLType: crate::types::Textual,
    D: Expr<'a, V>,
    D::SQLType: crate::types::Textual,
{
    SQLExpr::new(SQL::func(
        "STRING_AGG",
        expr.into_expr_sql()
            .push(crate::Token::COMMA)
            .append(delimiter.into_expr_sql()),
    ))
}

// =============================================================================
// PostgreSQL Aggregate Functions
// =============================================================================

/// True when every non-NULL input is true (`EVERY`), on PostgreSQL.
///
/// The SQL-standard spelling of [`bool_and`], with the same typing.
///
/// # Examples
///
/// ```rust
/// # use drizzle_core::dialect::{Dialect, DialectTypes, PostgresDialect as D};
/// # use drizzle_core::{ColumnRef, SQL, SQLParam, expr::*};
/// # #[derive(Clone, Debug)] struct Value(String);
/// # impl SQLParam for Value { const DIALECT: Dialect = Dialect::PostgreSQL; type DialectMarker = D; }
/// # impl<X: ToString> From<X> for Value { fn from(v: X) -> Self { Value(v.to_string()) } }
/// # impl From<Value> for std::borrow::Cow<'_, Value> { fn from(v: Value) -> Self { Self::Owned(v) } }
/// # type C<X, N = NonNull> = &'static SQLExpr<'static, Value, X, N>;
/// # fn col<X: drizzle_core::types::DataType, N: Nullability>(c: &'static str) -> C<X, N> { Box::leak(Box::new(SQLExpr::new(SQL::column(ColumnRef::sql("users", c))))) }
/// # type Int = <D as DialectTypes>::Int; type Text = <D as DialectTypes>::Text; type Real = <D as DialectTypes>::Double;
/// # struct Users { id: C<Int>, age: C<Int>, name: C<Text>, email: C<Text, Null>, score: C<Real, Null>, active: C<<D as DialectTypes>::Bool>, created_at: C<<D as DialectTypes>::Timestamp> }
/// # let users = Users { id: col("id"), age: col("age"), name: col("name"), email: col("email"), score: col("score"), active: col("active"), created_at: col("created_at") };
/// assert_eq!(every(users.active).sql(), r#"EVERY ("users"."active")"#);
/// ```
pub fn every<'a, V, E>(
    expr: E,
) -> SQLExpr<'a, V, <V::DialectMarker as DialectTypes>::Bool, Null, Agg, E::Sources>
where
    V: SQLParam + 'a,
    V::DialectMarker: DialectSupports<feature::PostgresAggregate>,
    E: Expr<'a, V>,
    E::SQLType: BooleanAggregatePolicy<V::DialectMarker>,
{
    SQLExpr::new(SQL::func("EVERY", expr.into_expr_sql()))
}

/// Collects key/value pairs into a JSON object (`JSON_OBJECT_AGG`), on PostgreSQL.
///
/// Accepts any key and value expressions. The result is `json`, nullable, and
/// an aggregate.
///
/// # Examples
///
/// ```rust
/// # use drizzle_core::dialect::{Dialect, DialectTypes, PostgresDialect as D};
/// # use drizzle_core::{ColumnRef, SQL, SQLParam, expr::*};
/// # #[derive(Clone, Debug)] struct Value(String);
/// # impl SQLParam for Value { const DIALECT: Dialect = Dialect::PostgreSQL; type DialectMarker = D; }
/// # impl<X: ToString> From<X> for Value { fn from(v: X) -> Self { Value(v.to_string()) } }
/// # impl From<Value> for std::borrow::Cow<'_, Value> { fn from(v: Value) -> Self { Self::Owned(v) } }
/// # type C<X, N = NonNull> = &'static SQLExpr<'static, Value, X, N>;
/// # fn col<X: drizzle_core::types::DataType, N: Nullability>(c: &'static str) -> C<X, N> { Box::leak(Box::new(SQLExpr::new(SQL::column(ColumnRef::sql("users", c))))) }
/// # type Int = <D as DialectTypes>::Int; type Text = <D as DialectTypes>::Text; type Real = <D as DialectTypes>::Double;
/// # struct Users { id: C<Int>, age: C<Int>, name: C<Text>, email: C<Text, Null>, score: C<Real, Null>, active: C<<D as DialectTypes>::Bool>, created_at: C<<D as DialectTypes>::Timestamp> }
/// # let users = Users { id: col("id"), age: col("age"), name: col("name"), email: col("email"), score: col("score"), active: col("active"), created_at: col("created_at") };
/// let by_name = json_object_agg(users.name, users.age);
/// assert_eq!(by_name.sql(), r#"JSON_OBJECT_AGG ("users"."name", "users"."age")"#);
/// ```
#[allow(clippy::type_complexity)]
pub fn json_object_agg<'a, V, K, Val>(
    key: K,
    value: Val,
) -> SQLExpr<'a, V, <V::DialectMarker as DialectTypes>::Json, Null, Agg, (K::Sources, Val::Sources)>
where
    V: SQLParam + 'a,
    V::DialectMarker: DialectSupports<feature::PostgresAggregate>,
    K: Expr<'a, V>,
    Val: Expr<'a, V>,
{
    SQLExpr::new(SQL::func(
        "JSON_OBJECT_AGG",
        key.into_expr_sql()
            .push(crate::Token::COMMA)
            .append(value.into_expr_sql()),
    ))
}

/// Collects key/value pairs into a JSONB object (`JSONB_OBJECT_AGG`), on PostgreSQL.
///
/// Accepts any key and value expressions. The result is `jsonb`, nullable,
/// and an aggregate.
///
/// # Examples
///
/// ```rust
/// # use drizzle_core::dialect::{Dialect, DialectTypes, PostgresDialect as D};
/// # use drizzle_core::{ColumnRef, SQL, SQLParam, expr::*};
/// # #[derive(Clone, Debug)] struct Value(String);
/// # impl SQLParam for Value { const DIALECT: Dialect = Dialect::PostgreSQL; type DialectMarker = D; }
/// # impl<X: ToString> From<X> for Value { fn from(v: X) -> Self { Value(v.to_string()) } }
/// # impl From<Value> for std::borrow::Cow<'_, Value> { fn from(v: Value) -> Self { Self::Owned(v) } }
/// # type C<X, N = NonNull> = &'static SQLExpr<'static, Value, X, N>;
/// # fn col<X: drizzle_core::types::DataType, N: Nullability>(c: &'static str) -> C<X, N> { Box::leak(Box::new(SQLExpr::new(SQL::column(ColumnRef::sql("users", c))))) }
/// # type Int = <D as DialectTypes>::Int; type Text = <D as DialectTypes>::Text; type Real = <D as DialectTypes>::Double;
/// # struct Users { id: C<Int>, age: C<Int>, name: C<Text>, email: C<Text, Null>, score: C<Real, Null>, active: C<<D as DialectTypes>::Bool>, created_at: C<<D as DialectTypes>::Timestamp> }
/// # let users = Users { id: col("id"), age: col("age"), name: col("name"), email: col("email"), score: col("score"), active: col("active"), created_at: col("created_at") };
/// let by_name = jsonb_object_agg(users.name, users.age);
/// assert_eq!(by_name.sql(), r#"JSONB_OBJECT_AGG ("users"."name", "users"."age")"#);
/// ```
#[allow(clippy::type_complexity)]
pub fn jsonb_object_agg<'a, V, K, Val>(
    key: K,
    value: Val,
) -> SQLExpr<'a, V, <V::DialectMarker as DialectTypes>::Jsonb, Null, Agg, (K::Sources, Val::Sources)>
where
    V: SQLParam + 'a,
    V::DialectMarker: DialectSupports<feature::PostgresAggregate>,
    K: Expr<'a, V>,
    Val: Expr<'a, V>,
{
    SQLExpr::new(SQL::func(
        "JSONB_OBJECT_AGG",
        key.into_expr_sql()
            .push(crate::Token::COMMA)
            .append(value.into_expr_sql()),
    ))
}

// =============================================================================
// Distinct Wrapper
// =============================================================================

/// Prefixes an expression with `DISTINCT`.
///
/// Renders `DISTINCT expr` and keeps the expression's type and nullability.
/// Note that it is marked scalar, even when `expr` is an aggregate. For
/// aggregates, prefer [`count_distinct`], [`sum_distinct`] and
/// [`avg_distinct`].
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
/// assert_eq!(distinct(users.name).sql(), r#"DISTINCT "users"."name""#);
/// ```
pub fn distinct<'a, V, E>(expr: E) -> SQLExpr<'a, V, E::SQLType, E::Nullable, Scalar, E::Sources>
where
    V: SQLParam + 'a,
    E: Expr<'a, V>,
{
    SQLExpr::new(SQL::raw("DISTINCT").append(expr.into_expr_sql()))
}

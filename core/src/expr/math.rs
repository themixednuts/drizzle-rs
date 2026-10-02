//! Math functions: `ABS`, `ROUND`, `CEIL`, `FLOOR`, `SQRT`, `POWER`, `LN`, ...
//!
//! Every function needs numeric arguments; passing text does not compile.
//! Results keep the input's nullability and aggregate kind unless the function
//! says otherwise.
//!
//! On SQLite, `CEIL`, `FLOOR`, `TRUNC`, `SQRT`, `POWER`, `EXP`, `LN`, `LOG`,
//! `LOG10`, `LOG2` and `PI` exist only when SQLite was built with
//! `SQLITE_ENABLE_MATH_FUNCTIONS`. Those functions compile for SQLite only
//! with this crate's `math` feature (see [`MathExt`]).

use crate::dialect::DialectTypes;
use crate::sql::{SQL, Token};
use crate::traits::SQLParam;
use crate::types::{DataType, Integral, Numeric};
use crate::{Dialect, MySQLDialect, PostgresDialect, SQLiteDialect};
use drizzle_types::mysql::types::{
    BigInt as MyBigInt, BigIntUnsigned as MyBigIntUnsigned, Decimal as MyDecimal,
    Double as MyDouble, Float as MyFloat, Int as MyInt, IntUnsigned as MyIntUnsigned,
    MediumInt as MyMediumInt, MediumIntUnsigned as MyMediumIntUnsigned, SmallInt as MySmallInt,
    SmallIntUnsigned as MySmallIntUnsigned, TinyInt as MyTinyInt,
    TinyIntUnsigned as MyTinyIntUnsigned, Year as MyYear,
};
use drizzle_types::postgres::types::{Float4, Float8, Int2, Int4, Int8, Numeric as PgNumeric};
use drizzle_types::sqlite::types::{
    Integer as SqliteInteger, Numeric as SqliteNumeric, Real as SqliteReal,
};

use super::{AggregateKind, Expr, Nullability, SQLExpr, Scalar};

/// Dialects that provide the optional math functions.
///
/// `CEIL`, `FLOOR`, `TRUNC`, `SQRT`, `POWER`, `EXP`, `LN`, `LOG`, `LOG10`,
/// `LOG2` and `PI` are built into PostgreSQL and MySQL. SQLite has them only
/// when it is compiled with `SQLITE_ENABLE_MATH_FUNCTIONS`, which the bundled
/// `rusqlite` and `libsql` builds do not set. Enabling the `math` cargo
/// feature says that the linked SQLite has them. Without it, these functions
/// do not compile for SQLite, instead of failing at runtime with "no such
/// function".
#[diagnostic::on_unimplemented(
    message = "`{Self}` does not provide this math function",
    label = "SQLite only has CEIL/FLOOR/TRUNC/SQRT/POWER/EXP/LN/LOG*/PI with SQLITE_ENABLE_MATH_FUNCTIONS",
    note = "enable drizzle's `math` feature and build SQLite with the math functions, e.g. `LIBSQLITE3_FLAGS=\"-DSQLITE_ENABLE_MATH_FUNCTIONS\"` for bundled rusqlite"
)]
pub trait MathExt {}

impl MathExt for PostgresDialect {}
impl MathExt for MySQLDialect {}
#[cfg(feature = "math")]
impl MathExt for SQLiteDialect {}

#[diagnostic::on_unimplemented(
    message = "this math function is not available for this dialect",
    label = "use a dialect-specific alternative"
)]
/// Dialects that provide `LOG2`, and the nullability of its result.
///
/// Implemented for SQLite and MySQL, which both return NULL outside the
/// logarithm's domain. PostgreSQL has no `LOG2`.
pub trait Log2Policy {
    /// Nullability of the `LOG2` result.
    type Nullable: Nullability;
}

impl Log2Policy for SQLiteDialect {
    type Nullable = super::Null;
}
impl Log2Policy for MySQLDialect {
    type Nullable = super::Null;
}

#[diagnostic::on_unimplemented(
    message = "no rounding policy for `{Self}` on this dialect",
    label = "round/ceil/floor/trunc return type is not defined for this SQL type/dialect"
)]
/// Result type of [`round`], [`round_to`], [`ceil`], [`floor`] and [`trunc`]
/// for a numeric SQL type on dialect `D`.
///
/// | Dialect | Input | Result |
/// |---|---|---|
/// | SQLite | any numeric | `REAL` |
/// | PostgreSQL | any numeric | `float8` (the call is cast to `DOUBLE PRECISION`) |
/// | MySQL | signed integers | `BIGINT` |
/// | MySQL | unsigned integers, `YEAR` | `BIGINT UNSIGNED` |
/// | MySQL | `FLOAT`, `DOUBLE` | `DOUBLE` |
/// | MySQL | `DECIMAL` | `DECIMAL` |
pub trait RoundingPolicy<D>: Numeric {
    /// Result type of the rounding functions.
    type Output: DataType;

    /// Prepares the operand of `ROUND(expr, precision)`.
    ///
    /// PostgreSQL only defines the two-argument `ROUND` for `numeric`, and
    /// `double precision` does not cast to it implicitly.
    fn precision_operand<'a, V: SQLParam + 'a>(expr: SQL<'a, V>) -> SQL<'a, V> {
        expr
    }

    /// Coerces a rounding function's result to [`Self::Output`].
    ///
    /// PostgreSQL returns `numeric` for every rounding function unless the
    /// argument is `double precision`; the declared output is `float8`.
    fn coerce_result<'a, V: SQLParam + 'a>(sql: SQL<'a, V>) -> SQL<'a, V> {
        sql
    }
}

/// Casts a math function's result to `DOUBLE PRECISION` on PostgreSQL.
///
/// PostgreSQL resolves `SQRT`, `EXP`, `LN`, `LOG`, `POWER` and `SIGN` to their
/// `numeric` overloads for integer or `numeric` arguments, while the declared
/// result type is the dialect's double. The cast is a no-op for `float8`.
pub(super) fn pg_double<'a, V: SQLParam + 'a>(sql: SQL<'a, V>) -> SQL<'a, V> {
    match V::DIALECT {
        Dialect::PostgreSQL => pg_cast(sql, "DOUBLE PRECISION"),
        Dialect::SQLite | Dialect::MySQL => sql,
    }
}

/// Renders `CAST(expr AS type_name)`.
pub(super) fn pg_cast<'a, V: SQLParam + 'a>(
    expr: SQL<'a, V>,
    type_name: &'static str,
) -> SQL<'a, V> {
    SQL::func("CAST", expr.push(Token::AS).append(SQL::raw(type_name)))
}

impl RoundingPolicy<SQLiteDialect> for SqliteInteger {
    type Output = SqliteReal;
}
impl RoundingPolicy<SQLiteDialect> for SqliteReal {
    type Output = Self;
}
impl RoundingPolicy<SQLiteDialect> for SqliteNumeric {
    type Output = SqliteReal;
}

// Integers and NUMERIC round through NUMERIC on PostgreSQL; the result is
// cast to DOUBLE PRECISION so it decodes as the declared `Float8`.
macro_rules! postgres_numeric_rounding_policy {
    ($($ty:ty),+ $(,)?) => {
        $(
            impl RoundingPolicy<PostgresDialect> for $ty {
                type Output = Float8;

                fn coerce_result<'a, V: SQLParam + 'a>(sql: SQL<'a, V>) -> SQL<'a, V> {
                    pg_cast(sql, "DOUBLE PRECISION")
                }
            }
        )+
    };
}
postgres_numeric_rounding_policy!(Int2, Int4, Int8, PgNumeric);

// Floats round natively, but `ROUND(float, n)` only exists for NUMERIC, so the
// precision form casts in and back out.
macro_rules! postgres_float_rounding_policy {
    ($($ty:ty),+ $(,)?) => {
        $(
            impl RoundingPolicy<PostgresDialect> for $ty {
                type Output = Float8;

                fn precision_operand<'a, V: SQLParam + 'a>(expr: SQL<'a, V>) -> SQL<'a, V> {
                    pg_cast(expr, "NUMERIC")
                }

                fn coerce_result<'a, V: SQLParam + 'a>(sql: SQL<'a, V>) -> SQL<'a, V> {
                    pg_cast(sql, "DOUBLE PRECISION")
                }
            }
        )+
    };
}
postgres_float_rounding_policy!(Float4, Float8);

macro_rules! mysql_rounding_policy {
    ($output:ty; $($ty:ty),+ $(,)?) => {
        $(
            impl RoundingPolicy<MySQLDialect> for $ty {
                type Output = $output;
            }
        )+
    };
}

mysql_rounding_policy!(MyBigInt; MyTinyInt, MySmallInt, MyMediumInt, MyInt, MyBigInt,);
mysql_rounding_policy!(MyBigIntUnsigned;
    MyTinyIntUnsigned,
    MySmallIntUnsigned,
    MyMediumIntUnsigned,
    MyIntUnsigned,
    MyBigIntUnsigned,
    MyYear,
);

impl RoundingPolicy<MySQLDialect> for MyFloat {
    type Output = MyDouble;
}
impl RoundingPolicy<MySQLDialect> for MyDouble {
    type Output = Self;
}
impl RoundingPolicy<MySQLDialect> for MyDecimal {
    type Output = Self;
}

// =============================================================================
// ABSOLUTE VALUE
// =============================================================================

/// Absolute value (`ABS`).
///
/// The argument must be numeric. The result keeps the argument's SQL type,
/// nullability and aggregate kind.
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
/// assert_eq!(abs(users.score).sql(), r#"ABS ("users"."score")"#);
/// ```
///
/// # Type safety
///
/// `ABS` of a text column does not compile:
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
/// let wrong = abs(users.name);
/// ```
pub fn abs<'a, V, E>(expr: E) -> SQLExpr<'a, V, E::SQLType, E::Nullable, E::Aggregate, E::Sources>
where
    V: SQLParam + 'a,
    E: Expr<'a, V>,
    E::SQLType: Numeric,
{
    SQLExpr::new(SQL::func("ABS", expr.into_sql()))
}

// =============================================================================
// ROUNDING FUNCTIONS
// =============================================================================

/// Rounds to the nearest integer (`ROUND(expr)`).
///
/// The argument must be numeric. The result type comes from
/// [`RoundingPolicy`] (`REAL` on SQLite, `float8` on PostgreSQL, the
/// matching integer or decimal type on MySQL) and keeps the argument's
/// nullability.
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
/// assert_eq!(round(users.score).sql(), r#"ROUND ("users"."score")"#);
/// ```
#[allow(clippy::type_complexity)]
pub fn round<'a, V, E>(
    expr: E,
) -> SQLExpr<
    'a,
    V,
    <E::SQLType as RoundingPolicy<V::DialectMarker>>::Output,
    E::Nullable,
    E::Aggregate,
    E::Sources,
>
where
    V: SQLParam + 'a,
    E: Expr<'a, V>,
    E::SQLType: RoundingPolicy<V::DialectMarker>,
{
    SQLExpr::new(
        <E::SQLType as RoundingPolicy<V::DialectMarker>>::coerce_result(SQL::func(
            "ROUND",
            expr.into_sql(),
        )),
    )
}

/// Rounds to `precision` decimal places (`ROUND(expr, precision)`).
///
/// `expr` must be numeric and `precision` an integer. The result type comes
/// from [`RoundingPolicy`]; it is nullable if either argument is. On
/// PostgreSQL a float argument is cast to `NUMERIC` first, because only
/// `ROUND(numeric, int)` exists there.
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
/// assert_eq!(round_to(users.score, 2).sql(), r#"ROUND ("users"."score", ?)"#);
/// ```
#[allow(clippy::type_complexity)]
pub fn round_to<'a, V, E, P>(
    expr: E,
    precision: P,
) -> SQLExpr<
    'a,
    V,
    <E::SQLType as RoundingPolicy<V::DialectMarker>>::Output,
    <E::Nullable as Nullability>::Or<P::Nullable>,
    <E::Aggregate as AggregateKind>::Or<P::Aggregate>,
    (E::Sources, P::Sources),
>
where
    V: SQLParam + 'a,
    E: Expr<'a, V>,
    E::SQLType: RoundingPolicy<V::DialectMarker>,
    P: Expr<'a, V>,
    P::SQLType: Integral,
    P::Nullable: Nullability,
{
    SQLExpr::new(
        <E::SQLType as RoundingPolicy<V::DialectMarker>>::coerce_result(SQL::func(
            "ROUND",
            <E::SQLType as RoundingPolicy<V::DialectMarker>>::precision_operand(expr.into_sql())
                .push(Token::COMMA)
                .append(precision.into_sql()),
        )),
    )
}

/// Rounds up to the nearest integer (`CEIL`).
///
/// The argument must be numeric. The result type comes from
/// [`RoundingPolicy`] and keeps the argument's nullability. On SQLite this
/// needs the `math` feature (see [`MathExt`]).
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
///     ceil(users.score).sql(),
///     r#"CAST (CEIL ("users"."score") AS DOUBLE PRECISION)"#
/// );
/// ```
#[allow(clippy::type_complexity)]
pub fn ceil<'a, V, E>(
    expr: E,
) -> SQLExpr<
    'a,
    V,
    <E::SQLType as RoundingPolicy<V::DialectMarker>>::Output,
    E::Nullable,
    E::Aggregate,
    E::Sources,
>
where
    V: SQLParam + 'a,
    V::DialectMarker: MathExt,
    E: Expr<'a, V>,
    E::SQLType: RoundingPolicy<V::DialectMarker>,
{
    SQLExpr::new(
        <E::SQLType as RoundingPolicy<V::DialectMarker>>::coerce_result(SQL::func(
            "CEIL",
            expr.into_sql(),
        )),
    )
}

/// Rounds down to the nearest integer (`FLOOR`).
///
/// The argument must be numeric. The result type comes from
/// [`RoundingPolicy`] and keeps the argument's nullability. On SQLite this
/// needs the `math` feature (see [`MathExt`]).
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
///     floor(users.score).sql(),
///     r#"CAST (FLOOR ("users"."score") AS DOUBLE PRECISION)"#
/// );
/// ```
#[allow(clippy::type_complexity)]
pub fn floor<'a, V, E>(
    expr: E,
) -> SQLExpr<
    'a,
    V,
    <E::SQLType as RoundingPolicy<V::DialectMarker>>::Output,
    E::Nullable,
    E::Aggregate,
    E::Sources,
>
where
    V: SQLParam + 'a,
    V::DialectMarker: MathExt,
    E: Expr<'a, V>,
    E::SQLType: RoundingPolicy<V::DialectMarker>,
{
    SQLExpr::new(
        <E::SQLType as RoundingPolicy<V::DialectMarker>>::coerce_result(SQL::func(
            "FLOOR",
            expr.into_sql(),
        )),
    )
}

/// Truncates toward zero.
///
/// Renders `TRUNC(expr)` on SQLite and PostgreSQL and `TRUNCATE(expr, 0)` on
/// MySQL. The argument must be numeric. The result type comes from
/// [`RoundingPolicy`] and keeps the argument's nullability. On SQLite this
/// needs the `math` feature (see [`MathExt`]).
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
///     trunc(users.score).sql(),
///     r#"CAST (TRUNC ("users"."score") AS DOUBLE PRECISION)"#
/// );
/// ```
#[allow(clippy::type_complexity)]
pub fn trunc<'a, V, E>(
    expr: E,
) -> SQLExpr<
    'a,
    V,
    <E::SQLType as RoundingPolicy<V::DialectMarker>>::Output,
    E::Nullable,
    E::Aggregate,
    E::Sources,
>
where
    V: SQLParam + 'a,
    V::DialectMarker: MathExt,
    E: Expr<'a, V>,
    E::SQLType: RoundingPolicy<V::DialectMarker>,
{
    let expr = expr.into_sql();
    let truncated = match V::DIALECT {
        Dialect::MySQL => SQL::func("TRUNCATE", expr.push(Token::COMMA).append(SQL::raw("0"))),
        Dialect::SQLite | Dialect::PostgreSQL => SQL::func("TRUNC", expr),
    };
    SQLExpr::new(<E::SQLType as RoundingPolicy<V::DialectMarker>>::coerce_result(truncated))
}

// =============================================================================
// POWER AND ROOT FUNCTIONS
// =============================================================================

/// Square root (`SQRT`).
///
/// The argument must be numeric. The result is the dialect's double type. On
/// SQLite and MySQL a negative argument gives NULL, so the result is
/// nullable there; PostgreSQL raises an error instead and keeps the
/// argument's nullability. On SQLite this needs the `math` feature.
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
///     sqrt(users.score).sql(),
///     r#"CAST (SQRT ("users"."score") AS DOUBLE PRECISION)"#
/// );
/// ```
#[allow(clippy::type_complexity)]
pub fn sqrt<'a, V, E>(
    expr: E,
) -> SQLExpr<
    'a,
    V,
    <V::DialectMarker as DialectTypes>::Double,
    <V::DialectMarker as DialectTypes>::DomainNullable<E::Nullable>,
    E::Aggregate,
    E::Sources,
>
where
    V: SQLParam + 'a,
    V::DialectMarker: MathExt,
    E: Expr<'a, V>,
    E::SQLType: Numeric,
{
    SQLExpr::new(pg_double(SQL::func("SQRT", expr.into_sql())))
}

/// `base` raised to `exponent` (`POWER`).
///
/// Both arguments must be numeric. The result is the dialect's double type,
/// nullable if either argument is. On SQLite this needs the `math` feature.
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
///     power(users.age, 2).sql(),
///     r#"CAST (POWER ("users"."age", $1) AS DOUBLE PRECISION)"#
/// );
/// ```
#[allow(clippy::type_complexity)]
pub fn power<'a, V, E1, E2>(
    base: E1,
    exponent: E2,
) -> SQLExpr<
    'a,
    V,
    <V::DialectMarker as DialectTypes>::Double,
    <E1::Nullable as Nullability>::Or<E2::Nullable>,
    <E1::Aggregate as AggregateKind>::Or<E2::Aggregate>,
    (E1::Sources, E2::Sources),
>
where
    V: SQLParam + 'a,
    V::DialectMarker: MathExt,
    E1: Expr<'a, V>,
    E1::SQLType: Numeric,
    E2: Expr<'a, V>,
    E2::SQLType: Numeric,
    E2::Nullable: Nullability,
{
    SQLExpr::new(pg_double(SQL::func(
        "POWER",
        base.into_sql()
            .push(Token::COMMA)
            .append(exponent.into_sql()),
    )))
}

// =============================================================================
// LOGARITHMIC AND EXPONENTIAL FUNCTIONS
// =============================================================================

/// e raised to the argument (`EXP`).
///
/// The argument must be numeric. The result is the dialect's double type and
/// keeps the argument's nullability. On SQLite this needs the `math` feature.
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
///     exp(users.score).sql(),
///     r#"CAST (EXP ("users"."score") AS DOUBLE PRECISION)"#
/// );
/// ```
#[allow(clippy::type_complexity)]
pub fn exp<'a, V, E>(
    expr: E,
) -> SQLExpr<'a, V, <V::DialectMarker as DialectTypes>::Double, E::Nullable, E::Aggregate, E::Sources>
where
    V: SQLParam + 'a,
    V::DialectMarker: MathExt,
    E: Expr<'a, V>,
    E::SQLType: Numeric,
{
    SQLExpr::new(pg_double(SQL::func("EXP", expr.into_sql())))
}

/// Natural logarithm (`LN`).
///
/// The argument must be numeric. The result is the dialect's double type. On
/// SQLite and MySQL an argument outside the domain (zero or negative) gives
/// NULL, so the result is nullable there; PostgreSQL raises an error instead
/// and keeps the argument's nullability. On SQLite this needs the `math`
/// feature.
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
///     ln(users.score).sql(),
///     r#"CAST (LN ("users"."score") AS DOUBLE PRECISION)"#
/// );
/// ```
#[allow(clippy::type_complexity)]
pub fn ln<'a, V, E>(
    expr: E,
) -> SQLExpr<
    'a,
    V,
    <V::DialectMarker as DialectTypes>::Double,
    <V::DialectMarker as DialectTypes>::DomainNullable<E::Nullable>,
    E::Aggregate,
    E::Sources,
>
where
    V: SQLParam + 'a,
    V::DialectMarker: MathExt,
    E: Expr<'a, V>,
    E::SQLType: Numeric,
{
    SQLExpr::new(pg_double(SQL::func("LN", expr.into_sql())))
}

/// Base-10 logarithm (`LOG10`).
///
/// The argument must be numeric. The result is the dialect's double type. On
/// SQLite and MySQL an argument outside the domain (zero or negative) gives
/// NULL, so the result is nullable there; PostgreSQL raises an error instead
/// and keeps the argument's nullability. On SQLite this needs the `math`
/// feature.
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
///     log10(users.score).sql(),
///     r#"CAST (LOG10 ("users"."score") AS DOUBLE PRECISION)"#
/// );
/// ```
#[allow(clippy::type_complexity)]
pub fn log10<'a, V, E>(
    expr: E,
) -> SQLExpr<
    'a,
    V,
    <V::DialectMarker as DialectTypes>::Double,
    <V::DialectMarker as DialectTypes>::DomainNullable<E::Nullable>,
    E::Aggregate,
    E::Sources,
>
where
    V: SQLParam + 'a,
    V::DialectMarker: MathExt,
    E: Expr<'a, V>,
    E::SQLType: Numeric,
{
    SQLExpr::new(pg_double(SQL::func("LOG10", expr.into_sql())))
}

/// Logarithm of `value` in base `base` (`LOG(base, value)`).
///
/// Both arguments must be numeric. The result is the dialect's double type.
/// It is nullable if either argument is, and always nullable on SQLite and
/// MySQL, which return NULL outside the domain. On PostgreSQL both arguments
/// are cast to `NUMERIC`, since only `LOG(numeric, numeric)` exists there. On
/// SQLite this needs the `math` feature.
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
///     log(2, users.score).sql(),
///     r#"CAST (LOG (CAST ($1 AS NUMERIC), CAST ("users"."score" AS NUMERIC)) AS DOUBLE PRECISION)"#
/// );
/// ```
#[allow(clippy::type_complexity)]
pub fn log<'a, V, E1, E2>(
    base: E1,
    value: E2,
) -> SQLExpr<
    'a,
    V,
    <V::DialectMarker as DialectTypes>::Double,
    <V::DialectMarker as DialectTypes>::DomainNullable<
        <E1::Nullable as Nullability>::Or<E2::Nullable>,
    >,
    <E1::Aggregate as AggregateKind>::Or<E2::Aggregate>,
    (E1::Sources, E2::Sources),
>
where
    V: SQLParam + 'a,
    V::DialectMarker: MathExt,
    E1: Expr<'a, V>,
    E1::SQLType: Numeric,
    E2: Expr<'a, V>,
    E2::SQLType: Numeric,
    E2::Nullable: Nullability,
{
    let (base, value) = (base.into_sql(), value.into_sql());
    // PostgreSQL only defines the two-argument LOG for NUMERIC operands.
    let (base, value) = match V::DIALECT {
        Dialect::PostgreSQL => (pg_cast(base, "NUMERIC"), pg_cast(value, "NUMERIC")),
        Dialect::SQLite | Dialect::MySQL => (base, value),
    };
    SQLExpr::new(pg_double(SQL::func(
        "LOG",
        base.push(Token::COMMA).append(value),
    )))
}

// =============================================================================
// SIGN AND MODULO
// =============================================================================

/// Sign of a number: -1, 0 or 1 (`SIGN`).
///
/// The argument must be numeric. The result is the dialect's
/// [`Sign`](DialectTypes::Sign) type: an integer on SQLite and MySQL,
/// `float8` on PostgreSQL. It keeps the argument's nullability.
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
/// assert_eq!(sign(users.score).sql(), r#"SIGN ("users"."score")"#);
/// ```
#[allow(clippy::type_complexity)]
pub fn sign<'a, V, E>(
    expr: E,
) -> SQLExpr<'a, V, <V::DialectMarker as DialectTypes>::Sign, E::Nullable, E::Aggregate, E::Sources>
where
    V: SQLParam + 'a,
    E: Expr<'a, V>,
    E::SQLType: Numeric,
{
    SQLExpr::new(pg_double(SQL::func("SIGN", expr.into_sql())))
}

/// Remainder of a division, rendered with the `%` operator.
///
/// Both arguments must be numeric. The result has the dividend's SQL type and
/// is nullable if either argument is. Named `mod_` because `mod` is a Rust
/// keyword. `expr % n` on an [`SQLExpr`] renders the same SQL, but types the
/// result through [`ArithmeticOutput`](crate::types::ArithmeticOutput), which
/// also marks it nullable on SQLite and MySQL (where `x % 0` is NULL).
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
/// assert_eq!(mod_(users.age, 10).sql(), r#""users"."age" % ?"#);
/// ```
#[allow(clippy::type_complexity)]
pub fn mod_<'a, V, E1, E2>(
    dividend: E1,
    divisor: E2,
) -> SQLExpr<
    'a,
    V,
    E1::SQLType,
    <E1::Nullable as Nullability>::Or<E2::Nullable>,
    <E1::Aggregate as AggregateKind>::Or<E2::Aggregate>,
    (E1::Sources, E2::Sources),
>
where
    V: SQLParam + 'a,
    E1: Expr<'a, V>,
    E1::SQLType: Numeric,
    E2: Expr<'a, V>,
    E2::SQLType: Numeric,
    E2::Nullable: Nullability,
{
    SQLExpr::new(super::ops::binary_operator_sql(
        dividend.into_expr_sql(),
        Token::REM,
        divisor.into_expr_sql(),
    ))
}

// =============================================================================
// CONSTANTS AND RANDOM
// =============================================================================

/// The constant pi (`PI()`).
///
/// The result is the dialect's double type and never NULL. Available on
/// PostgreSQL and MySQL, and on SQLite with the `math` feature.
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
/// assert_eq!(pi::<Value>().sql(), "PI()");
/// ```
#[must_use]
pub fn pi<'a, V>()
-> SQLExpr<'a, V, <V::DialectMarker as DialectTypes>::Double, super::NonNull, Scalar, ()>
where
    V: SQLParam + 'a,
    V::DialectMarker: MathExt,
    V::DialectMarker: MathExt,
{
    SQLExpr::new(SQL::raw("PI()"))
}

/// A random value.
///
/// Renders `RANDOM()` on SQLite and PostgreSQL and `RAND()` on MySQL. The
/// result type depends on the dialect: SQLite returns a 64-bit integer,
/// PostgreSQL and MySQL a float in `[0, 1)`. It is never NULL.
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
/// assert_eq!(random::<Value>().sql(), "RANDOM()");
/// ```
#[must_use]
pub fn random<'a, V>()
-> SQLExpr<'a, V, <V::DialectMarker as DialectTypes>::Random, super::NonNull, Scalar, ()>
where
    V: SQLParam + 'a,
{
    SQLExpr::new(SQL::raw(match V::DIALECT {
        Dialect::MySQL => "RAND()",
        Dialect::SQLite | Dialect::PostgreSQL => "RANDOM()",
    }))
}

// =============================================================================
// Dialect-gated Math Functions
// =============================================================================

/// Base-2 logarithm (`LOG2`), on SQLite and MySQL.
///
/// The argument must be numeric. The result is the dialect's double type and
/// always nullable, since both dialects return NULL outside the domain.
/// SQLite needs the `math` feature. PostgreSQL has no `LOG2`, so this does not
/// compile for PostgreSQL; use [`log`] with base 2 there.
///
/// # Examples
///
/// ```rust
/// # use drizzle_core::dialect::{Dialect, DialectTypes, MySQLDialect as D};
/// # use drizzle_core::{ColumnRef, SQL, SQLParam, expr::*};
/// # #[derive(Clone, Debug)] struct Value(String);
/// # impl SQLParam for Value { const DIALECT: Dialect = Dialect::MySQL; type DialectMarker = D; }
/// # impl<X: ToString> From<X> for Value { fn from(v: X) -> Self { Value(v.to_string()) } }
/// # impl From<Value> for std::borrow::Cow<'_, Value> { fn from(v: Value) -> Self { Self::Owned(v) } }
/// # type C<X, N = NonNull> = &'static SQLExpr<'static, Value, X, N>;
/// # fn col<X: drizzle_core::types::DataType, N: Nullability>(c: &'static str) -> C<X, N> { Box::leak(Box::new(SQLExpr::new(SQL::column(ColumnRef::sql("users", c))))) }
/// # type Int = <D as DialectTypes>::Int; type Text = <D as DialectTypes>::Text; type Real = <D as DialectTypes>::Double;
/// # struct Users { id: C<Int>, age: C<Int>, name: C<Text>, email: C<Text, Null>, score: C<Real, Null>, active: C<<D as DialectTypes>::Bool>, created_at: C<<D as DialectTypes>::Timestamp> }
/// # let users = Users { id: col("id"), age: col("age"), name: col("name"), email: col("email"), score: col("score"), active: col("active"), created_at: col("created_at") };
/// assert_eq!(log2(users.score).sql(), "LOG2(`users`.`score`)");
/// ```
#[allow(clippy::type_complexity)]
pub fn log2<'a, V, E>(
    expr: E,
) -> SQLExpr<
    'a,
    V,
    <V::DialectMarker as DialectTypes>::Double,
    <V::DialectMarker as Log2Policy>::Nullable,
    E::Aggregate,
    E::Sources,
>
where
    V: SQLParam + 'a,
    V::DialectMarker: MathExt,
    V::DialectMarker: Log2Policy,
    E: Expr<'a, V>,
    E::SQLType: Numeric,
{
    SQLExpr::new(SQL::func("LOG2", expr.into_sql()))
}

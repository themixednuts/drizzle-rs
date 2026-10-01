//! String functions: `UPPER`, `LOWER`, `TRIM`, `LENGTH`, `SUBSTR`, `REPLACE`, ...
//!
//! Text arguments must have a text SQL type and position or length arguments
//! an integer type; anything else does not compile. Functions that exist on
//! only some databases do not compile for the others.

use crate::dialect::{Dialect, DialectTypes};
use crate::dialect::{DialectSupports, feature};
use crate::sql::{SQL, Token};
use crate::traits::{SQLParam, ToSQL};
use crate::types::{DataType, Integral, Textual};
use crate::{MySQLDialect, PostgresDialect, SQLiteDialect};
use drizzle_types::postgres::types::{
    Char as PgChar, Int4 as PgInt4, Text as PgText, Varchar as PgVarchar,
};
use drizzle_types::sqlite::types::{Integer as SqliteInteger, Text as SqliteText};

use super::ExprSources;
use super::{AggregateKind, Expr, NonNull, Nullability, SQLExpr};
use crate::scope::Arg;

#[diagnostic::on_unimplemented(
    message = "no length policy for `{Self}` on this dialect",
    label = "length return type is not defined for this SQL type/dialect"
)]
/// Result type of [`length`], [`char_length`] and [`octet_length`] for a
/// text SQL type on dialect `D`: `INTEGER` on SQLite, `int4` on PostgreSQL,
/// `BIGINT` on MySQL.
pub trait LengthPolicy<D>: DataType {
    /// Result type of the length functions.
    type Output: DataType;
}

#[diagnostic::on_unimplemented(
    message = "INSTR is not available for this dialect",
    label = "use a dialect-specific substring-position function"
)]
/// Dialects that provide [`instr`] (SQLite and MySQL), and its result type.
pub trait InstrPolicy {
    /// Result type of `INSTR`.
    type Output: DataType;
}

impl LengthPolicy<SQLiteDialect> for SqliteText {
    type Output = SqliteInteger;
}
impl LengthPolicy<SQLiteDialect> for drizzle_types::sqlite::types::Any {
    type Output = SqliteInteger;
}

impl LengthPolicy<PostgresDialect> for PgVarchar {
    type Output = PgInt4;
}
impl LengthPolicy<PostgresDialect> for PgText {
    type Output = PgInt4;
}
impl LengthPolicy<PostgresDialect> for PgChar {
    type Output = PgInt4;
}

macro_rules! mysql_length_policy {
    ($($ty:ty),+ $(,)?) => {
        $(
            impl LengthPolicy<MySQLDialect> for $ty {
                type Output = drizzle_types::mysql::types::BigInt;
            }
        )+
    };
}

mysql_length_policy!(
    drizzle_types::mysql::types::Char,
    drizzle_types::mysql::types::Varchar,
    drizzle_types::mysql::types::TinyText,
    drizzle_types::mysql::types::Text,
    drizzle_types::mysql::types::MediumText,
    drizzle_types::mysql::types::LongText,
    drizzle_types::mysql::types::Enum,
    drizzle_types::mysql::types::Set,
);

impl DialectSupports<feature::PostgresString> for PostgresDialect {}

impl InstrPolicy for SQLiteDialect {
    type Output = SqliteInteger;
}
impl InstrPolicy for MySQLDialect {
    type Output = drizzle_types::mysql::types::BigInt;
}

impl DialectSupports<feature::LeftRight> for PostgresDialect {}
impl DialectSupports<feature::LeftRight> for MySQLDialect {}
impl DialectSupports<feature::Pad> for PostgresDialect {}
impl DialectSupports<feature::Pad> for MySQLDialect {}
impl DialectSupports<feature::Reverse> for PostgresDialect {}
impl DialectSupports<feature::Reverse> for MySQLDialect {}
impl DialectSupports<feature::Repeat> for PostgresDialect {}
impl DialectSupports<feature::Repeat> for MySQLDialect {}

// =============================================================================
// CASE CONVERSION
// =============================================================================

/// Converts text to upper case (`UPPER`).
///
/// The argument must be text. The result is text and keeps the argument's
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
/// assert_eq!(upper(users.name).sql(), r#"UPPER ("users"."name")"#);
/// ```
///
/// # Type safety
///
/// `UPPER` of an integer column does not compile:
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
/// let wrong = upper(users.id);
/// ```
#[allow(clippy::type_complexity)]
pub fn upper<'a, V, E>(
    expr: E,
) -> SQLExpr<'a, V, <V::DialectMarker as DialectTypes>::Text, E::Nullable, E::Aggregate, E::Sources>
where
    V: SQLParam + 'a,
    E: Expr<'a, V>,
    E::SQLType: Textual,
{
    SQLExpr::new(SQL::func("UPPER", expr.into_sql()))
}

/// Converts text to lower case (`LOWER`).
///
/// The argument must be text. The result is text and keeps the argument's
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
/// assert_eq!(lower(users.email).sql(), r#"LOWER ("users"."email")"#);
/// ```
#[allow(clippy::type_complexity)]
pub fn lower<'a, V, E>(
    expr: E,
) -> SQLExpr<'a, V, <V::DialectMarker as DialectTypes>::Text, E::Nullable, E::Aggregate, E::Sources>
where
    V: SQLParam + 'a,
    E: Expr<'a, V>,
    E::SQLType: Textual,
{
    SQLExpr::new(SQL::func("LOWER", expr.into_sql()))
}

// =============================================================================
// TRIM FUNCTIONS
// =============================================================================

/// Removes leading and trailing spaces (`TRIM`).
///
/// The argument must be text. The result is text and keeps the argument's
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
/// assert_eq!(trim(users.name).sql(), r#"TRIM ("users"."name")"#);
/// ```
#[allow(clippy::type_complexity)]
pub fn trim<'a, V, E>(
    expr: E,
) -> SQLExpr<'a, V, <V::DialectMarker as DialectTypes>::Text, E::Nullable, E::Aggregate, E::Sources>
where
    V: SQLParam + 'a,
    E: Expr<'a, V>,
    E::SQLType: Textual,
{
    SQLExpr::new(SQL::func("TRIM", expr.into_sql()))
}

/// Removes leading spaces (`LTRIM`).
///
/// The argument must be text. The result is text and keeps the argument's
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
/// assert_eq!(ltrim(users.name).sql(), r#"LTRIM ("users"."name")"#);
/// ```
#[allow(clippy::type_complexity)]
pub fn ltrim<'a, V, E>(
    expr: E,
) -> SQLExpr<'a, V, <V::DialectMarker as DialectTypes>::Text, E::Nullable, E::Aggregate, E::Sources>
where
    V: SQLParam + 'a,
    E: Expr<'a, V>,
    E::SQLType: Textual,
{
    SQLExpr::new(SQL::func("LTRIM", expr.into_sql()))
}

/// Removes trailing spaces (`RTRIM`).
///
/// The argument must be text. The result is text and keeps the argument's
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
/// assert_eq!(rtrim(users.name).sql(), r#"RTRIM ("users"."name")"#);
/// ```
#[allow(clippy::type_complexity)]
pub fn rtrim<'a, V, E>(
    expr: E,
) -> SQLExpr<'a, V, <V::DialectMarker as DialectTypes>::Text, E::Nullable, E::Aggregate, E::Sources>
where
    V: SQLParam + 'a,
    E: Expr<'a, V>,
    E::SQLType: Textual,
{
    SQLExpr::new(SQL::func("RTRIM", expr.into_sql()))
}

// =============================================================================
// COLLATE
// =============================================================================

/// Applies a collation to a text expression (`expr COLLATE "name"`).
///
/// Renders `(expr COLLATE "name")`, with the name quoted as an identifier,
/// which both SQLite and PostgreSQL accept. The argument must be text. The
/// result keeps its SQL type, nullability and aggregate kind. Use it in a
/// comparison for case-insensitive matching, or in `ORDER BY`.
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
/// // Case-insensitive comparison on SQLite.
/// let cond = eq(collate(users.name, "NOCASE"), "alice");
/// assert_eq!(cond.sql(), r#"("users"."name" COLLATE "NOCASE") = ?"#);
/// ```
pub fn collate<'a, V, E>(
    expr: E,
    name: &'static str,
) -> SQLExpr<'a, V, E::SQLType, E::Nullable, E::Aggregate, E::Sources>
where
    V: SQLParam + 'a,
    E: Expr<'a, V>,
    E::SQLType: Textual,
{
    let inner = expr.into_sql().parens_if_subquery();
    SQLExpr::new(
        SQL::token(Token::LPAREN)
            .append(inner)
            .push(Token::COLLATE)
            .append(SQL::ident(name))
            .push(Token::RPAREN),
    )
}

// =============================================================================
// LENGTH
// =============================================================================

/// Length of a text value (`LENGTH`).
///
/// SQLite and PostgreSQL count characters; MySQL counts bytes. Use
/// [`char_length`] to count characters on every dialect. The argument must be
/// text. The result is an integer (see [`LengthPolicy`]) and keeps the
/// argument's nullability.
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
/// assert_eq!(length(users.name).sql(), r#"LENGTH ("users"."name")"#);
/// ```
#[allow(clippy::type_complexity)]
pub fn length<'a, V, E>(
    expr: E,
) -> SQLExpr<
    'a,
    V,
    <E::SQLType as LengthPolicy<V::DialectMarker>>::Output,
    E::Nullable,
    E::Aggregate,
    E::Sources,
>
where
    V: SQLParam + 'a,
    E: Expr<'a, V>,
    E::SQLType: LengthPolicy<V::DialectMarker>,
{
    SQLExpr::new(SQL::func("LENGTH", expr.into_sql()))
}

// =============================================================================
// SUBSTRING
// =============================================================================

/// Part of a text value (`SUBSTR(expr, start, len)`).
///
/// Returns `len` characters starting at `start`, counting from 1. `expr`
/// must be text; `start` and `len` must be integers. The result is text,
/// nullable if any argument is.
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
/// // The first three characters.
/// let prefix = substr(users.name, 1, 3);
/// assert_eq!(prefix.sql(), r#"SUBSTR ("users"."name", ?, ?)"#);
/// ```
#[allow(clippy::type_complexity)]
pub fn substr<'a, V, E, S, L>(
    expr: E,
    start: S,
    len: L,
) -> SQLExpr<
    'a,
    V,
    <V::DialectMarker as DialectTypes>::Text,
    <<E::Nullable as Nullability>::Or<S::Nullable> as Nullability>::Or<L::Nullable>,
    <<E::Aggregate as AggregateKind>::Or<S::Aggregate> as AggregateKind>::Or<L::Aggregate>,
    (E::Sources, (S::Sources, L::Sources)),
>
where
    V: SQLParam + 'a,
    E: Expr<'a, V>,
    E::SQLType: Textual,
    S: Expr<'a, V>,
    S::SQLType: Integral,
    S::Nullable: Nullability,
    S::Aggregate: AggregateKind,
    L: Expr<'a, V>,
    L::SQLType: Integral,
    L::Nullable: Nullability,
    L::Aggregate: AggregateKind,
{
    SQLExpr::new(SQL::func(
        "SUBSTR",
        expr.into_sql()
            .push(Token::COMMA)
            .append(start.into_sql())
            .push(Token::COMMA)
            .append(len.into_sql()),
    ))
}

// =============================================================================
// REPLACE
// =============================================================================

/// Replaces every occurrence of `from` with `to` (`REPLACE`).
///
/// All three arguments must be text. The result is text, nullable if any
/// argument is.
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
/// let email = replace(users.email, "@old.example", "@new.example");
/// assert_eq!(email.sql(), r#"REPLACE ("users"."email", ?, ?)"#);
/// ```
#[allow(clippy::type_complexity)]
pub fn replace<'a, V, E, F, T>(
    expr: E,
    from: F,
    to: T,
) -> SQLExpr<
    'a,
    V,
    <V::DialectMarker as DialectTypes>::Text,
    <<E::Nullable as Nullability>::Or<F::Nullable> as Nullability>::Or<T::Nullable>,
    <<E::Aggregate as AggregateKind>::Or<F::Aggregate> as AggregateKind>::Or<T::Aggregate>,
    (E::Sources, (F::Sources, T::Sources)),
>
where
    V: SQLParam + 'a,
    E: Expr<'a, V>,
    E::SQLType: Textual,
    F: Expr<'a, V>,
    F::SQLType: Textual,
    F::Nullable: Nullability,
    F::Aggregate: AggregateKind,
    T: Expr<'a, V>,
    T::SQLType: Textual,
    T::Nullable: Nullability,
    T::Aggregate: AggregateKind,
{
    SQLExpr::new(SQL::func(
        "REPLACE",
        expr.into_sql()
            .push(Token::COMMA)
            .append(from.into_sql())
            .push(Token::COMMA)
            .append(to.into_sql()),
    ))
}

// =============================================================================
// INSTR
// =============================================================================

/// Position of `search` within a text value (`INSTR`), on SQLite and MySQL.
///
/// Returns the 1-based position of the first match, or 0 when there is none.
/// Both arguments must be text. The result is an integer, nullable if either
/// argument is. On PostgreSQL, use [`strpos`].
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
/// assert_eq!(instr(users.email, "@").sql(), r#"INSTR ("users"."email", ?)"#);
/// ```
#[allow(clippy::type_complexity)]
pub fn instr<'a, V, E, S>(
    expr: E,
    search: S,
) -> SQLExpr<
    'a,
    V,
    <V::DialectMarker as InstrPolicy>::Output,
    <E::Nullable as Nullability>::Or<S::Nullable>,
    <E::Aggregate as AggregateKind>::Or<S::Aggregate>,
    (E::Sources, S::Sources),
>
where
    V: SQLParam + 'a,
    V::DialectMarker: InstrPolicy,
    E: Expr<'a, V>,
    E::SQLType: Textual,
    S: Expr<'a, V>,
    S::SQLType: Textual,
    S::Nullable: Nullability,
    S::Aggregate: AggregateKind,
{
    SQLExpr::new(SQL::func(
        "INSTR",
        expr.into_sql().push(Token::COMMA).append(search.into_sql()),
    ))
}

/// Position of `search` within a text value (`STRPOS`), on PostgreSQL.
///
/// Returns the 1-based position of the first match, or 0 when there is none.
/// Both arguments must be text. The result is `int4`, nullable if either
/// argument is. On SQLite and MySQL, use [`instr`].
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
/// assert_eq!(strpos(users.name, "a").sql(), r#"STRPOS ("users"."name", $1)"#);
/// ```
#[allow(clippy::type_complexity)]
pub fn strpos<'a, V, E, S>(
    expr: E,
    search: S,
) -> SQLExpr<
    'a,
    V,
    drizzle_types::postgres::types::Int4,
    <E::Nullable as Nullability>::Or<S::Nullable>,
    <E::Aggregate as AggregateKind>::Or<S::Aggregate>,
    (E::Sources, S::Sources),
>
where
    V: SQLParam + 'a,
    V::DialectMarker: DialectSupports<feature::PostgresString>,
    E: Expr<'a, V>,
    E::SQLType: Textual,
    S: Expr<'a, V>,
    S::SQLType: Textual,
    S::Nullable: Nullability,
    S::Aggregate: AggregateKind,
{
    SQLExpr::new(SQL::func(
        "STRPOS",
        expr.into_sql().push(Token::COMMA).append(search.into_sql()),
    ))
}

// =============================================================================
// CONCAT (with NULL propagation)
// =============================================================================

/// Joins two text values.
///
/// Renders `left || right` on SQLite and PostgreSQL and `CONCAT(left, right)`
/// on MySQL, where `||` means logical OR by default. Both arguments must be
/// text. The result is text, nullable if either argument is (concatenating
/// NULL gives NULL). [`string_concat`](super::string_concat) is the same
/// function.
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
/// let label = concat(concat(users.name, " <"), concat(users.email, ">"));
/// assert_eq!(label.sql(), r#""users"."name" || ? || ("users"."email" || ?)"#);
/// ```
///
/// # Type safety
///
/// Joining an integer column does not compile:
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
/// let wrong = concat(users.id, users.name);
/// ```
#[allow(clippy::type_complexity)]
pub fn concat<'a, V, E1, E2>(
    expr1: E1,
    expr2: E2,
) -> SQLExpr<
    'a,
    V,
    <V::DialectMarker as DialectTypes>::Text,
    <E1::Nullable as Nullability>::Or<E2::Nullable>,
    <E1::Aggregate as AggregateKind>::Or<E2::Aggregate>,
    (E1::Sources, E2::Sources),
>
where
    V: SQLParam + 'a,
    E1: Expr<'a, V>,
    E1::SQLType: Textual,
    E2: Expr<'a, V>,
    E2::SQLType: Textual,
    E2::Nullable: Nullability,
    E2::Aggregate: AggregateKind,
{
    let left = expr1.into_sql();
    let right = expr2.into_sql();
    let sql = match V::DIALECT {
        Dialect::MySQL => SQL::func("CONCAT", left.push(Token::COMMA).append(right)),
        Dialect::SQLite | Dialect::PostgreSQL => {
            super::ops::binary_operator_sql(left, Token::CONCAT, right)
        }
    };
    SQLExpr::new(sql)
}

// =============================================================================
// CONCAT_WS (with separator)
// =============================================================================

/// Joins values with a separator, skipping NULLs (`CONCAT_WS`).
///
/// Renders `CONCAT_WS(sep, v1, v2, ...)`. The separator and values must be
/// text; the values share one Rust type. NULL values are skipped, so the
/// result is nullable only if the separator is. Needs SQLite 3.44 or later;
/// also available on PostgreSQL and MySQL.
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
/// let names = concat_ws(", ", [users.name, users.name]);
/// assert_eq!(names.sql(), r#"CONCAT_WS (?, "users"."name", "users"."name")"#);
/// ```
#[allow(clippy::type_complexity)]
pub fn concat_ws<'a, V, S, I>(
    sep: S,
    values: I,
) -> SQLExpr<
    'a,
    V,
    <V::DialectMarker as DialectTypes>::Text,
    S::Nullable,
    <S::Aggregate as AggregateKind>::Or<<I::Item as Expr<'a, V>>::Aggregate>,
    (S::Sources, <I::Item as ExprSources>::Sources),
>
where
    V: SQLParam + 'a,
    S: Expr<'a, V>,
    S::SQLType: Textual,
    I: IntoIterator,
    I::Item: Expr<'a, V>,
    <I::Item as Expr<'a, V>>::SQLType: Textual,
    <I::Item as Expr<'a, V>>::Aggregate: AggregateKind,
{
    let mut sql = sep.into_sql();
    for value in values {
        sql = sql.push(Token::COMMA).append(value.into_sql());
    }
    SQLExpr::new(SQL::func("CONCAT_WS", sql))
}

// =============================================================================
// Dialect-gated String Functions
// =============================================================================

/// The first `n` characters of a text value (`LEFT`), on PostgreSQL and MySQL.
///
/// `expr` must be text and `n` an integer. The result is text, nullable if
/// either argument is. SQLite has no `LEFT`; use [`substr`] there.
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
/// assert_eq!(left(users.name, 3).sql(), r#"LEFT ("users"."name", $1)"#);
/// ```
#[allow(clippy::type_complexity)]
pub fn left<'a, V, E, N>(
    expr: E,
    n: N,
) -> SQLExpr<
    'a,
    V,
    <V::DialectMarker as DialectTypes>::Text,
    <E::Nullable as Nullability>::Or<N::Nullable>,
    <E::Aggregate as AggregateKind>::Or<N::Aggregate>,
    (E::Sources, N::Sources),
>
where
    V: SQLParam + 'a,
    V::DialectMarker: DialectSupports<feature::LeftRight>,
    E: Expr<'a, V>,
    E::SQLType: Textual,
    N: Expr<'a, V>,
    N::SQLType: Integral,
    N::Nullable: Nullability,
    N::Aggregate: AggregateKind,
{
    SQLExpr::new(SQL::func(
        "LEFT",
        expr.into_sql().push(Token::COMMA).append(n.into_sql()),
    ))
}

/// The last `n` characters of a text value (`RIGHT`), on PostgreSQL and MySQL.
///
/// `expr` must be text and `n` an integer. The result is text, nullable if
/// either argument is. SQLite has no `RIGHT`; use [`substr`] there.
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
/// assert_eq!(right(users.name, 3).sql(), r#"RIGHT ("users"."name", $1)"#);
/// ```
#[allow(clippy::type_complexity)]
pub fn right<'a, V, E, N>(
    expr: E,
    n: N,
) -> SQLExpr<
    'a,
    V,
    <V::DialectMarker as DialectTypes>::Text,
    <E::Nullable as Nullability>::Or<N::Nullable>,
    <E::Aggregate as AggregateKind>::Or<N::Aggregate>,
    (E::Sources, N::Sources),
>
where
    V: SQLParam + 'a,
    V::DialectMarker: DialectSupports<feature::LeftRight>,
    E: Expr<'a, V>,
    E::SQLType: Textual,
    N: Expr<'a, V>,
    N::SQLType: Integral,
    N::Nullable: Nullability,
    N::Aggregate: AggregateKind,
{
    SQLExpr::new(SQL::func(
        "RIGHT",
        expr.into_sql().push(Token::COMMA).append(n.into_sql()),
    ))
}

/// The `n`-th field of a text value split on `delimiter` (`SPLIT_PART`), on PostgreSQL.
///
/// Fields are numbered from 1. `expr` and `delimiter` must be text and `n` an
/// integer. The result is text, nullable if any argument is.
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
/// // The domain part of an email address.
/// let domain = split_part(users.email, "@", 2);
/// assert_eq!(domain.sql(), r#"SPLIT_PART ("users"."email", $1, $2)"#);
/// ```
#[allow(clippy::type_complexity)]
pub fn split_part<'a, V, E, D, N>(
    expr: E,
    delimiter: D,
    n: N,
) -> SQLExpr<
    'a,
    V,
    <V::DialectMarker as DialectTypes>::Text,
    <<E::Nullable as Nullability>::Or<D::Nullable> as Nullability>::Or<N::Nullable>,
    <<E::Aggregate as AggregateKind>::Or<D::Aggregate> as AggregateKind>::Or<N::Aggregate>,
    (E::Sources, (D::Sources, N::Sources)),
>
where
    V: SQLParam + 'a,
    V::DialectMarker: DialectSupports<feature::PostgresString>,
    E: Expr<'a, V>,
    E::SQLType: Textual,
    D: Expr<'a, V>,
    D::SQLType: Textual,
    D::Nullable: Nullability,
    D::Aggregate: AggregateKind,
    N: Expr<'a, V>,
    N::SQLType: Integral,
    N::Nullable: Nullability,
    N::Aggregate: AggregateKind,
{
    SQLExpr::new(SQL::func(
        "SPLIT_PART",
        expr.into_sql()
            .push(Token::COMMA)
            .append(delimiter.into_sql())
            .push(Token::COMMA)
            .append(n.into_sql()),
    ))
}

/// Pads a text value on the left to `length` characters (`LPAD`), on PostgreSQL and MySQL.
///
/// `expr` and `fill` must be text and `length` an integer. Longer values are
/// cut to `length`. The result is text, nullable if any argument is.
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
/// let padded = lpad(users.name, 10, ".");
/// assert_eq!(padded.sql(), r#"LPAD ("users"."name", $1, $2)"#);
/// ```
#[allow(clippy::type_complexity)]
pub fn lpad<'a, V, E, L, F>(
    expr: E,
    length: L,
    fill: F,
) -> SQLExpr<
    'a,
    V,
    <V::DialectMarker as DialectTypes>::Text,
    <<E::Nullable as Nullability>::Or<L::Nullable> as Nullability>::Or<F::Nullable>,
    <<E::Aggregate as AggregateKind>::Or<L::Aggregate> as AggregateKind>::Or<F::Aggregate>,
    (E::Sources, (L::Sources, F::Sources)),
>
where
    V: SQLParam + 'a,
    V::DialectMarker: DialectSupports<feature::Pad>,
    E: Expr<'a, V>,
    E::SQLType: Textual,
    L: Expr<'a, V>,
    L::SQLType: Integral,
    L::Nullable: Nullability,
    L::Aggregate: AggregateKind,
    F: Expr<'a, V>,
    F::SQLType: Textual,
    F::Nullable: Nullability,
    F::Aggregate: AggregateKind,
{
    SQLExpr::new(SQL::func(
        "LPAD",
        expr.into_sql()
            .push(Token::COMMA)
            .append(length.into_sql())
            .push(Token::COMMA)
            .append(fill.into_sql()),
    ))
}

/// Pads a text value on the right to `length` characters (`RPAD`), on PostgreSQL and MySQL.
///
/// `expr` and `fill` must be text and `length` an integer. Longer values are
/// cut to `length`. The result is text, nullable if any argument is.
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
/// let padded = rpad(users.name, 10, ".");
/// assert_eq!(padded.sql(), r#"RPAD ("users"."name", $1, $2)"#);
/// ```
#[allow(clippy::type_complexity)]
pub fn rpad<'a, V, E, L, F>(
    expr: E,
    length: L,
    fill: F,
) -> SQLExpr<
    'a,
    V,
    <V::DialectMarker as DialectTypes>::Text,
    <<E::Nullable as Nullability>::Or<L::Nullable> as Nullability>::Or<F::Nullable>,
    <<E::Aggregate as AggregateKind>::Or<L::Aggregate> as AggregateKind>::Or<F::Aggregate>,
    (E::Sources, (L::Sources, F::Sources)),
>
where
    V: SQLParam + 'a,
    V::DialectMarker: DialectSupports<feature::Pad>,
    E: Expr<'a, V>,
    E::SQLType: Textual,
    L: Expr<'a, V>,
    L::SQLType: Integral,
    L::Nullable: Nullability,
    L::Aggregate: AggregateKind,
    F: Expr<'a, V>,
    F::SQLType: Textual,
    F::Nullable: Nullability,
    F::Aggregate: AggregateKind,
{
    SQLExpr::new(SQL::func(
        "RPAD",
        expr.into_sql()
            .push(Token::COMMA)
            .append(length.into_sql())
            .push(Token::COMMA)
            .append(fill.into_sql()),
    ))
}

/// Capitalizes the first letter of each word (`INITCAP`), on PostgreSQL.
///
/// The argument must be text. The result is text and keeps the argument's
/// nullability.
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
/// assert_eq!(initcap(users.name).sql(), r#"INITCAP ("users"."name")"#);
/// ```
#[allow(clippy::type_complexity)]
pub fn initcap<'a, V, E>(
    expr: E,
) -> SQLExpr<'a, V, <V::DialectMarker as DialectTypes>::Text, E::Nullable, E::Aggregate, E::Sources>
where
    V: SQLParam + 'a,
    V::DialectMarker: DialectSupports<feature::PostgresString>,
    E: Expr<'a, V>,
    E::SQLType: Textual,
{
    SQLExpr::new(SQL::func("INITCAP", expr.into_sql()))
}

/// Reverses a text value (`REVERSE`), on PostgreSQL and MySQL.
///
/// The argument must be text. The result is text and keeps the argument's
/// nullability.
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
/// assert_eq!(reverse(users.name).sql(), r#"REVERSE ("users"."name")"#);
/// ```
#[allow(clippy::type_complexity)]
pub fn reverse<'a, V, E>(
    expr: E,
) -> SQLExpr<'a, V, <V::DialectMarker as DialectTypes>::Text, E::Nullable, E::Aggregate, E::Sources>
where
    V: SQLParam + 'a,
    V::DialectMarker: DialectSupports<feature::Reverse>,
    E: Expr<'a, V>,
    E::SQLType: Textual,
{
    SQLExpr::new(SQL::func("REVERSE", expr.into_sql()))
}

/// Repeats a text value `n` times (`REPEAT`), on PostgreSQL and MySQL.
///
/// `expr` must be text and `n` an integer. The result is text, nullable if
/// either argument is.
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
/// assert_eq!(repeat(users.name, 2).sql(), r#"REPEAT ("users"."name", $1)"#);
/// ```
#[allow(clippy::type_complexity)]
pub fn repeat<'a, V, E, N>(
    expr: E,
    n: N,
) -> SQLExpr<
    'a,
    V,
    <V::DialectMarker as DialectTypes>::Text,
    <E::Nullable as Nullability>::Or<N::Nullable>,
    <E::Aggregate as AggregateKind>::Or<N::Aggregate>,
    (E::Sources, N::Sources),
>
where
    V: SQLParam + 'a,
    V::DialectMarker: DialectSupports<feature::Repeat>,
    E: Expr<'a, V>,
    E::SQLType: Textual,
    N: Expr<'a, V>,
    N::SQLType: Integral,
    N::Nullable: Nullability,
    N::Aggregate: AggregateKind,
{
    SQLExpr::new(SQL::func(
        "REPEAT",
        expr.into_sql().push(Token::COMMA).append(n.into_sql()),
    ))
}

/// Whether a text value starts with `prefix` (`STARTS_WITH`), on PostgreSQL.
///
/// Both arguments must be text. Like the comparison operators, the result is
/// the dialect's boolean, typed as non-null.
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
/// let cond = starts_with(users.email, "admin");
/// assert_eq!(cond.sql(), r#"STARTS_WITH ("users"."email", $1)"#);
/// ```
#[allow(clippy::type_complexity)]
pub fn starts_with<'a, V, E, P>(
    expr: E,
    prefix: P,
) -> SQLExpr<
    'a,
    V,
    <V::DialectMarker as DialectTypes>::Bool,
    NonNull,
    <E::Aggregate as AggregateKind>::Or<P::Aggregate>,
    (Arg<E::Nullable, E::Sources>, Arg<P::Nullable, P::Sources>),
>
where
    V: SQLParam + 'a,
    V::DialectMarker: DialectSupports<feature::PostgresString>,
    E: Expr<'a, V>,
    E::SQLType: Textual,
    P: Expr<'a, V>,
    P::SQLType: Textual,
    P::Aggregate: AggregateKind,
{
    SQLExpr::new(SQL::func(
        "STARTS_WITH",
        expr.into_sql().push(Token::COMMA).append(prefix.into_sql()),
    ))
}

// =============================================================================
// CHAR_LENGTH / OCTET_LENGTH (Standard SQL)
// =============================================================================

/// Number of characters in a text value.
///
/// Renders `CHAR_LENGTH(expr)` on PostgreSQL and MySQL and `LENGTH(expr)` on
/// SQLite, which counts characters. The argument must be text. The result is
/// an integer (see [`LengthPolicy`]) and keeps the argument's nullability.
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
/// // SQLite
/// assert_eq!(char_length(users.name).sql(), r#"LENGTH ("users"."name")"#);
/// ```
#[allow(clippy::type_complexity)]
pub fn char_length<'a, V, E>(
    expr: E,
) -> SQLExpr<
    'a,
    V,
    <E::SQLType as LengthPolicy<V::DialectMarker>>::Output,
    E::Nullable,
    E::Aggregate,
    E::Sources,
>
where
    V: SQLParam + 'a,
    E: Expr<'a, V>,
    E::SQLType: LengthPolicy<V::DialectMarker>,
{
    SQLExpr::new(SQL::func(
        <V::DialectMarker as DialectTypes>::CHAR_LENGTH_FN,
        expr.into_sql(),
    ))
}

/// Number of bytes in a text value (`OCTET_LENGTH`).
///
/// Needs SQLite 3.43 or later; also available on PostgreSQL and MySQL. The
/// argument must be text. The result is an integer (see [`LengthPolicy`]) and
/// keeps the argument's nullability.
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
/// assert_eq!(octet_length(users.name).sql(), r#"OCTET_LENGTH ("users"."name")"#);
/// ```
#[allow(clippy::type_complexity)]
pub fn octet_length<'a, V, E>(
    expr: E,
) -> SQLExpr<
    'a,
    V,
    <E::SQLType as LengthPolicy<V::DialectMarker>>::Output,
    E::Nullable,
    E::Aggregate,
    E::Sources,
>
where
    V: SQLParam + 'a,
    E: Expr<'a, V>,
    E::SQLType: LengthPolicy<V::DialectMarker>,
{
    SQLExpr::new(SQL::func("OCTET_LENGTH", expr.into_sql()))
}

// =============================================================================
// TRANSLATE (PostgreSQL)
// =============================================================================

/// Replaces characters one for one (`TRANSLATE`), on PostgreSQL.
///
/// Each character of `from` is replaced by the character at the same position
/// in `to`; characters of `from` with no partner in `to` are removed. All
/// arguments must be text. The result is text and keeps `expr`'s
/// nullability.
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
/// // Strip "(", ")" and "-".
/// let digits = translate(users.name, "()-", "");
/// assert_eq!(digits.sql(), r#"TRANSLATE ("users"."name", $1, $2)"#);
/// ```
#[allow(clippy::type_complexity)]
pub fn translate<'a, V, E, F, T>(
    expr: E,
    from: F,
    to: T,
) -> SQLExpr<
    'a,
    V,
    <V::DialectMarker as DialectTypes>::Text,
    E::Nullable,
    <<E::Aggregate as AggregateKind>::Or<F::Aggregate> as AggregateKind>::Or<T::Aggregate>,
    (E::Sources, (F::Sources, T::Sources)),
>
where
    V: SQLParam + 'a,
    V::DialectMarker: DialectSupports<feature::PostgresString>,
    E: Expr<'a, V>,
    E::SQLType: Textual,
    F: Expr<'a, V>,
    F::SQLType: Textual,
    F::Aggregate: AggregateKind,
    T: Expr<'a, V>,
    T::SQLType: Textual,
    T::Aggregate: AggregateKind,
{
    SQLExpr::new(SQL::func(
        "TRANSLATE",
        expr.into_sql()
            .push(Token::COMMA)
            .append(from.into_sql())
            .push(Token::COMMA)
            .append(to.into_sql()),
    ))
}

// =============================================================================
// REGEXP_REPLACE / REGEXP_MATCH (PostgreSQL)
// =============================================================================

/// Replaces the first POSIX regular expression match (`REGEXP_REPLACE`), on PostgreSQL.
///
/// All arguments must be text. The result is text and keeps `expr`'s
/// nullability. To replace every match, use [`regexp_replace_flags`] with
/// `"g"`.
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
/// let cleaned = regexp_replace(users.name, "[^a-z]", "");
/// assert_eq!(cleaned.sql(), r#"REGEXP_REPLACE ("users"."name", $1, $2)"#);
/// ```
#[allow(clippy::type_complexity)]
pub fn regexp_replace<'a, V, E, P, R>(
    expr: E,
    pattern: P,
    replacement: R,
) -> SQLExpr<
    'a,
    V,
    <V::DialectMarker as DialectTypes>::Text,
    E::Nullable,
    <<E::Aggregate as AggregateKind>::Or<P::Aggregate> as AggregateKind>::Or<R::Aggregate>,
    (E::Sources, (P::Sources, R::Sources)),
>
where
    V: SQLParam + 'a,
    V::DialectMarker: DialectSupports<feature::PostgresString>,
    E: Expr<'a, V>,
    E::SQLType: Textual,
    P: Expr<'a, V>,
    P::SQLType: Textual,
    P::Aggregate: AggregateKind,
    R: Expr<'a, V>,
    R::SQLType: Textual,
    R::Aggregate: AggregateKind,
{
    SQLExpr::new(SQL::func(
        "REGEXP_REPLACE",
        expr.into_sql()
            .push(Token::COMMA)
            .append(pattern.into_sql())
            .push(Token::COMMA)
            .append(replacement.into_sql()),
    ))
}

/// Replaces POSIX regular expression matches, with flags, on PostgreSQL.
///
/// Like [`regexp_replace`] with a fourth argument: renders
/// `REGEXP_REPLACE(expr, pattern, replacement, flags)`.
///
/// Common flags: `"g"` (every match), `"i"` (ignore case), `"gi"` (both). All
/// arguments must be text. The result is text and keeps `expr`'s
/// nullability.
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
/// let digits = regexp_replace_flags(users.name, "[^0-9]", "", "g");
/// assert_eq!(digits.sql(), r#"REGEXP_REPLACE ("users"."name", $1, $2, $3)"#);
/// ```
#[allow(clippy::type_complexity)]
pub fn regexp_replace_flags<'a, V, E, P, R, F>(
    expr: E,
    pattern: P,
    replacement: R,
    flags: F,
) -> SQLExpr<
    'a,
    V,
    <V::DialectMarker as DialectTypes>::Text,
    E::Nullable,
    <<<E::Aggregate as AggregateKind>::Or<P::Aggregate> as AggregateKind>::Or<R::Aggregate> as AggregateKind>::Or<F::Aggregate,>,
    (E::Sources, (P::Sources, (R::Sources, F::Sources))),
>
where
    V: SQLParam + 'a,
    V::DialectMarker: DialectSupports<feature::PostgresString>,
    E: Expr<'a, V>,
    E::SQLType: Textual,
    P: Expr<'a, V>,
    P::SQLType: Textual,
    P::Aggregate: AggregateKind,
    R: Expr<'a, V>,
    R::SQLType: Textual,
    R::Aggregate: AggregateKind,
    F: Expr<'a, V>,
    F::SQLType: Textual,
    F::Aggregate: AggregateKind,
{
    SQLExpr::new(SQL::func(
        "REGEXP_REPLACE",
        expr.into_sql()
            .push(Token::COMMA)
            .append(pattern.into_sql())
            .push(Token::COMMA)
            .append(replacement.into_sql())
            .push(Token::COMMA)
            .append(flags.into_sql()),
    ))
}

/// Capture groups of the first POSIX regular expression match (`REGEXP_MATCH`), on PostgreSQL.
///
/// Returns a text array with one element per capture group, or the whole
/// match when the pattern has no groups. Both arguments must be text. The
/// result is always nullable: it is NULL when nothing matches.
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
/// let parts = regexp_match(users.email, "(.+)@(.+)");
/// assert_eq!(parts.sql(), r#"REGEXP_MATCH ("users"."email", $1)"#);
/// ```
#[allow(clippy::type_complexity)]
pub fn regexp_match<'a, V, E, P>(
    expr: E,
    pattern: P,
) -> SQLExpr<
    'a,
    V,
    crate::types::Array<<V::DialectMarker as DialectTypes>::Text>,
    super::Null,
    <E::Aggregate as AggregateKind>::Or<P::Aggregate>,
    (E::Sources, P::Sources),
>
where
    V: SQLParam + 'a,
    V::DialectMarker: DialectSupports<feature::PostgresString>,
    E: Expr<'a, V>,
    E::SQLType: Textual,
    P: Expr<'a, V>,
    P::SQLType: Textual,
    P::Aggregate: AggregateKind,
{
    SQLExpr::new(SQL::func(
        "REGEXP_MATCH",
        expr.into_sql()
            .push(Token::COMMA)
            .append(pattern.into_sql()),
    ))
}

/// Capture groups of the first POSIX regular expression match, with flags,
/// on PostgreSQL.
///
/// Like [`regexp_match`] with a third argument: renders
/// `REGEXP_MATCH(expr, pattern, flags)`.
///
/// A common flag is `"i"` (ignore case). The `"g"` flag is not allowed here.
/// All arguments must be text. The result is a nullable text array.
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
/// let parts = regexp_match_flags(users.email, "(.+)@(.+)", "i");
/// assert_eq!(parts.sql(), r#"REGEXP_MATCH ("users"."email", $1, $2)"#);
/// ```
#[allow(clippy::type_complexity)]
pub fn regexp_match_flags<'a, V, E, P, F>(
    expr: E,
    pattern: P,
    flags: F,
) -> SQLExpr<
    'a,
    V,
    crate::types::Array<<V::DialectMarker as DialectTypes>::Text>,
    super::Null,
    <<E::Aggregate as AggregateKind>::Or<P::Aggregate> as AggregateKind>::Or<F::Aggregate>,
    (E::Sources, (P::Sources, F::Sources)),
>
where
    V: SQLParam + 'a,
    V::DialectMarker: DialectSupports<feature::PostgresString>,
    E: Expr<'a, V>,
    E::SQLType: Textual,
    P: Expr<'a, V>,
    P::SQLType: Textual,
    P::Aggregate: AggregateKind,
    F: Expr<'a, V>,
    F::SQLType: Textual,
    F::Aggregate: AggregateKind,
{
    SQLExpr::new(SQL::func(
        "REGEXP_MATCH",
        expr.into_sql()
            .push(Token::COMMA)
            .append(pattern.into_sql())
            .push(Token::COMMA)
            .append(flags.into_sql()),
    ))
}

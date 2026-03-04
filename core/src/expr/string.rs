//! Type-safe string functions.
//!
//! These functions require `Textual` types (Text, VarChar) and provide
//! compile-time enforcement of string operations.
//!
//! # Type Safety
//!
//! - `upper`, `lower`, `trim`: Require `Textual` types
//! - `length`: Dialect-aware integer output from text input
//! - `substr`, `replace`, `instr`: Require `Textual` types

use crate::dialect::DialectTypes;
use crate::sql::{SQL, Token};
use crate::traits::{SQLParam, ToSQL};
use crate::types::{DataType, Integral, Textual};
use crate::{PostgresDialect, SQLiteDialect};
use drizzle_types::postgres::types::{
    Char as PgChar, Int4 as PgInt4, Text as PgText, Varchar as PgVarchar,
};
use drizzle_types::sqlite::types::{Integer as SqliteInteger, Text as SqliteText};

use super::{AggOr, AggregateKind, Expr, NonNull, NullOr, Nullability, SQLExpr};

#[diagnostic::on_unimplemented(
    message = "no length policy for `{Self}` on this dialect",
    label = "length return type is not defined for this SQL type/dialect"
)]
pub trait LengthPolicy<D>: DataType {
    type Output: DataType;
}

#[diagnostic::on_unimplemented(
    message = "this string function is not available for this dialect",
    label = "use a dialect-specific alternative"
)]
pub trait SQLiteStringSupport {}

#[diagnostic::on_unimplemented(
    message = "this string function is not available for this dialect",
    label = "use a dialect-specific alternative"
)]
pub trait PostgresStringSupport {}

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

impl SQLiteStringSupport for SQLiteDialect {}
impl PostgresStringSupport for PostgresDialect {}

// =============================================================================
// CASE CONVERSION
// =============================================================================

/// UPPER - converts string to uppercase.
///
/// Preserves the nullability of the input expression.
///
/// # Type Safety
///
/// ```ignore
/// // ✅ OK: Text column
/// upper(users.name);
///
/// // ❌ Compile error: Int is not Textual
/// upper(users.id);
/// ```
pub fn upper<'a, V, E>(
    expr: E,
) -> SQLExpr<'a, V, <V::DialectMarker as DialectTypes>::Text, E::Nullable, E::Aggregate>
where
    V: SQLParam + 'a,
    E: Expr<'a, V>,
    E::SQLType: Textual,
{
    SQLExpr::new(SQL::func("UPPER", expr.into_sql()))
}

/// LOWER - converts string to lowercase.
///
/// Preserves the nullability of the input expression.
///
/// # Example
///
/// ```ignore
/// use drizzle_core::expr::lower;
///
/// // SELECT LOWER(users.email)
/// let email_lower = lower(users.email);
/// ```
pub fn lower<'a, V, E>(
    expr: E,
) -> SQLExpr<'a, V, <V::DialectMarker as DialectTypes>::Text, E::Nullable, E::Aggregate>
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

/// TRIM - removes leading and trailing whitespace.
///
/// Preserves the nullability of the input expression.
///
/// # Example
///
/// ```ignore
/// use drizzle_core::expr::trim;
///
/// // SELECT TRIM(users.name)
/// let trimmed = trim(users.name);
/// ```
pub fn trim<'a, V, E>(
    expr: E,
) -> SQLExpr<'a, V, <V::DialectMarker as DialectTypes>::Text, E::Nullable, E::Aggregate>
where
    V: SQLParam + 'a,
    E: Expr<'a, V>,
    E::SQLType: Textual,
{
    SQLExpr::new(SQL::func("TRIM", expr.into_sql()))
}

/// LTRIM - removes leading whitespace.
///
/// Preserves the nullability of the input expression.
pub fn ltrim<'a, V, E>(
    expr: E,
) -> SQLExpr<'a, V, <V::DialectMarker as DialectTypes>::Text, E::Nullable, E::Aggregate>
where
    V: SQLParam + 'a,
    E: Expr<'a, V>,
    E::SQLType: Textual,
{
    SQLExpr::new(SQL::func("LTRIM", expr.into_sql()))
}

/// RTRIM - removes trailing whitespace.
///
/// Preserves the nullability of the input expression.
pub fn rtrim<'a, V, E>(
    expr: E,
) -> SQLExpr<'a, V, <V::DialectMarker as DialectTypes>::Text, E::Nullable, E::Aggregate>
where
    V: SQLParam + 'a,
    E: Expr<'a, V>,
    E::SQLType: Textual,
{
    SQLExpr::new(SQL::func("RTRIM", expr.into_sql()))
}

// =============================================================================
// LENGTH
// =============================================================================

/// LENGTH - returns the length of a string.
///
/// Returns a dialect-aware integer type, preserves nullability.
///
/// # Example
///
/// ```ignore
/// use drizzle_core::expr::length;
///
/// // SELECT LENGTH(users.name)
/// let name_len = length(users.name);
/// ```
#[allow(clippy::type_complexity)]
pub fn length<'a, V, E>(
    expr: E,
) -> SQLExpr<'a, V, <E::SQLType as LengthPolicy<V::DialectMarker>>::Output, E::Nullable, E::Aggregate>
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

/// SUBSTR - extracts a substring from a string.
///
/// Extracts `len` characters starting at position `start` (1-indexed).
/// Preserves the nullability of the input expression.
///
/// # Example
///
/// ```ignore
/// use drizzle_core::expr::substr;
///
/// // SELECT SUBSTR(users.name, 1, 3) -- first 3 characters
/// let prefix = substr(users.name, 1, 3);
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
    E::Nullable,
    <<E::Aggregate as AggOr<S::Aggregate>>::Output as AggOr<L::Aggregate>>::Output,
>
where
    V: SQLParam + 'a,
    E: Expr<'a, V>,
    E::SQLType: Textual,
    S: Expr<'a, V>,
    S::SQLType: Integral,
    S::Aggregate: AggregateKind,
    L: Expr<'a, V>,
    L::SQLType: Integral,
    L::Aggregate: AggregateKind,
    E::Aggregate: AggOr<S::Aggregate>,
    <E::Aggregate as AggOr<S::Aggregate>>::Output: AggOr<L::Aggregate>,
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

/// REPLACE - replaces occurrences of a substring.
///
/// Replaces all occurrences of `from` with `to` in the expression.
/// Preserves the nullability of the input expression.
///
/// # Example
///
/// ```ignore
/// use drizzle_core::expr::replace;
///
/// // SELECT REPLACE(users.email, '@old.com', '@new.com')
/// let new_email = replace(users.email, "@old.com", "@new.com");
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
    E::Nullable,
    <<E::Aggregate as AggOr<F::Aggregate>>::Output as AggOr<T::Aggregate>>::Output,
>
where
    V: SQLParam + 'a,
    E: Expr<'a, V>,
    E::SQLType: Textual,
    F: Expr<'a, V>,
    F::SQLType: Textual,
    F::Aggregate: AggregateKind,
    T: Expr<'a, V>,
    T::SQLType: Textual,
    T::Aggregate: AggregateKind,
    E::Aggregate: AggOr<F::Aggregate>,
    <E::Aggregate as AggOr<F::Aggregate>>::Output: AggOr<T::Aggregate>,
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

/// INSTR - finds the position of a substring.
///
/// Returns the 1-indexed position of the first occurrence of `search`
/// in the expression, or 0 if not found. Returns SQLite INTEGER.
/// Preserves the nullability of the input expression.
///
/// # Example
///
/// ```ignore
/// use drizzle_core::expr::instr;
///
/// // SELECT INSTR(users.email, '@')
/// let at_pos = instr(users.email, "@");
/// ```
#[allow(clippy::type_complexity)]
pub fn instr<'a, V, E, S>(
    expr: E,
    search: S,
) -> SQLExpr<
    'a,
    V,
    drizzle_types::sqlite::types::Integer,
    E::Nullable,
    <E::Aggregate as AggOr<S::Aggregate>>::Output,
>
where
    V: SQLParam + 'a,
    V::DialectMarker: SQLiteStringSupport,
    E: Expr<'a, V>,
    E::SQLType: Textual,
    S: Expr<'a, V>,
    S::SQLType: Textual,
    S::Aggregate: AggregateKind,
    E::Aggregate: AggOr<S::Aggregate>,
{
    SQLExpr::new(SQL::func(
        "INSTR",
        expr.into_sql().push(Token::COMMA).append(search.into_sql()),
    ))
}

/// STRPOS - finds the position of a substring (PostgreSQL).
#[allow(clippy::type_complexity)]
pub fn strpos<'a, V, E, S>(
    expr: E,
    search: S,
) -> SQLExpr<
    'a,
    V,
    drizzle_types::postgres::types::Int4,
    E::Nullable,
    <E::Aggregate as AggOr<S::Aggregate>>::Output,
>
where
    V: SQLParam + 'a,
    V::DialectMarker: PostgresStringSupport,
    E: Expr<'a, V>,
    E::SQLType: Textual,
    S: Expr<'a, V>,
    S::SQLType: Textual,
    S::Aggregate: AggregateKind,
    E::Aggregate: AggOr<S::Aggregate>,
{
    SQLExpr::new(SQL::func(
        "STRPOS",
        expr.into_sql().push(Token::COMMA).append(search.into_sql()),
    ))
}

// =============================================================================
// CONCAT (with NULL propagation)
// =============================================================================

/// Concatenate two string expressions using || operator.
///
/// Nullability follows SQL concatenation rules: if either input is nullable,
/// the result is nullable. `string_concat` is a compatibility alias.
///
/// # Type Safety
///
/// ```ignore
/// // ✅ OK: Both are Text
/// concat(users.first_name, users.last_name);
///
/// // ✅ OK: Text with string literal
/// concat(users.first_name, " ");
///
/// // ❌ Compile error: Int is not Textual
/// concat(users.id, users.name);
/// ```
///
/// # Example
///
/// ```ignore
/// use drizzle_core::expr::concat;
///
/// // SELECT users.first_name || ' ' || users.last_name
/// let full_name = concat(concat(users.first_name, " "), users.last_name);
/// ```
#[allow(clippy::type_complexity)]
pub fn concat<'a, V, E1, E2>(
    expr1: E1,
    expr2: E2,
) -> SQLExpr<
    'a,
    V,
    <V::DialectMarker as DialectTypes>::Text,
    <E1::Nullable as NullOr<E2::Nullable>>::Output,
    <E1::Aggregate as AggOr<E2::Aggregate>>::Output,
>
where
    V: SQLParam + 'a,
    E1: Expr<'a, V>,
    E1::SQLType: Textual,
    E2: Expr<'a, V>,
    E2::SQLType: Textual,
    E1::Nullable: NullOr<E2::Nullable>,
    E2::Nullable: Nullability,
    E2::Aggregate: AggregateKind,
    E1::Aggregate: AggOr<E2::Aggregate>,
{
    SQLExpr::new(
        expr1
            .into_sql()
            .push(Token::CONCAT)
            .append(expr2.into_sql()),
    )
}

// =============================================================================
// CONCAT_WS (with separator)
// =============================================================================

/// CONCAT_WS - concatenates values with a separator, skipping NULLs.
///
/// Unlike `||`, CONCAT_WS skips NULL values and never returns NULL
/// (unless the separator itself is NULL).
///
/// Supported by both SQLite (3.44+) and PostgreSQL.
///
/// # Example
///
/// ```ignore
/// use drizzle_core::expr::concat_ws;
///
/// // SELECT CONCAT_WS(', ', users.city, users.state, users.country)
/// let location = concat_ws(", ", [users.city, users.state, users.country]);
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
    <S::Aggregate as AggOr<<I::Item as Expr<'a, V>>::Aggregate>>::Output,
>
where
    V: SQLParam + 'a,
    S: Expr<'a, V>,
    S::SQLType: Textual,
    I: IntoIterator,
    I::Item: Expr<'a, V>,
    <I::Item as Expr<'a, V>>::SQLType: Textual,
    S::Aggregate: AggOr<<I::Item as Expr<'a, V>>::Aggregate>,
    <I::Item as Expr<'a, V>>::Aggregate: AggregateKind,
{
    let mut sql = sep.into_sql();
    for value in values {
        sql = sql.push(Token::COMMA).append(value.into_sql());
    }
    SQLExpr::new(SQL::func("CONCAT_WS", sql))
}

// =============================================================================
// PostgreSQL-specific String Functions
// =============================================================================

/// LEFT - returns the first n characters of a string (PostgreSQL).
///
/// Preserves the nullability of the input expression.
///
/// # Example
///
/// ```ignore
/// use drizzle_core::expr::left;
///
/// // SELECT LEFT(users.name, 3)
/// let prefix = left(users.name, 3);
/// ```
#[allow(clippy::type_complexity)]
pub fn left<'a, V, E, N>(
    expr: E,
    n: N,
) -> SQLExpr<
    'a,
    V,
    <V::DialectMarker as DialectTypes>::Text,
    E::Nullable,
    <E::Aggregate as AggOr<N::Aggregate>>::Output,
>
where
    V: SQLParam + 'a,
    V::DialectMarker: PostgresStringSupport,
    E: Expr<'a, V>,
    E::SQLType: Textual,
    N: Expr<'a, V>,
    N::SQLType: Integral,
    N::Aggregate: AggregateKind,
    E::Aggregate: AggOr<N::Aggregate>,
{
    SQLExpr::new(SQL::func(
        "LEFT",
        expr.into_sql().push(Token::COMMA).append(n.into_sql()),
    ))
}

/// RIGHT - returns the last n characters of a string (PostgreSQL).
///
/// Preserves the nullability of the input expression.
///
/// # Example
///
/// ```ignore
/// use drizzle_core::expr::right;
///
/// // SELECT RIGHT(users.phone, 4)
/// let last_four = right(users.phone, 4);
/// ```
#[allow(clippy::type_complexity)]
pub fn right<'a, V, E, N>(
    expr: E,
    n: N,
) -> SQLExpr<
    'a,
    V,
    <V::DialectMarker as DialectTypes>::Text,
    E::Nullable,
    <E::Aggregate as AggOr<N::Aggregate>>::Output,
>
where
    V: SQLParam + 'a,
    V::DialectMarker: PostgresStringSupport,
    E: Expr<'a, V>,
    E::SQLType: Textual,
    N: Expr<'a, V>,
    N::SQLType: Integral,
    N::Aggregate: AggregateKind,
    E::Aggregate: AggOr<N::Aggregate>,
{
    SQLExpr::new(SQL::func(
        "RIGHT",
        expr.into_sql().push(Token::COMMA).append(n.into_sql()),
    ))
}

/// SPLIT_PART - splits a string and returns the nth field (PostgreSQL).
///
/// Returns the field at position `n` (1-indexed) when splitting by `delimiter`.
///
/// # Example
///
/// ```ignore
/// use drizzle_core::expr::split_part;
///
/// // SELECT SPLIT_PART(users.email, '@', 2)  -- get domain
/// let domain = split_part(users.email, "@", 2);
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
    E::Nullable,
    <<E::Aggregate as AggOr<D::Aggregate>>::Output as AggOr<N::Aggregate>>::Output,
>
where
    V: SQLParam + 'a,
    V::DialectMarker: PostgresStringSupport,
    E: Expr<'a, V>,
    E::SQLType: Textual,
    D: Expr<'a, V>,
    D::SQLType: Textual,
    D::Aggregate: AggregateKind,
    N: Expr<'a, V>,
    N::SQLType: Integral,
    N::Aggregate: AggregateKind,
    E::Aggregate: AggOr<D::Aggregate>,
    <E::Aggregate as AggOr<D::Aggregate>>::Output: AggOr<N::Aggregate>,
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

/// LPAD - pads a string on the left to a specified length (PostgreSQL).
///
/// # Example
///
/// ```ignore
/// use drizzle_core::expr::lpad;
///
/// // SELECT LPAD(users.id::text, 5, '0')  -- zero-pad to 5 digits
/// let padded = lpad(users.code, 5, "0");
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
    E::Nullable,
    <<E::Aggregate as AggOr<L::Aggregate>>::Output as AggOr<F::Aggregate>>::Output,
>
where
    V: SQLParam + 'a,
    V::DialectMarker: PostgresStringSupport,
    E: Expr<'a, V>,
    E::SQLType: Textual,
    L: Expr<'a, V>,
    L::SQLType: Integral,
    L::Aggregate: AggregateKind,
    F: Expr<'a, V>,
    F::SQLType: Textual,
    F::Aggregate: AggregateKind,
    E::Aggregate: AggOr<L::Aggregate>,
    <E::Aggregate as AggOr<L::Aggregate>>::Output: AggOr<F::Aggregate>,
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

/// RPAD - pads a string on the right to a specified length (PostgreSQL).
///
/// # Example
///
/// ```ignore
/// use drizzle_core::expr::rpad;
///
/// // SELECT RPAD(users.name, 20, '.')
/// let padded = rpad(users.name, 20, ".");
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
    E::Nullable,
    <<E::Aggregate as AggOr<L::Aggregate>>::Output as AggOr<F::Aggregate>>::Output,
>
where
    V: SQLParam + 'a,
    V::DialectMarker: PostgresStringSupport,
    E: Expr<'a, V>,
    E::SQLType: Textual,
    L: Expr<'a, V>,
    L::SQLType: Integral,
    L::Aggregate: AggregateKind,
    F: Expr<'a, V>,
    F::SQLType: Textual,
    F::Aggregate: AggregateKind,
    E::Aggregate: AggOr<L::Aggregate>,
    <E::Aggregate as AggOr<L::Aggregate>>::Output: AggOr<F::Aggregate>,
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

/// INITCAP - converts the first letter of each word to uppercase (PostgreSQL).
///
/// Preserves the nullability of the input expression.
pub fn initcap<'a, V, E>(
    expr: E,
) -> SQLExpr<'a, V, <V::DialectMarker as DialectTypes>::Text, E::Nullable, E::Aggregate>
where
    V: SQLParam + 'a,
    V::DialectMarker: PostgresStringSupport,
    E: Expr<'a, V>,
    E::SQLType: Textual,
{
    SQLExpr::new(SQL::func("INITCAP", expr.into_sql()))
}

/// REVERSE - reverses a string (PostgreSQL).
///
/// Preserves the nullability of the input expression.
pub fn reverse<'a, V, E>(
    expr: E,
) -> SQLExpr<'a, V, <V::DialectMarker as DialectTypes>::Text, E::Nullable, E::Aggregate>
where
    V: SQLParam + 'a,
    V::DialectMarker: PostgresStringSupport,
    E: Expr<'a, V>,
    E::SQLType: Textual,
{
    SQLExpr::new(SQL::func("REVERSE", expr.into_sql()))
}

/// REPEAT - repeats a string n times (PostgreSQL).
///
/// # Example
///
/// ```ignore
/// use drizzle_core::expr::repeat;
///
/// // SELECT REPEAT('-', 40)
/// let separator = repeat("-", 40);
/// ```
#[allow(clippy::type_complexity)]
pub fn repeat<'a, V, E, N>(
    expr: E,
    n: N,
) -> SQLExpr<
    'a,
    V,
    <V::DialectMarker as DialectTypes>::Text,
    E::Nullable,
    <E::Aggregate as AggOr<N::Aggregate>>::Output,
>
where
    V: SQLParam + 'a,
    V::DialectMarker: PostgresStringSupport,
    E: Expr<'a, V>,
    E::SQLType: Textual,
    N: Expr<'a, V>,
    N::SQLType: Integral,
    N::Aggregate: AggregateKind,
    E::Aggregate: AggOr<N::Aggregate>,
{
    SQLExpr::new(SQL::func(
        "REPEAT",
        expr.into_sql().push(Token::COMMA).append(n.into_sql()),
    ))
}

/// STARTS_WITH - tests if a string starts with a prefix (PostgreSQL).
///
/// Returns a boolean expression. Follows comparison operator convention
/// of returning NonNull.
///
/// # Example
///
/// ```ignore
/// use drizzle_core::expr::starts_with;
///
/// // SELECT * FROM users WHERE STARTS_WITH(email, 'admin')
/// let is_admin = starts_with(users.email, "admin");
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
    <E::Aggregate as AggOr<P::Aggregate>>::Output,
>
where
    V: SQLParam + 'a,
    V::DialectMarker: PostgresStringSupport,
    E: Expr<'a, V>,
    E::SQLType: Textual,
    P: Expr<'a, V>,
    P::SQLType: Textual,
    P::Aggregate: AggregateKind,
    E::Aggregate: AggOr<P::Aggregate>,
{
    SQLExpr::new(SQL::func(
        "STARTS_WITH",
        expr.into_sql().push(Token::COMMA).append(prefix.into_sql()),
    ))
}

// =============================================================================
// CHAR_LENGTH / OCTET_LENGTH (Standard SQL)
// =============================================================================

/// Dialect-aware function name for CHAR_LENGTH.
///
/// PostgreSQL uses `CHAR_LENGTH`; SQLite uses `LENGTH`.
pub trait CharLengthPolicy {
    const CHAR_LENGTH_FN: &'static str;
}

impl CharLengthPolicy for SQLiteDialect {
    const CHAR_LENGTH_FN: &'static str = "LENGTH";
}

impl CharLengthPolicy for PostgresDialect {
    const CHAR_LENGTH_FN: &'static str = "CHAR_LENGTH";
}

/// CHAR_LENGTH - returns the number of characters in a string.
///
/// Standard SQL function. Emits `CHAR_LENGTH` on PostgreSQL, `LENGTH` on SQLite.
///
/// # Example
///
/// ```ignore
/// use drizzle_core::expr::char_length;
///
/// // SELECT CHAR_LENGTH(users.name)  -- PG
/// // SELECT LENGTH(users.name)       -- SQLite
/// let name_len = char_length(users.name);
/// ```
#[allow(clippy::type_complexity)]
pub fn char_length<'a, V, E>(
    expr: E,
) -> SQLExpr<'a, V, <E::SQLType as LengthPolicy<V::DialectMarker>>::Output, E::Nullable, E::Aggregate>
where
    V: SQLParam + 'a,
    V::DialectMarker: CharLengthPolicy,
    E: Expr<'a, V>,
    E::SQLType: LengthPolicy<V::DialectMarker>,
{
    SQLExpr::new(SQL::func(
        <V::DialectMarker as CharLengthPolicy>::CHAR_LENGTH_FN,
        expr.into_sql(),
    ))
}

/// OCTET_LENGTH - returns the number of bytes in a string.
///
/// Standard SQL function. Works on both SQLite (3.43+) and PostgreSQL.
///
/// # Example
///
/// ```ignore
/// use drizzle_core::expr::octet_length;
///
/// // SELECT OCTET_LENGTH(users.name)
/// let byte_len = octet_length(users.name);
/// ```
#[allow(clippy::type_complexity)]
pub fn octet_length<'a, V, E>(
    expr: E,
) -> SQLExpr<'a, V, <E::SQLType as LengthPolicy<V::DialectMarker>>::Output, E::Nullable, E::Aggregate>
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

/// TRANSLATE - replaces each character in `from` with the corresponding
/// character in `to` (PostgreSQL).
///
/// Characters in `from` that have no match in `to` are removed.
///
/// # Example
///
/// ```ignore
/// use drizzle_core::expr::translate;
///
/// // SELECT TRANSLATE(users.phone, '()-', '')
/// let clean_phone = translate(users.phone, "()-", "");
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
    <<E::Aggregate as AggOr<F::Aggregate>>::Output as AggOr<T::Aggregate>>::Output,
>
where
    V: SQLParam + 'a,
    V::DialectMarker: PostgresStringSupport,
    E: Expr<'a, V>,
    E::SQLType: Textual,
    F: Expr<'a, V>,
    F::SQLType: Textual,
    F::Aggregate: AggregateKind,
    T: Expr<'a, V>,
    T::SQLType: Textual,
    T::Aggregate: AggregateKind,
    E::Aggregate: AggOr<F::Aggregate>,
    <E::Aggregate as AggOr<F::Aggregate>>::Output: AggOr<T::Aggregate>,
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

/// REGEXP_REPLACE - replaces substrings matching a POSIX regex (PostgreSQL).
///
/// Replaces the first match of `pattern` in `expr` with `replacement`.
/// Use optional flags (e.g., `"g"` for global) via `regexp_replace_flags`.
///
/// # Example
///
/// ```ignore
/// use drizzle_core::expr::regexp_replace;
///
/// // SELECT REGEXP_REPLACE(users.phone, '[^0-9]', '')
/// let digits_only = regexp_replace(users.phone, "[^0-9]", "");
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
    <<E::Aggregate as AggOr<P::Aggregate>>::Output as AggOr<R::Aggregate>>::Output,
>
where
    V: SQLParam + 'a,
    V::DialectMarker: PostgresStringSupport,
    E: Expr<'a, V>,
    E::SQLType: Textual,
    P: Expr<'a, V>,
    P::SQLType: Textual,
    P::Aggregate: AggregateKind,
    R: Expr<'a, V>,
    R::SQLType: Textual,
    R::Aggregate: AggregateKind,
    E::Aggregate: AggOr<P::Aggregate>,
    <E::Aggregate as AggOr<P::Aggregate>>::Output: AggOr<R::Aggregate>,
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

/// REGEXP_REPLACE with flags - replaces substrings matching a POSIX regex (PostgreSQL).
///
/// Common flags: `"g"` (global), `"i"` (case-insensitive), `"gi"` (both).
///
/// # Example
///
/// ```ignore
/// use drizzle_core::expr::regexp_replace_flags;
///
/// // SELECT REGEXP_REPLACE(users.phone, '[^0-9]', '', 'g')
/// let digits_only = regexp_replace_flags(users.phone, "[^0-9]", "", "g");
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
    <<<E::Aggregate as AggOr<P::Aggregate>>::Output as AggOr<R::Aggregate>>::Output as AggOr<
        F::Aggregate,
    >>::Output,
>
where
    V: SQLParam + 'a,
    V::DialectMarker: PostgresStringSupport,
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
    E::Aggregate: AggOr<P::Aggregate>,
    <E::Aggregate as AggOr<P::Aggregate>>::Output: AggOr<R::Aggregate>,
    <<E::Aggregate as AggOr<P::Aggregate>>::Output as AggOr<R::Aggregate>>::Output:
        AggOr<F::Aggregate>,
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

/// REGEXP_MATCH - returns captured groups from the first POSIX regex match (PostgreSQL).
///
/// Returns a text array of captured groups. If the pattern has no groups,
/// the result is a single-element array with the whole match.
///
/// # Example
///
/// ```ignore
/// use drizzle_core::expr::regexp_match;
///
/// // SELECT REGEXP_MATCH(users.email, '(.+)@(.+)')
/// let parts = regexp_match(users.email, "(.+)@(.+)");
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
    <E::Aggregate as AggOr<P::Aggregate>>::Output,
>
where
    V: SQLParam + 'a,
    V::DialectMarker: PostgresStringSupport,
    E: Expr<'a, V>,
    E::SQLType: Textual,
    P: Expr<'a, V>,
    P::SQLType: Textual,
    P::Aggregate: AggregateKind,
    E::Aggregate: AggOr<P::Aggregate>,
{
    SQLExpr::new(SQL::func(
        "REGEXP_MATCH",
        expr.into_sql()
            .push(Token::COMMA)
            .append(pattern.into_sql()),
    ))
}

/// REGEXP_MATCH with flags (PostgreSQL).
///
/// Common flags: `"i"` (case-insensitive), `"g"` (not valid for regexp_match, use regexp_matches).
///
/// # Example
///
/// ```ignore
/// use drizzle_core::expr::regexp_match_flags;
///
/// // SELECT REGEXP_MATCH(users.email, '(.+)@(.+)', 'i')
/// let parts = regexp_match_flags(users.email, "(.+)@(.+)", "i");
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
    <<E::Aggregate as AggOr<P::Aggregate>>::Output as AggOr<F::Aggregate>>::Output,
>
where
    V: SQLParam + 'a,
    V::DialectMarker: PostgresStringSupport,
    E: Expr<'a, V>,
    E::SQLType: Textual,
    P: Expr<'a, V>,
    P::SQLType: Textual,
    P::Aggregate: AggregateKind,
    F: Expr<'a, V>,
    F::SQLType: Textual,
    F::Aggregate: AggregateKind,
    E::Aggregate: AggOr<P::Aggregate>,
    <E::Aggregate as AggOr<P::Aggregate>>::Output: AggOr<F::Aggregate>,
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

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

use crate::sql::{SQL, Token};
use crate::traits::SQLParam;
use crate::types::{DataType, Text, Textual, VarChar};
use crate::{PostgresDialect, SQLiteDialect};

use super::{Expr, NullOr, Nullability, SQLExpr, Scalar};

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

impl LengthPolicy<SQLiteDialect> for Text {
    type Output = drizzle_types::sqlite::types::Integer;
}
impl LengthPolicy<SQLiteDialect> for VarChar {
    type Output = drizzle_types::sqlite::types::Integer;
}
impl LengthPolicy<SQLiteDialect> for crate::types::Any {
    type Output = drizzle_types::sqlite::types::Integer;
}

impl LengthPolicy<PostgresDialect> for Text {
    type Output = drizzle_types::postgres::types::Int4;
}
impl LengthPolicy<PostgresDialect> for VarChar {
    type Output = drizzle_types::postgres::types::Int4;
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
pub fn upper<'a, V, E>(expr: E) -> SQLExpr<'a, V, Text, E::Nullable, Scalar>
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
pub fn lower<'a, V, E>(expr: E) -> SQLExpr<'a, V, Text, E::Nullable, Scalar>
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
pub fn trim<'a, V, E>(expr: E) -> SQLExpr<'a, V, Text, E::Nullable, Scalar>
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
pub fn ltrim<'a, V, E>(expr: E) -> SQLExpr<'a, V, Text, E::Nullable, Scalar>
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
pub fn rtrim<'a, V, E>(expr: E) -> SQLExpr<'a, V, Text, E::Nullable, Scalar>
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
) -> SQLExpr<'a, V, <E::SQLType as LengthPolicy<V::DialectMarker>>::Output, E::Nullable, Scalar>
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
pub fn substr<'a, V, E, S, L>(
    expr: E,
    start: S,
    len: L,
) -> SQLExpr<'a, V, Text, E::Nullable, Scalar>
where
    V: SQLParam + 'a,
    E: Expr<'a, V>,
    E::SQLType: Textual,
    S: Expr<'a, V>,
    L: Expr<'a, V>,
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
pub fn replace<'a, V, E, F, T>(expr: E, from: F, to: T) -> SQLExpr<'a, V, Text, E::Nullable, Scalar>
where
    V: SQLParam + 'a,
    E: Expr<'a, V>,
    E::SQLType: Textual,
    F: Expr<'a, V>,
    F::SQLType: Textual,
    T: Expr<'a, V>,
    T::SQLType: Textual,
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
pub fn instr<'a, V, E, S>(
    expr: E,
    search: S,
) -> SQLExpr<'a, V, drizzle_types::sqlite::types::Integer, E::Nullable, Scalar>
where
    V: SQLParam + 'a,
    V::DialectMarker: SQLiteStringSupport,
    E: Expr<'a, V>,
    E::SQLType: Textual,
    S: Expr<'a, V>,
    S::SQLType: Textual,
{
    SQLExpr::new(SQL::func(
        "INSTR",
        expr.into_sql().push(Token::COMMA).append(search.into_sql()),
    ))
}

/// STRPOS - finds the position of a substring (PostgreSQL).
pub fn strpos<'a, V, E, S>(
    expr: E,
    search: S,
) -> SQLExpr<'a, V, drizzle_types::postgres::types::Int4, E::Nullable, Scalar>
where
    V: SQLParam + 'a,
    V::DialectMarker: PostgresStringSupport,
    E: Expr<'a, V>,
    E::SQLType: Textual,
    S: Expr<'a, V>,
    S::SQLType: Textual,
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
pub fn concat<'a, V, E1, E2>(
    expr1: E1,
    expr2: E2,
) -> SQLExpr<'a, V, Text, <E1::Nullable as NullOr<E2::Nullable>>::Output, Scalar>
where
    V: SQLParam + 'a,
    E1: Expr<'a, V>,
    E1::SQLType: Textual,
    E2: Expr<'a, V>,
    E2::SQLType: Textual,
    E1::Nullable: NullOr<E2::Nullable>,
    E2::Nullable: Nullability,
{
    SQLExpr::new(
        expr1
            .into_sql()
            .push(Token::CONCAT)
            .append(expr2.into_sql()),
    )
}

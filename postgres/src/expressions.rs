//! PostgreSQL-specific expressions
//!
//! This module provides PostgreSQL dialect-specific SQL expressions.
//! For standard SQL expressions, use `drizzle_core::expressions`.

use crate::{PostgresSQL, PostgresValue};
use drizzle_core::traits::ToSQL;

/// Case-insensitive LIKE pattern matching (PostgreSQL-specific)
///
/// # Example
///
/// ```ignore
/// use drizzle::postgres::expressions::ilike;
///
/// let query = ilike(user.name, "%john%");
/// // Generates: "name" ILIKE '%john%'
/// ```
pub fn ilike<'a, L, R>(left: L, pattern: R) -> PostgresSQL<'a>
where
    L: ToSQL<'a, PostgresValue<'a>>,
    R: Into<PostgresValue<'a>> + ToSQL<'a, PostgresValue<'a>>,
{
    use drizzle_core::sql::SQLChunk;
    left.to_sql()
        .push(SQLChunk::Raw("ILIKE".into()))
        .append(pattern.to_sql())
}

/// Case-insensitive NOT LIKE pattern matching (PostgreSQL-specific)
///
/// # Example
///
/// ```ignore
/// use drizzle::postgres::expressions::not_ilike;
///
/// let query = not_ilike(user.name, "%admin%");
/// // Generates: "name" NOT ILIKE '%admin%'
/// ```
pub fn not_ilike<'a, L, R>(left: L, pattern: R) -> PostgresSQL<'a>
where
    L: ToSQL<'a, PostgresValue<'a>>,
    R: Into<PostgresValue<'a>> + ToSQL<'a, PostgresValue<'a>>,
{
    use drizzle_core::sql::{SQLChunk, Token};
    left.to_sql()
        .push(Token::NOT)
        .push(SQLChunk::Raw("ILIKE".into()))
        .append(pattern.to_sql())
}

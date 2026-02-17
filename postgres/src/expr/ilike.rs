//! PostgreSQL ILIKE operators.

use crate::values::PostgresValue;
use drizzle_core::{SQL, ToSQL};

/// Case-insensitive LIKE pattern matching (PostgreSQL-specific)
///
/// # Example
///
/// ```ignore
/// use drizzle_postgres::expr::ilike;
///
/// let query = ilike(user.name, "%john%");
/// // Generates: "name" ILIKE '%john%'
/// ```
pub fn ilike<'a, L, R>(left: L, pattern: R) -> SQL<'a, PostgresValue<'a>>
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
/// use drizzle_postgres::expr::not_ilike;
///
/// let query = not_ilike(user.name, "%admin%");
/// // Generates: "name" NOT ILIKE '%admin%'
/// ```
pub fn not_ilike<'a, L, R>(left: L, pattern: R) -> SQL<'a, PostgresValue<'a>>
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

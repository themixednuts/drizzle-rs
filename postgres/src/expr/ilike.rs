//! `PostgreSQL` ILIKE operators.

use crate::values::PostgresValue;
use drizzle_core::expr::{Expr, NonNull, SQLExpr, Scalar};
use drizzle_core::sql::{SQL, SQLChunk, Token};
use drizzle_types::postgres::types::Boolean;

/// Case-insensitive LIKE pattern matching (PostgreSQL-specific)
///
/// The result is a boolean expression, so it can be used directly in
/// `WHERE`, `HAVING` and join conditions.
///
/// # Example
///
/// ```
/// # use drizzle_postgres::expr::ilike;
/// # use drizzle_core::{SQL, ToSQL};
/// # use drizzle_postgres::values::PostgresValue;
/// let name = SQL::<PostgresValue>::raw("name");
/// let cond = ilike(name, "%john%");
/// assert!(cond.to_sql().sql().contains("ILIKE"));
/// ```
pub fn ilike<'a, E, P>(
    expr: E,
    pattern: P,
) -> SQLExpr<'a, PostgresValue<'a>, Boolean, NonNull, Scalar>
where
    E: Expr<'a, PostgresValue<'a>>,
    P: Into<PostgresValue<'a>>,
{
    SQLExpr::new(
        expr.to_sql()
            .push(SQLChunk::Raw("ILIKE".into()))
            .append(SQL::param(pattern.into())),
    )
}

/// Case-insensitive NOT LIKE pattern matching (PostgreSQL-specific)
///
/// # Example
///
/// ```
/// # use drizzle_postgres::expr::not_ilike;
/// # use drizzle_core::{SQL, ToSQL};
/// # use drizzle_postgres::values::PostgresValue;
/// let name = SQL::<PostgresValue>::raw("name");
/// let cond = not_ilike(name, "%admin%");
/// assert!(cond.to_sql().sql().contains("NOT ILIKE"));
/// ```
pub fn not_ilike<'a, E, P>(
    expr: E,
    pattern: P,
) -> SQLExpr<'a, PostgresValue<'a>, Boolean, NonNull, Scalar>
where
    E: Expr<'a, PostgresValue<'a>>,
    P: Into<PostgresValue<'a>>,
{
    SQLExpr::new(
        expr.to_sql()
            .push(Token::NOT)
            .push(SQLChunk::Raw("ILIKE".into()))
            .append(SQL::param(pattern.into())),
    )
}

//! PostgreSQL regular expression operators.
//!
//! Provides type-safe access to PostgreSQL regex operators:
//! - `~` (matches regex, case-sensitive)
//! - `~*` (matches regex, case-insensitive)
//! - `!~` (does not match regex, case-sensitive)
//! - `!~*` (does not match regex, case-insensitive)

use crate::values::PostgresValue;
use drizzle_core::expr::{Expr, NonNull, SQLExpr, Scalar};
use drizzle_core::sql::{SQL, SQLChunk};
use drizzle_types::postgres::types::Boolean;

/// PostgreSQL `~` operator - case-sensitive regex match.
///
/// # Example
///
/// ```
/// # use drizzle_postgres::expr::regex_match;
/// # use drizzle_core::{SQL, ToSQL};
/// # use drizzle_postgres::values::PostgresValue;
/// let name = SQL::<PostgresValue>::raw("name");
/// let cond = regex_match(name, "^[A-Z]");
/// assert!(cond.to_sql().sql().contains("~"));
/// ```
pub fn regex_match<'a, E>(
    expr: E,
    pattern: &'a str,
) -> SQLExpr<'a, PostgresValue<'a>, Boolean, NonNull, Scalar>
where
    E: Expr<'a, PostgresValue<'a>>,
{
    SQLExpr::new(
        expr.to_sql()
            .push(SQLChunk::Raw("~".into()))
            .append(SQL::param(PostgresValue::Text(pattern.into()))),
    )
}

/// PostgreSQL `~*` operator - case-insensitive regex match.
///
/// # Example
///
/// ```
/// # use drizzle_postgres::expr::regex_match_ci;
/// # use drizzle_core::{SQL, ToSQL};
/// # use drizzle_postgres::values::PostgresValue;
/// let name = SQL::<PostgresValue>::raw("name");
/// let cond = regex_match_ci(name, "^john");
/// assert!(cond.to_sql().sql().contains("~*"));
/// ```
pub fn regex_match_ci<'a, E>(
    expr: E,
    pattern: &'a str,
) -> SQLExpr<'a, PostgresValue<'a>, Boolean, NonNull, Scalar>
where
    E: Expr<'a, PostgresValue<'a>>,
{
    SQLExpr::new(
        expr.to_sql()
            .push(SQLChunk::Raw("~*".into()))
            .append(SQL::param(PostgresValue::Text(pattern.into()))),
    )
}

/// PostgreSQL `!~` operator - case-sensitive regex non-match.
///
/// # Example
///
/// ```
/// # use drizzle_postgres::expr::regex_not_match;
/// # use drizzle_core::{SQL, ToSQL};
/// # use drizzle_postgres::values::PostgresValue;
/// let name = SQL::<PostgresValue>::raw("name");
/// let cond = regex_not_match(name, "^[0-9]");
/// assert!(cond.to_sql().sql().contains("!~"));
/// ```
pub fn regex_not_match<'a, E>(
    expr: E,
    pattern: &'a str,
) -> SQLExpr<'a, PostgresValue<'a>, Boolean, NonNull, Scalar>
where
    E: Expr<'a, PostgresValue<'a>>,
{
    SQLExpr::new(
        expr.to_sql()
            .push(SQLChunk::Raw("!~".into()))
            .append(SQL::param(PostgresValue::Text(pattern.into()))),
    )
}

/// PostgreSQL `!~*` operator - case-insensitive regex non-match.
///
/// # Example
///
/// ```
/// # use drizzle_postgres::expr::regex_not_match_ci;
/// # use drizzle_core::{SQL, ToSQL};
/// # use drizzle_postgres::values::PostgresValue;
/// let name = SQL::<PostgresValue>::raw("name");
/// let cond = regex_not_match_ci(name, "^admin");
/// assert!(cond.to_sql().sql().contains("!~*"));
/// ```
pub fn regex_not_match_ci<'a, E>(
    expr: E,
    pattern: &'a str,
) -> SQLExpr<'a, PostgresValue<'a>, Boolean, NonNull, Scalar>
where
    E: Expr<'a, PostgresValue<'a>>,
{
    SQLExpr::new(
        expr.to_sql()
            .push(SQLChunk::Raw("!~*".into()))
            .append(SQL::param(PostgresValue::Text(pattern.into()))),
    )
}

/// Extension trait providing method-based regex operators for PostgreSQL expressions.
pub trait RegexExprExt<'a>: Expr<'a, PostgresValue<'a>> + Sized {
    /// Case-sensitive regex match (`~` operator).
    fn regex_match(
        self,
        pattern: &'a str,
    ) -> SQLExpr<'a, PostgresValue<'a>, Boolean, NonNull, Scalar> {
        regex_match(self, pattern)
    }

    /// Case-insensitive regex match (`~*` operator).
    fn regex_match_ci(
        self,
        pattern: &'a str,
    ) -> SQLExpr<'a, PostgresValue<'a>, Boolean, NonNull, Scalar> {
        regex_match_ci(self, pattern)
    }

    /// Case-sensitive regex non-match (`!~` operator).
    fn regex_not_match(
        self,
        pattern: &'a str,
    ) -> SQLExpr<'a, PostgresValue<'a>, Boolean, NonNull, Scalar> {
        regex_not_match(self, pattern)
    }

    /// Case-insensitive regex non-match (`!~*` operator).
    fn regex_not_match_ci(
        self,
        pattern: &'a str,
    ) -> SQLExpr<'a, PostgresValue<'a>, Boolean, NonNull, Scalar> {
        regex_not_match_ci(self, pattern)
    }
}

/// Blanket implementation for all PostgreSQL `Expr` types.
impl<'a, E: Expr<'a, PostgresValue<'a>>> RegexExprExt<'a> for E {}

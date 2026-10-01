//! `PostgreSQL` case-insensitive pattern matching: [`ilike`] and [`not_ilike`].
//!
//! Both operands must be text: the left side is a `text`, `varchar`, `char`
//! or enum expression ([`Textual`]), and the pattern must be a textual value
//! that is [`Compatible`] with it, such as a `&str` or a placeholder.

use crate::values::PostgresValue;
use drizzle_core::expr::{AggregateKind, ComparisonOperand, Expr, NonNull, SQLExpr};
use drizzle_core::scope::Arg;
use drizzle_core::sql::{SQLChunk, Token};
use drizzle_types::postgres::types::Boolean;
use drizzle_types::{Compatible, Textual};

/// Result of [`ilike`] and [`not_ilike`]: `boolean`, NULL when either operand
/// is NULL, and an aggregate when either operand is one.
type IlikeExpr<'a, E, P> = SQLExpr<
    'a,
    PostgresValue<'a>,
    Boolean,
    NonNull,
    <<E as Expr<'a, PostgresValue<'a>>>::Aggregate as AggregateKind>::Or<
        <P as ComparisonOperand<'a, PostgresValue<'a>, E>>::Aggregate,
    >,
    (
        Arg<
            <E as Expr<'a, PostgresValue<'a>>>::Nullable,
            <E as drizzle_core::expr::ExprSources>::Sources,
        >,
        Arg<
            <P as ComparisonOperand<'a, PostgresValue<'a>, E>>::Nullable,
            <P as ComparisonOperand<'a, PostgresValue<'a>, E>>::Sources,
        >,
    ),
>;

/// Matches a text expression against a `LIKE` pattern, ignoring case (`ILIKE`).
///
/// In the pattern, `%` matches any run of characters and `_` matches one
/// character. The result is a boolean condition for `WHERE`, `HAVING` or a
/// join, and is NULL when either operand is NULL.
///
/// # Examples
///
/// ```
/// use drizzle_core::{ToSQL, expr::raw_non_null};
/// use drizzle_postgres::expr::ilike;
/// use drizzle_postgres::values::PostgresValue;
/// use drizzle_types::postgres::types::Text;
///
/// // Stands in for a `name text NOT NULL` column.
/// let name = raw_non_null::<PostgresValue, Text>("name");
/// let cond = ilike(name, "%john%"); // matches "John", "JOHNNY", ...
/// assert_eq!(cond.to_sql().sql(), "name ILIKE $1");
/// ```
///
/// # Type safety
///
/// Both sides must be text.
///
/// ```compile_fail
/// use drizzle_core::expr::raw_non_null;
/// use drizzle_postgres::expr::ilike;
/// use drizzle_postgres::values::PostgresValue;
/// use drizzle_types::postgres::types::Int4;
///
/// let age = raw_non_null::<PostgresValue, Int4>("age");
/// let _ = ilike(age, "%1%"); // `int4` is not textual
/// ```
pub fn ilike<'a, E, P>(expr: E, pattern: P) -> IlikeExpr<'a, E, P>
where
    E: Expr<'a, PostgresValue<'a>>,
    P: ComparisonOperand<'a, PostgresValue<'a>, E>,
    E::SQLType: Compatible<P::SQLType> + Textual,
    P::SQLType: Textual,
{
    SQLExpr::new(
        expr.to_sql()
            .push(SQLChunk::Raw("ILIKE".into()))
            .append(pattern.into_comparison_sql()),
    )
}

/// Tests that a text expression does not match a `LIKE` pattern, ignoring case
/// (`NOT ILIKE`).
///
/// Operand rules and NULL handling are the same as [`ilike`].
///
/// # Examples
///
/// ```
/// use drizzle_core::{ToSQL, expr::raw_non_null};
/// use drizzle_postgres::expr::not_ilike;
/// use drizzle_postgres::values::PostgresValue;
/// use drizzle_types::postgres::types::Varchar;
///
/// let email = raw_non_null::<PostgresValue, Varchar>("email");
/// let cond = not_ilike(email, "%@example.com");
/// assert_eq!(cond.to_sql().sql(), "email NOT ILIKE $1");
/// ```
pub fn not_ilike<'a, E, P>(expr: E, pattern: P) -> IlikeExpr<'a, E, P>
where
    E: Expr<'a, PostgresValue<'a>>,
    P: ComparisonOperand<'a, PostgresValue<'a>, E>,
    E::SQLType: Compatible<P::SQLType> + Textual,
    P::SQLType: Textual,
{
    SQLExpr::new(
        expr.to_sql()
            .push(Token::NOT)
            .push(SQLChunk::Raw("ILIKE".into()))
            .append(pattern.into_comparison_sql()),
    )
}

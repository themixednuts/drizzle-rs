//! `PostgreSQL` ILIKE operators.

use crate::values::PostgresValue;
use drizzle_core::expr::{AggOr, ComparisonOperand, Expr, NonNull, SQLExpr};
use drizzle_core::scope::Arg;
use drizzle_core::sql::{SQLChunk, Token};
use drizzle_types::postgres::types::Boolean;
use drizzle_types::{Compatible, Textual};

/// `ILIKE` result: NULL when either operand is, aggregate when either is.
type IlikeExpr<'a, E, P> = SQLExpr<
    'a,
    PostgresValue<'a>,
    Boolean,
    NonNull,
    <<E as Expr<'a, PostgresValue<'a>>>::Aggregate as AggOr<
        <P as ComparisonOperand<'a, PostgresValue<'a>, E>>::Aggregate,
    >>::Output,
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

/// Case-insensitive LIKE pattern matching (PostgreSQL-specific)
///
/// The result is a boolean expression, so it can be used directly in
/// `WHERE`, `HAVING` and join conditions.
///
/// # Example
///
/// ```
/// # use drizzle_postgres::expr::ilike;
/// # use drizzle_core::ToSQL;
/// # use drizzle_postgres::values::PostgresValue;
/// let name = drizzle_core::expr::raw_non_null::<PostgresValue, drizzle_types::postgres::types::Text>("name");
/// let cond = ilike(name, "%john%");
/// assert!(cond.to_sql().sql().contains("ILIKE"));
/// ```
pub fn ilike<'a, E, P>(expr: E, pattern: P) -> IlikeExpr<'a, E, P>
where
    E: Expr<'a, PostgresValue<'a>>,
    P: ComparisonOperand<'a, PostgresValue<'a>, E>,
    E::SQLType: Compatible<P::SQLType> + Textual,
    P::SQLType: Textual,
    E::Aggregate: AggOr<P::Aggregate>,
{
    SQLExpr::new(
        expr.to_sql()
            .push(SQLChunk::Raw("ILIKE".into()))
            .append(pattern.into_comparison_sql()),
    )
}

/// Case-insensitive NOT LIKE pattern matching (PostgreSQL-specific)
///
/// # Example
///
/// ```
/// # use drizzle_postgres::expr::not_ilike;
/// # use drizzle_core::ToSQL;
/// # use drizzle_postgres::values::PostgresValue;
/// let name = drizzle_core::expr::raw_non_null::<PostgresValue, drizzle_types::postgres::types::Text>("name");
/// let cond = not_ilike(name, "%admin%");
/// assert!(cond.to_sql().sql().contains("NOT ILIKE"));
/// ```
pub fn not_ilike<'a, E, P>(expr: E, pattern: P) -> IlikeExpr<'a, E, P>
where
    E: Expr<'a, PostgresValue<'a>>,
    P: ComparisonOperand<'a, PostgresValue<'a>, E>,
    E::SQLType: Compatible<P::SQLType> + Textual,
    P::SQLType: Textual,
    E::Aggregate: AggOr<P::Aggregate>,
{
    SQLExpr::new(
        expr.to_sql()
            .push(Token::NOT)
            .push(SQLChunk::Raw("ILIKE".into()))
            .append(pattern.into_comparison_sql()),
    )
}

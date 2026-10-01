//! NULL propagation and handling.
//!
//! This module provides traits and functions for handling SQL NULL values
//! in a type-safe manner.
//!
//! # Type Safety
//!
//! - `coalesce`, `ifnull`: Require compatible types between expression and default
//! - `nullif`: Requires compatible types between the two arguments

use crate::sql::{SQL, Token};
use crate::traits::SQLParam;
use crate::types::Compatible;
use crate::{MySQLDialect, PostgresDialect};

use super::{AggregateKind, Expr, ExprSources, Null, Nullability, SQLExpr};
use crate::scope::{Arg, Coalesce};

/// Sources of a NULL-absorbing pair (`COALESCE(a, b)`): NULL only when both are.
type FallbackSources<'a, V, A, B> = Coalesce<
    Arg<<A as Expr<'a, V>>::Nullable, <A as ExprSources>::Sources>,
    Arg<<B as Expr<'a, V>>::Nullable, <B as ExprSources>::Sources>,
>;

// =============================================================================
// COALESCE Function
// =============================================================================

/// COALESCE - returns first non-null value.
///
/// Requires compatible types between the expression and default.
///
/// # Type Safety
///
/// ```rust
/// # let _ = r####"
/// // ✅ OK: Both are Text
/// coalesce(users.nickname, users.name);
///
/// // ✅ OK: Int with i32 literal
/// coalesce(users.age, 0);
///
/// // ❌ Compile error: Int not compatible with Text
/// coalesce(users.age, "unknown");
/// # "####;
/// ```
#[allow(clippy::type_complexity)]
pub fn coalesce<'a, V, E, D>(
    expr: E,
    default: D,
) -> SQLExpr<
    'a,
    V,
    E::SQLType,
    <E::Nullable as Nullability>::And<D::Nullable>,
    <E::Aggregate as AggregateKind>::Or<D::Aggregate>,
    FallbackSources<'a, V, E, D>,
>
where
    V: SQLParam + 'a,
    E: Expr<'a, V>,
    D: Expr<'a, V>,
    E::SQLType: Compatible<D::SQLType>,
    D::Nullable: Nullability,
    D::Aggregate: AggregateKind,
{
    SQLExpr::new(SQL::func(
        "COALESCE",
        expr.into_expr_sql()
            .push(Token::COMMA)
            .append(default.into_expr_sql()),
    ))
}

/// COALESCE with multiple values.
///
/// Returns the first non-null value from the provided expressions.
/// Takes an explicit first argument to guarantee at least one value at compile time.
///
/// # Example
///
/// ```rust
/// # let _ = r####"
/// use drizzle_core::expr::coalesce_many;
///
/// // COALESCE(users.nickname, users.username, 'Anonymous')
/// let name = coalesce_many(users.nickname, [users.username, "Anonymous"]);
/// # "####;
/// ```
#[allow(clippy::type_complexity)]
pub fn coalesce_many<'a, V, E, I>(
    first: E,
    rest: I,
) -> SQLExpr<
    'a,
    V,
    E::SQLType,
    <E::Nullable as Nullability>::And<<I::Item as Expr<'a, V>>::Nullable>,
    <E::Aggregate as AggregateKind>::Or<<I::Item as Expr<'a, V>>::Aggregate>,
    FallbackSources<'a, V, E, I::Item>,
>
where
    V: SQLParam + 'a,
    E: Expr<'a, V>,
    I: IntoIterator,
    I::Item: Expr<'a, V>,
    E::SQLType: Compatible<<I::Item as Expr<'a, V>>::SQLType>,
    <I::Item as Expr<'a, V>>::Nullable: Nullability,
    <I::Item as Expr<'a, V>>::Aggregate: AggregateKind,
{
    let mut sql = first.into_expr_sql();
    for value in rest {
        sql = sql.push(Token::COMMA).append(value.into_expr_sql());
    }
    SQLExpr::new(SQL::func("COALESCE", sql))
}

// =============================================================================
// NULLIF Function
// =============================================================================

/// NULLIF - returns NULL if arguments are equal, else first argument.
///
/// Requires compatible types between the two arguments.
/// The result is always nullable since it can return NULL.
///
/// # Example
///
/// ```rust
/// # let _ = r####"
/// use drizzle_core::expr::nullif;
///
/// // Returns NULL if status is 'unknown', otherwise returns status
/// let status = nullif(item.status, "unknown");
/// # "####;
/// ```
#[allow(clippy::type_complexity)]
pub fn nullif<'a, V, E1, E2>(
    expr1: E1,
    expr2: E2,
) -> SQLExpr<
    'a,
    V,
    E1::SQLType,
    Null,
    <E1::Aggregate as AggregateKind>::Or<E2::Aggregate>,
    (E1::Sources, E2::Sources),
>
where
    V: SQLParam + 'a,
    E1: Expr<'a, V>,
    E2: Expr<'a, V>,
    E1::SQLType: Compatible<E2::SQLType>,
    E2::Aggregate: AggregateKind,
{
    SQLExpr::new(SQL::func(
        "NULLIF",
        expr1
            .into_expr_sql()
            .push(Token::COMMA)
            .append(expr2.into_expr_sql()),
    ))
}

// =============================================================================
// IFNULL / NVL Function
// =============================================================================

/// IFNULL - SQLite/MySQL equivalent of COALESCE with two arguments.
///
/// Requires compatible types between the expression and default.
/// Returns the first argument if not NULL, otherwise returns the second.
#[allow(clippy::type_complexity)]
pub fn ifnull<'a, V, E, D>(
    expr: E,
    default: D,
) -> SQLExpr<
    'a,
    V,
    E::SQLType,
    <E::Nullable as Nullability>::And<D::Nullable>,
    <E::Aggregate as AggregateKind>::Or<D::Aggregate>,
    FallbackSources<'a, V, E, D>,
>
where
    V: SQLParam + 'a,
    E: Expr<'a, V>,
    D: Expr<'a, V>,
    E::SQLType: Compatible<D::SQLType>,
    D::Nullable: Nullability,
    D::Aggregate: AggregateKind,
{
    SQLExpr::new(SQL::func(
        "IFNULL",
        expr.into_expr_sql()
            .push(Token::COMMA)
            .append(default.into_expr_sql()),
    ))
}

// =============================================================================
// GREATEST / LEAST
// =============================================================================

/// Dialect-specific NULL propagation for `GREATEST` and `LEAST`.
///
/// PostgreSQL ignores NULL arguments, while MySQL returns NULL when either
/// argument is NULL. SQLite does not provide these functions.
#[diagnostic::on_unimplemented(
    message = "GREATEST/LEAST are not available for this dialect",
    label = "use a dialect-specific extrema expression"
)]
pub trait GreatestLeastPolicy<L: Nullability, R: Nullability> {
    type Nullable: Nullability;
    /// How the operands' sources (each an [`Arg`]) combine.
    type Sources<A, B>;
}

impl<L, R> GreatestLeastPolicy<L, R> for PostgresDialect
where
    L: Nullability,
    R: Nullability,
{
    type Nullable = <L as Nullability>::And<R>;
    type Sources<A, B> = Coalesce<A, B>;
}

impl<L, R> GreatestLeastPolicy<L, R> for MySQLDialect
where
    L: Nullability,
    R: Nullability,
{
    type Nullable = <L as Nullability>::Or<R>;
    type Sources<A, B> = (A, B);
}

/// GREATEST - returns the largest of the given values (`PostgreSQL` and `MySQL`).
///
/// Both arguments must have compatible types. `PostgreSQL` ignores NULL inputs,
/// so `GREATEST(1, NULL)` returns `1`. The result is only NULL when all
/// inputs are NULL. MySQL returns NULL if either input is NULL.
///
/// # Example
///
/// ```rust
/// # let _ = r####"
/// use drizzle_core::expr::greatest;
///
/// // Clamp to minimum of 0
/// let score = greatest(users.score, 0);
/// # "####;
/// ```
#[allow(clippy::type_complexity)]
pub fn greatest<'a, V, L, R>(
    left: L,
    right: R,
) -> SQLExpr<
    'a,
    V,
    L::SQLType,
    <V::DialectMarker as GreatestLeastPolicy<L::Nullable, R::Nullable>>::Nullable,
    <L::Aggregate as AggregateKind>::Or<R::Aggregate>,
    <V::DialectMarker as GreatestLeastPolicy<L::Nullable, R::Nullable>>::Sources<
        Arg<L::Nullable, L::Sources>,
        Arg<R::Nullable, R::Sources>,
    >,
>
where
    V: SQLParam + 'a,
    V::DialectMarker: GreatestLeastPolicy<L::Nullable, R::Nullable>,
    L: Expr<'a, V>,
    R: Expr<'a, V>,
    L::SQLType: Compatible<R::SQLType>,
    R::Nullable: Nullability,
    R::Aggregate: AggregateKind,
{
    SQLExpr::new(SQL::func(
        "GREATEST",
        left.into_expr_sql()
            .push(Token::COMMA)
            .append(right.into_expr_sql()),
    ))
}

/// LEAST - returns the smallest of the given values (`PostgreSQL` and `MySQL`).
///
/// Both arguments must have compatible types. `PostgreSQL` ignores NULL inputs,
/// so `LEAST(1, NULL)` returns `1`. The result is only NULL when all
/// inputs are NULL. MySQL returns NULL if either input is NULL.
///
/// # Example
///
/// ```rust
/// # let _ = r####"
/// use drizzle_core::expr::least;
///
/// // Cap at maximum of 100
/// let score = least(users.score, 100);
/// # "####;
/// ```
#[allow(clippy::type_complexity)]
pub fn least<'a, V, L, R>(
    left: L,
    right: R,
) -> SQLExpr<
    'a,
    V,
    L::SQLType,
    <V::DialectMarker as GreatestLeastPolicy<L::Nullable, R::Nullable>>::Nullable,
    <L::Aggregate as AggregateKind>::Or<R::Aggregate>,
    <V::DialectMarker as GreatestLeastPolicy<L::Nullable, R::Nullable>>::Sources<
        Arg<L::Nullable, L::Sources>,
        Arg<R::Nullable, R::Sources>,
    >,
>
where
    V: SQLParam + 'a,
    V::DialectMarker: GreatestLeastPolicy<L::Nullable, R::Nullable>,
    L: Expr<'a, V>,
    R: Expr<'a, V>,
    L::SQLType: Compatible<R::SQLType>,
    R::Nullable: Nullability,
    R::Aggregate: AggregateKind,
{
    SQLExpr::new(SQL::func(
        "LEAST",
        left.into_expr_sql()
            .push(Token::COMMA)
            .append(right.into_expr_sql()),
    ))
}

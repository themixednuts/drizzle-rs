//! `PostgreSQL` sequence functions.
//!
//! These functions interact with `PostgreSQL` sequences (serial/identity columns).

use crate::dialect::DialectTypes;
use crate::dialect::{DialectSupports, feature};
use crate::sql::SQL;
use crate::traits::SQLParam;
use crate::types::Textual;

use super::{AggregateKind, Expr, NonNull, Nullability, SQLExpr, Scalar};
use crate::scope::ScopeOnly;

use crate::PostgresDialect;

impl DialectSupports<feature::Sequence> for PostgresDialect {}

/// NEXTVAL - advances a sequence and returns its new value (`PostgreSQL`).
///
/// The argument is the sequence name as a text expression.
///
/// # Example
///
/// ```rust
/// # let _ = r####"
/// use drizzle_core::expr::nextval;
///
/// // SELECT NEXTVAL('users_id_seq')
/// let next_id = nextval("users_id_seq");
/// # "####;
/// ```
pub fn nextval<'a, V, E>(
    sequence: E,
) -> SQLExpr<
    'a,
    V,
    <V::DialectMarker as DialectTypes>::BigInt,
    NonNull,
    Scalar,
    ScopeOnly<E::Sources>,
>
where
    V: SQLParam + 'a,
    V::DialectMarker: DialectSupports<feature::Sequence>,
    E: Expr<'a, V>,
    E::SQLType: Textual,
{
    SQLExpr::new(SQL::func("NEXTVAL", sequence.into_sql()))
}

/// CURRVAL - returns the most recently obtained value from a sequence (`PostgreSQL`).
///
/// Must be called after `NEXTVAL` has been used on the sequence in the current session.
///
/// # Example
///
/// ```rust
/// # let _ = r####"
/// use drizzle_core::expr::currval;
///
/// // SELECT CURRVAL('users_id_seq')
/// let current_id = currval("users_id_seq");
/// # "####;
/// ```
pub fn currval<'a, V, E>(
    sequence: E,
) -> SQLExpr<
    'a,
    V,
    <V::DialectMarker as DialectTypes>::BigInt,
    NonNull,
    Scalar,
    ScopeOnly<E::Sources>,
>
where
    V: SQLParam + 'a,
    V::DialectMarker: DialectSupports<feature::Sequence>,
    E: Expr<'a, V>,
    E::SQLType: Textual,
{
    SQLExpr::new(SQL::func("CURRVAL", sequence.into_sql()))
}

/// SETVAL - sets a sequence's current value (`PostgreSQL`).
///
/// # Example
///
/// ```rust
/// # let _ = r####"
/// use drizzle_core::expr::setval;
///
/// // SELECT SETVAL('users_id_seq', 100)
/// let set_id = setval("users_id_seq", 100);
/// # "####;
/// ```
#[allow(clippy::type_complexity)]
pub fn setval<'a, V, E, N>(
    sequence: E,
    value: N,
) -> SQLExpr<
    'a,
    V,
    <V::DialectMarker as DialectTypes>::BigInt,
    <E::Nullable as Nullability>::Or<N::Nullable>,
    <E::Aggregate as AggregateKind>::Or<N::Aggregate>,
    (E::Sources, N::Sources),
>
where
    V: SQLParam + 'a,
    V::DialectMarker: DialectSupports<feature::Sequence>,
    E: Expr<'a, V>,
    E::SQLType: Textual,
    N: Expr<'a, V>,
    N::SQLType: crate::types::Integral,
    N::Nullable: Nullability,
    N::Aggregate: AggregateKind,
{
    SQLExpr::new(SQL::func(
        "SETVAL",
        sequence
            .into_sql()
            .push(crate::Token::COMMA)
            .append(value.into_sql()),
    ))
}

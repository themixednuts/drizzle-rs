//! PostgreSQL sequence functions: `NEXTVAL`, `CURRVAL` and `SETVAL`.
//!
//! Sequences back `serial` and identity columns. The functions take the
//! sequence name as text and do not compile for SQLite or MySQL.

use crate::dialect::DialectTypes;
use crate::dialect::{DialectSupports, feature};
use crate::sql::SQL;
use crate::traits::SQLParam;
use crate::types::Textual;

use super::{AggregateKind, Expr, NonNull, Nullability, SQLExpr, Scalar};
use crate::scope::ScopeOnly;

use crate::PostgresDialect;

impl DialectSupports<feature::Sequence> for PostgresDialect {}

/// Advances a sequence and returns the new value (`NEXTVAL`), on PostgreSQL.
///
/// The argument is the sequence name as text. The result is `int8` and never
/// NULL.
///
/// # Examples
///
/// ```rust
/// # use drizzle_core::dialect::{Dialect, DialectTypes, PostgresDialect as D};
/// # use drizzle_core::{ColumnRef, SQL, SQLParam, expr::*};
/// # #[derive(Clone, Debug)] struct Value(String);
/// # impl SQLParam for Value { const DIALECT: Dialect = Dialect::PostgreSQL; type DialectMarker = D; }
/// # impl<X: ToString> From<X> for Value { fn from(v: X) -> Self { Value(v.to_string()) } }
/// # impl From<Value> for std::borrow::Cow<'_, Value> { fn from(v: Value) -> Self { Self::Owned(v) } }
/// # type C<X, N = NonNull> = &'static SQLExpr<'static, Value, X, N>;
/// # fn col<X: drizzle_core::types::DataType, N: Nullability>(c: &'static str) -> C<X, N> { Box::leak(Box::new(SQLExpr::new(SQL::column(ColumnRef::sql("users", c))))) }
/// # type Int = <D as DialectTypes>::Int; type Text = <D as DialectTypes>::Text; type Real = <D as DialectTypes>::Double;
/// # struct Users { id: C<Int>, age: C<Int>, name: C<Text>, email: C<Text, Null>, score: C<Real, Null>, active: C<<D as DialectTypes>::Bool>, created_at: C<<D as DialectTypes>::Timestamp> }
/// # let users = Users { id: col("id"), age: col("age"), name: col("name"), email: col("email"), score: col("score"), active: col("active"), created_at: col("created_at") };
/// let next_id = nextval::<Value, _>("users_id_seq");
/// assert_eq!(next_id.sql(), "NEXTVAL ($1)");
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

/// The value most recently returned by `NEXTVAL` for a sequence in this
/// session (`CURRVAL`), on PostgreSQL.
///
/// PostgreSQL raises an error if `NEXTVAL` has not been called for the
/// sequence in the current session. The argument is the sequence name as
/// text. The result is `int8` and never NULL.
///
/// # Examples
///
/// ```rust
/// # use drizzle_core::dialect::{Dialect, DialectTypes, PostgresDialect as D};
/// # use drizzle_core::{ColumnRef, SQL, SQLParam, expr::*};
/// # #[derive(Clone, Debug)] struct Value(String);
/// # impl SQLParam for Value { const DIALECT: Dialect = Dialect::PostgreSQL; type DialectMarker = D; }
/// # impl<X: ToString> From<X> for Value { fn from(v: X) -> Self { Value(v.to_string()) } }
/// # impl From<Value> for std::borrow::Cow<'_, Value> { fn from(v: Value) -> Self { Self::Owned(v) } }
/// # type C<X, N = NonNull> = &'static SQLExpr<'static, Value, X, N>;
/// # fn col<X: drizzle_core::types::DataType, N: Nullability>(c: &'static str) -> C<X, N> { Box::leak(Box::new(SQLExpr::new(SQL::column(ColumnRef::sql("users", c))))) }
/// # type Int = <D as DialectTypes>::Int; type Text = <D as DialectTypes>::Text; type Real = <D as DialectTypes>::Double;
/// # struct Users { id: C<Int>, age: C<Int>, name: C<Text>, email: C<Text, Null>, score: C<Real, Null>, active: C<<D as DialectTypes>::Bool>, created_at: C<<D as DialectTypes>::Timestamp> }
/// # let users = Users { id: col("id"), age: col("age"), name: col("name"), email: col("email"), score: col("score"), active: col("active"), created_at: col("created_at") };
/// let current_id = currval::<Value, _>("users_id_seq");
/// assert_eq!(current_id.sql(), "CURRVAL ($1)");
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

/// Sets a sequence's current value (`SETVAL`), on PostgreSQL.
///
/// The next `NEXTVAL` returns `value + 1`. `sequence` must be text and `value`
/// an integer. The result is the value set, as `int8`, nullable if either
/// argument is.
///
/// # Examples
///
/// ```rust
/// # use drizzle_core::dialect::{Dialect, DialectTypes, PostgresDialect as D};
/// # use drizzle_core::{ColumnRef, SQL, SQLParam, expr::*};
/// # #[derive(Clone, Debug)] struct Value(String);
/// # impl SQLParam for Value { const DIALECT: Dialect = Dialect::PostgreSQL; type DialectMarker = D; }
/// # impl<X: ToString> From<X> for Value { fn from(v: X) -> Self { Value(v.to_string()) } }
/// # impl From<Value> for std::borrow::Cow<'_, Value> { fn from(v: Value) -> Self { Self::Owned(v) } }
/// # type C<X, N = NonNull> = &'static SQLExpr<'static, Value, X, N>;
/// # fn col<X: drizzle_core::types::DataType, N: Nullability>(c: &'static str) -> C<X, N> { Box::leak(Box::new(SQLExpr::new(SQL::column(ColumnRef::sql("users", c))))) }
/// # type Int = <D as DialectTypes>::Int; type Text = <D as DialectTypes>::Text; type Real = <D as DialectTypes>::Double;
/// # struct Users { id: C<Int>, age: C<Int>, name: C<Text>, email: C<Text, Null>, score: C<Real, Null>, active: C<<D as DialectTypes>::Bool>, created_at: C<<D as DialectTypes>::Timestamp> }
/// # let users = Users { id: col("id"), age: col("age"), name: col("name"), email: col("email"), score: col("score"), active: col("active"), created_at: col("created_at") };
/// let reset = setval::<Value, _, _>("users_id_seq", 100);
/// assert_eq!(reset.sql(), "SETVAL ($1, $2)");
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

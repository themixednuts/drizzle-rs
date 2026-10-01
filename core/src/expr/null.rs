//! NULL handling: [`coalesce`], [`ifnull`], [`nullif`], [`greatest`], [`least`].
//!
//! All of these need arguments with compatible SQL types. Their results track
//! nullability: [`coalesce`] is non-null as soon as one argument is non-null,
//! while [`nullif`] is always nullable.

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

/// The first non-NULL of two values (`COALESCE(expr, default)`).
///
/// Both arguments must have compatible SQL types; the result has `expr`'s
/// type. The result is non-null if either argument is non-null, so a nullable
/// column with a non-null default becomes non-null. It is an aggregate if
/// either argument is.
///
/// # Examples
///
/// ```rust
/// # use drizzle_core::dialect::{Dialect, DialectTypes, SQLiteDialect as D};
/// # use drizzle_core::{ColumnRef, SQL, SQLParam, expr::*};
/// # #[derive(Clone, Debug)] struct Value(String);
/// # impl SQLParam for Value { const DIALECT: Dialect = Dialect::SQLite; type DialectMarker = D; }
/// # impl<X: ToString> From<X> for Value { fn from(v: X) -> Self { Value(v.to_string()) } }
/// # impl From<Value> for std::borrow::Cow<'_, Value> { fn from(v: Value) -> Self { Self::Owned(v) } }
/// # type C<X, N = NonNull> = &'static SQLExpr<'static, Value, X, N>;
/// # fn col<X: drizzle_core::types::DataType, N: Nullability>(c: &'static str) -> C<X, N> { Box::leak(Box::new(SQLExpr::new(SQL::column(ColumnRef::sql("users", c))))) }
/// # type Int = <D as DialectTypes>::Int; type Text = <D as DialectTypes>::Text; type Real = <D as DialectTypes>::Double;
/// # struct Users { id: C<Int>, age: C<Int>, name: C<Text>, email: C<Text, Null>, score: C<Real, Null>, active: C<<D as DialectTypes>::Bool>, created_at: C<<D as DialectTypes>::Timestamp> }
/// # let users = Users { id: col("id"), age: col("age"), name: col("name"), email: col("email"), score: col("score"), active: col("active"), created_at: col("created_at") };
/// // `email` is nullable; the result is not.
/// let email = coalesce(users.email, "unknown");
/// assert_eq!(email.sql(), r#"COALESCE ("users"."email", ?)"#);
/// ```
///
/// # Type safety
///
/// The default must have a compatible type:
///
/// ```rust,compile_fail
/// # use drizzle_core::dialect::{Dialect, DialectTypes, SQLiteDialect as D};
/// # use drizzle_core::{ColumnRef, SQL, SQLParam, expr::*};
/// # #[derive(Clone, Debug)] struct Value(String);
/// # impl SQLParam for Value { const DIALECT: Dialect = Dialect::SQLite; type DialectMarker = D; }
/// # impl<X: ToString> From<X> for Value { fn from(v: X) -> Self { Value(v.to_string()) } }
/// # impl From<Value> for std::borrow::Cow<'_, Value> { fn from(v: Value) -> Self { Self::Owned(v) } }
/// # type C<X, N = NonNull> = &'static SQLExpr<'static, Value, X, N>;
/// # fn col<X: drizzle_core::types::DataType, N: Nullability>(c: &'static str) -> C<X, N> { Box::leak(Box::new(SQLExpr::new(SQL::column(ColumnRef::sql("users", c))))) }
/// # type Int = <D as DialectTypes>::Int; type Text = <D as DialectTypes>::Text; type Real = <D as DialectTypes>::Double;
/// # struct Users { id: C<Int>, age: C<Int>, name: C<Text>, email: C<Text, Null>, score: C<Real, Null>, active: C<<D as DialectTypes>::Bool>, created_at: C<<D as DialectTypes>::Timestamp> }
/// # let users = Users { id: col("id"), age: col("age"), name: col("name"), email: col("email"), score: col("score"), active: col("active"), created_at: col("created_at") };
/// let wrong = coalesce(users.score, "none");
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

/// The first non-NULL of several values (`COALESCE(first, rest...)`).
///
/// `first` is separate so the list is never empty. `rest` is any iterator;
/// its elements share one Rust type, and their SQL type must be compatible
/// with `first`'s. The result has `first`'s type and is non-null if `first`
/// or the `rest` elements are non-null.
///
/// # Examples
///
/// ```rust
/// # use drizzle_core::dialect::{Dialect, DialectTypes, SQLiteDialect as D};
/// # use drizzle_core::{ColumnRef, SQL, SQLParam, expr::*};
/// # #[derive(Clone, Debug)] struct Value(String);
/// # impl SQLParam for Value { const DIALECT: Dialect = Dialect::SQLite; type DialectMarker = D; }
/// # impl<X: ToString> From<X> for Value { fn from(v: X) -> Self { Value(v.to_string()) } }
/// # impl From<Value> for std::borrow::Cow<'_, Value> { fn from(v: Value) -> Self { Self::Owned(v) } }
/// # type C<X, N = NonNull> = &'static SQLExpr<'static, Value, X, N>;
/// # fn col<X: drizzle_core::types::DataType, N: Nullability>(c: &'static str) -> C<X, N> { Box::leak(Box::new(SQLExpr::new(SQL::column(ColumnRef::sql("users", c))))) }
/// # type Int = <D as DialectTypes>::Int; type Text = <D as DialectTypes>::Text; type Real = <D as DialectTypes>::Double;
/// # struct Users { id: C<Int>, age: C<Int>, name: C<Text>, email: C<Text, Null>, score: C<Real, Null>, active: C<<D as DialectTypes>::Bool>, created_at: C<<D as DialectTypes>::Timestamp> }
/// # let users = Users { id: col("id"), age: col("age"), name: col("name"), email: col("email"), score: col("score"), active: col("active"), created_at: col("created_at") };
/// let email = coalesce_many(users.email, ["unknown", "n/a"]);
/// assert_eq!(email.sql(), r#"COALESCE ("users"."email", ?, ?)"#);
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

/// NULL when two values are equal, otherwise the first (`NULLIF(a, b)`).
///
/// Both arguments must have compatible SQL types. The result has the first
/// argument's type and is always nullable.
///
/// # Examples
///
/// ```rust
/// # use drizzle_core::dialect::{Dialect, DialectTypes, SQLiteDialect as D};
/// # use drizzle_core::{ColumnRef, SQL, SQLParam, expr::*};
/// # #[derive(Clone, Debug)] struct Value(String);
/// # impl SQLParam for Value { const DIALECT: Dialect = Dialect::SQLite; type DialectMarker = D; }
/// # impl<X: ToString> From<X> for Value { fn from(v: X) -> Self { Value(v.to_string()) } }
/// # impl From<Value> for std::borrow::Cow<'_, Value> { fn from(v: Value) -> Self { Self::Owned(v) } }
/// # type C<X, N = NonNull> = &'static SQLExpr<'static, Value, X, N>;
/// # fn col<X: drizzle_core::types::DataType, N: Nullability>(c: &'static str) -> C<X, N> { Box::leak(Box::new(SQLExpr::new(SQL::column(ColumnRef::sql("users", c))))) }
/// # type Int = <D as DialectTypes>::Int; type Text = <D as DialectTypes>::Text; type Real = <D as DialectTypes>::Double;
/// # struct Users { id: C<Int>, age: C<Int>, name: C<Text>, email: C<Text, Null>, score: C<Real, Null>, active: C<<D as DialectTypes>::Bool>, created_at: C<<D as DialectTypes>::Timestamp> }
/// # let users = Users { id: col("id"), age: col("age"), name: col("name"), email: col("email"), score: col("score"), active: col("active"), created_at: col("created_at") };
/// // Treat empty names as missing.
/// let name = nullif(users.name, "");
/// assert_eq!(name.sql(), r#"NULLIF ("users"."name", ?)"#);
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

/// The first value, or `default` when it is NULL (`IFNULL(expr, default)`).
///
/// Same typing as [`coalesce`]. `IFNULL` exists on SQLite and MySQL but not on
/// PostgreSQL; use [`coalesce`] for portable code.
///
/// # Examples
///
/// ```rust
/// # use drizzle_core::dialect::{Dialect, DialectTypes, SQLiteDialect as D};
/// # use drizzle_core::{ColumnRef, SQL, SQLParam, expr::*};
/// # #[derive(Clone, Debug)] struct Value(String);
/// # impl SQLParam for Value { const DIALECT: Dialect = Dialect::SQLite; type DialectMarker = D; }
/// # impl<X: ToString> From<X> for Value { fn from(v: X) -> Self { Value(v.to_string()) } }
/// # impl From<Value> for std::borrow::Cow<'_, Value> { fn from(v: Value) -> Self { Self::Owned(v) } }
/// # type C<X, N = NonNull> = &'static SQLExpr<'static, Value, X, N>;
/// # fn col<X: drizzle_core::types::DataType, N: Nullability>(c: &'static str) -> C<X, N> { Box::leak(Box::new(SQLExpr::new(SQL::column(ColumnRef::sql("users", c))))) }
/// # type Int = <D as DialectTypes>::Int; type Text = <D as DialectTypes>::Text; type Real = <D as DialectTypes>::Double;
/// # struct Users { id: C<Int>, age: C<Int>, name: C<Text>, email: C<Text, Null>, score: C<Real, Null>, active: C<<D as DialectTypes>::Bool>, created_at: C<<D as DialectTypes>::Timestamp> }
/// # let users = Users { id: col("id"), age: col("age"), name: col("name"), email: col("email"), score: col("score"), active: col("active"), created_at: col("created_at") };
/// let email = ifnull(users.email, "unknown");
/// assert_eq!(email.sql(), r#"IFNULL ("users"."email", ?)"#);
/// ```
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

/// How `GREATEST` and `LEAST` treat NULL on a dialect.
///
/// PostgreSQL ignores NULL arguments, while MySQL returns NULL when either
/// argument is NULL. SQLite has no such functions, so it does not implement
/// this trait.
#[diagnostic::on_unimplemented(
    message = "GREATEST/LEAST are not available for this dialect",
    label = "use a dialect-specific extrema expression"
)]
pub trait GreatestLeastPolicy<L: Nullability, R: Nullability> {
    /// Nullability of the result.
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

/// The larger of two values (`GREATEST(left, right)`), on PostgreSQL and MySQL.
///
/// Both arguments must have compatible SQL types; the result has `left`'s
/// type. On PostgreSQL, NULL arguments are ignored, so the result is NULL only
/// when both are. On MySQL, the result is NULL when either argument is. SQLite
/// has no `GREATEST`, so this does not compile for SQLite.
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
/// let n = greatest(users.age, 18);
/// assert_eq!(n.sql(), r#"GREATEST ("users"."age", $1)"#);
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

/// The smaller of two values (`LEAST(left, right)`), on PostgreSQL and MySQL.
///
/// Both arguments must have compatible SQL types; the result has `left`'s
/// type. On PostgreSQL, NULL arguments are ignored, so the result is NULL only
/// when both are. On MySQL, the result is NULL when either argument is. SQLite
/// has no `LEAST`, so this does not compile for SQLite.
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
/// let n = least(users.age, 18);
/// assert_eq!(n.sql(), r#"LEAST ("users"."age", $1)"#);
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

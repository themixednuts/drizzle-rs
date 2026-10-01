//! Logical operators: `AND`, `OR` and `NOT`.
//!
//! Use the functions [`and`], [`or`] and [`not`], or the Rust operators `&`,
//! `|` and `!` on [`SQLExpr`] values. For more than two conditions, see
//! [`all`](super::all), [`any`](super::any) and condition tuples.
//!
//! Operands must be boolean expressions. The result is nullable if any operand
//! is nullable, and is an aggregate if any operand is.
//!
//! # Examples
//!
//! ```rust
//! # use drizzle_core::dialect::{Dialect, DialectTypes, SQLiteDialect as D};
//! # use drizzle_core::{ColumnRef, SQL, SQLParam, expr::*};
//! # #[derive(Clone, Debug)] struct Value(String);
//! # impl SQLParam for Value { const DIALECT: Dialect = Dialect::SQLite; type DialectMarker = D; }
//! # impl<X: ToString> From<X> for Value { fn from(v: X) -> Self { Value(v.to_string()) } }
//! # impl From<Value> for std::borrow::Cow<'_, Value> { fn from(v: Value) -> Self { Self::Owned(v) } }
//! # type C<X, N = NonNull> = &'static SQLExpr<'static, Value, X, N>;
//! # fn col<X: drizzle_core::types::DataType, N: Nullability>(c: &'static str) -> C<X, N> { Box::leak(Box::new(SQLExpr::new(SQL::column(ColumnRef::sql("users", c))))) }
//! # type Int = <D as DialectTypes>::Int; type Text = <D as DialectTypes>::Text; type Real = <D as DialectTypes>::Double;
//! # struct Users { id: C<Int>, age: C<Int>, name: C<Text>, email: C<Text, Null>, score: C<Real, Null>, active: C<<D as DialectTypes>::Bool>, created_at: C<<D as DialectTypes>::Timestamp> }
//! # let users = Users { id: col("id"), age: col("age"), name: col("name"), email: col("email"), score: col("score"), active: col("active"), created_at: col("created_at") };
//! let a = and(gt(users.age, 18), is_not_null(users.email));
//! let b = gt(users.age, 18) & is_not_null(users.email);
//! assert_eq!(a.sql(), b.sql());
//! assert_eq!(a.sql(), r#"("users"."age" > ? AND "users"."email" IS NOT NULL)"#);
//!
//! let c = !eq(users.name, "admin") | lt(users.age, 13);
//! assert_eq!(c.sql(), r#"(NOT ("users"."name" = ?) OR "users"."age" < ?)"#);
//! ```

use core::ops::{BitAnd, BitOr, Not};

use crate::dialect::DialectTypes;
use crate::sql::{SQL, SQLChunk, Token};
use crate::traits::SQLParam;
use crate::types::BooleanLike;

use super::{AggregateKind, Expr, Nullability, SQLExpr};

#[inline]
fn operand_sql<'a, V, E>(value: E) -> SQL<'a, V>
where
    V: SQLParam + 'a,
    E: Expr<'a, V>,
    E::SQLType: BooleanLike,
{
    value.into_expr_sql()
}

#[inline]
fn binary_logical_op<'a, V, L, R>(left: L, token: Token, right: R) -> SQL<'a, V>
where
    V: SQLParam + 'a,
    L: Expr<'a, V>,
    L::SQLType: BooleanLike,
    R: Expr<'a, V>,
    R::SQLType: BooleanLike,
{
    SQL::from(Token::LPAREN)
        .append(operand_sql(left))
        .push(token)
        .append(operand_sql(right))
        .push(Token::RPAREN)
}

// =============================================================================
// NOT
// =============================================================================

/// Logical negation (`NOT`).
///
/// Renders `NOT (expr)`. The operand must be boolean. The result keeps the operand's
/// nullability and aggregate kind. `!expr` on an [`SQLExpr`] does the same.
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
/// assert_eq!(not(users.active).sql(), r#"NOT ("users"."active")"#);
/// assert_eq!(not(gt(users.age, 18)).sql(), r#"NOT ("users"."age" > ?)"#);
/// ```
///
/// # Type safety
///
/// Negating a text column does not compile:
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
/// let wrong = not(users.name);
/// ```
#[allow(clippy::type_complexity)]
pub fn not<'a, V, E>(
    expr: E,
) -> SQLExpr<'a, V, <V::DialectMarker as DialectTypes>::Bool, E::Nullable, E::Aggregate, E::Sources>
where
    V: SQLParam + 'a,
    E: Expr<'a, V>,
    E::SQLType: BooleanLike,
    E::Nullable: Nullability,
{
    let expr_sql: SQL<'a, V> = expr.into_expr_sql();
    let needs_paren = expr_sql.chunks.len() > 1
        || (expr_sql.chunks.len() == 1
            && !matches!(
                expr_sql.chunks[0],
                SQLChunk::Raw(_) | SQLChunk::Ident(_) | SQLChunk::Number(_)
            ));

    let sql = if needs_paren {
        SQL::from_iter([Token::NOT, Token::LPAREN])
            .append(expr_sql)
            .push(Token::RPAREN)
    } else {
        SQL::from(Token::NOT).append(expr_sql)
    };
    SQLExpr::new(sql)
}

// =============================================================================
// AND
// =============================================================================

/// Logical AND of two conditions.
///
/// Renders `(left AND right)`. Both operands must be boolean. The result is
/// nullable if either operand is, and is an aggregate if either operand is.
/// `left & right` on an [`SQLExpr`] does the same. For more than two
/// conditions, use a tuple or [`all`](super::all).
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
/// let cond = and(users.active, gt(users.age, 18));
/// assert_eq!(cond.sql(), r#"("users"."active" AND "users"."age" > ?)"#);
/// ```
#[allow(clippy::type_complexity)]
pub fn and<'a, V, L, R>(
    left: L,
    right: R,
) -> SQLExpr<
    'a,
    V,
    <V::DialectMarker as DialectTypes>::Bool,
    <L::Nullable as Nullability>::Or<R::Nullable>,
    <L::Aggregate as AggregateKind>::Or<R::Aggregate>,
    (L::Sources, R::Sources),
>
where
    V: SQLParam + 'a,
    L: Expr<'a, V>,
    L::SQLType: BooleanLike,
    R: Expr<'a, V>,
    R::SQLType: BooleanLike,
    R::Nullable: Nullability,
{
    SQLExpr::new(binary_logical_op(left, Token::AND, right))
}

// =============================================================================
// OR
// =============================================================================

/// Logical OR of two conditions.
///
/// Renders `(left OR right)`. Both operands must be boolean. The result is
/// nullable if either operand is, and is an aggregate if either operand is.
/// `left | right` on an [`SQLExpr`] does the same. For more than two
/// conditions, use [`any`](super::any).
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
/// let cond = or(eq(users.name, "admin"), eq(users.name, "root"));
/// assert_eq!(cond.sql(), r#"("users"."name" = ? OR "users"."name" = ?)"#);
/// ```
#[allow(clippy::type_complexity)]
pub fn or<'a, V, L, R>(
    left: L,
    right: R,
) -> SQLExpr<
    'a,
    V,
    <V::DialectMarker as DialectTypes>::Bool,
    <L::Nullable as Nullability>::Or<R::Nullable>,
    <L::Aggregate as AggregateKind>::Or<R::Aggregate>,
    (L::Sources, R::Sources),
>
where
    V: SQLParam + 'a,
    L: Expr<'a, V>,
    L::SQLType: BooleanLike,
    R: Expr<'a, V>,
    R::SQLType: BooleanLike,
    R::Nullable: Nullability,
{
    SQLExpr::new(binary_logical_op(left, Token::OR, right))
}

// =============================================================================
// Operator Trait Implementations
// =============================================================================

/// `!expr` renders `NOT expr`; see [`not`].
impl<'a, V, T, N, A, S> Not for SQLExpr<'a, V, T, N, A, S>
where
    V: SQLParam + 'a,
    T: BooleanLike,
    N: Nullability,
    A: AggregateKind,
{
    type Output = SQLExpr<'a, V, <V::DialectMarker as DialectTypes>::Bool, N, A, S>;

    fn not(self) -> Self::Output {
        not(self)
    }
}

/// `left & right` renders `(left AND right)`; see [`and`].
impl<'a, V, T, N, A, S, Rhs> BitAnd<Rhs> for SQLExpr<'a, V, T, N, A, S>
where
    V: SQLParam + 'a,
    T: BooleanLike,
    N: Nullability,
    A: AggregateKind,
    Rhs: Expr<'a, V>,
    Rhs::SQLType: BooleanLike,
    Rhs::Nullable: Nullability,
{
    type Output = SQLExpr<
        'a,
        V,
        <V::DialectMarker as DialectTypes>::Bool,
        <N as Nullability>::Or<Rhs::Nullable>,
        <A as AggregateKind>::Or<Rhs::Aggregate>,
        (S, Rhs::Sources),
    >;

    fn bitand(self, rhs: Rhs) -> Self::Output {
        and(self, rhs)
    }
}

/// `left | right` renders `(left OR right)`; see [`or`].
impl<'a, V, T, N, A, S, Rhs> BitOr<Rhs> for SQLExpr<'a, V, T, N, A, S>
where
    V: SQLParam + 'a,
    T: BooleanLike,
    N: Nullability,
    A: AggregateKind,
    Rhs: Expr<'a, V>,
    Rhs::SQLType: BooleanLike,
    Rhs::Nullable: Nullability,
{
    type Output = SQLExpr<
        'a,
        V,
        <V::DialectMarker as DialectTypes>::Bool,
        <N as Nullability>::Or<Rhs::Nullable>,
        <A as AggregateKind>::Or<Rhs::Aggregate>,
        (S, Rhs::Sources),
    >;

    fn bitor(self, rhs: Rhs) -> Self::Output {
        or(self, rhs)
    }
}

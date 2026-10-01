//! Comparison operators: `=`, `<>`, `<`, `>`, `LIKE`, `BETWEEN`, `IS NULL`, ...
//!
//! Each operator is available as a function ([`eq`], [`gt`], ...) and as a
//! method through [`ExprExt`] (`users.age.gt(18)`).
//!
//! # Type safety
//!
//! - [`eq`], [`ne`], [`gt`], [`gte`], [`lt`], [`lte`], [`between`] and the
//!   `IS [NOT] DISTINCT FROM` functions need operands with compatible SQL
//!   types (integers with integers, text with text, ...).
//! - [`like`] and [`not_like`] need text on both sides.
//! - [`is_null`], [`is_not_null`], [`is_true`] and [`is_false`] accept any
//!   expression.
//!
//! Every comparison returns the dialect's boolean type, typed as non-null.

use crate::dialect::{Dialect, DialectTypes};
use crate::sql::{SQL, Token};
use crate::traits::SQLParam;
use crate::types::{Compatible, DataType, Textual};

use super::{AggregateKind, Expr, ExprSources, NonNull, Nullability, SQLExpr};
use crate::scope::{Arg, ScopeOnly};

/// Sources of a NULL-propagating comparison: NULL when either operand is.
type CmpSources<'a, V, L, R> = (
    Arg<<L as Expr<'a, V>>::Nullable, <L as ExprSources>::Sources>,
    Arg<<R as ComparisonOperand<'a, V, L>>::Nullable, <R as ComparisonOperand<'a, V, L>>::Sources>,
);

/// Sources of `BETWEEN`: NULL when any operand is.
type BetweenSources<'a, V, E, L, H> = (
    Arg<<E as Expr<'a, V>>::Nullable, <E as ExprSources>::Sources>,
    (
        Arg<
            <L as ComparisonOperand<'a, V, E>>::Nullable,
            <L as ComparisonOperand<'a, V, E>>::Sources,
        >,
        Arg<
            <H as ComparisonOperand<'a, V, E>>::Nullable,
            <H as ComparisonOperand<'a, V, E>>::Sources,
        >,
    ),
);

/// Sources of a NULL-safe comparison: never NULL.
type NullSafeCmpSources<'a, V, L, R> = ScopeOnly<(
    <L as ExprSources>::Sources,
    <R as ComparisonOperand<'a, V, L>>::Sources,
)>;

// =============================================================================
// Internal Helper
// =============================================================================

fn binary_op<'a, V, L, R>(left: L, operator: Token, right: R) -> SQL<'a, V>
where
    V: SQLParam + 'a,
    L: Expr<'a, V>,
    R: ComparisonOperand<'a, V, L>,
    L::SQLType: Compatible<<R as ComparisonOperand<'a, V, L>>::SQLType>,
{
    let left_sql = operand_sql(left);
    let right_sql = ComparisonOperand::into_comparison_sql(right);

    left_sql.push(operator).append(right_sql)
}

#[inline]
fn operand_sql<'a, V, T>(value: T) -> SQL<'a, V>
where
    V: SQLParam + 'a,
    T: Expr<'a, V>,
{
    value.into_expr_sql()
}

/// A value that can be the right-hand side of a comparison with `L`.
///
/// Every [`Expr`] whose SQL type is compatible with `L`'s is an operand. The
/// table macros also implement this trait for a column's custom Rust type
/// (such as an enum or JSON struct) against that one column, so
/// `eq(table.custom, value)` works without making the custom type an
/// expression everywhere.
pub trait ComparisonOperand<'a, V, L>: Sized
where
    V: SQLParam + 'a,
    L: Expr<'a, V>,
{
    type SQLType: DataType;
    type Nullable: Nullability;
    type Aggregate: AggregateKind;
    /// See [`ExprSources::Sources`].
    type Sources;

    /// Renders the operand.
    fn into_comparison_sql(self) -> SQL<'a, V>;
}

impl<'a, V, L, R> ComparisonOperand<'a, V, L> for R
where
    V: SQLParam + 'a,
    L: Expr<'a, V>,
    R: Expr<'a, V>,
    L::SQLType: Compatible<R::SQLType>,
{
    type SQLType = R::SQLType;
    type Nullable = R::Nullable;
    type Aggregate = R::Aggregate;
    type Sources = R::Sources;

    fn into_comparison_sql(self) -> SQL<'a, V> {
        self.into_expr_sql()
    }
}

// =============================================================================
// Equality Comparisons
// =============================================================================

/// Equality comparison (`=`).
///
/// Renders `left = right`. Both sides must have compatible SQL types. The
/// result is the dialect's boolean, typed as non-null, and is an aggregate if
/// either side is.
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
/// let by_id = eq(users.id, 42);
/// assert_eq!(by_id.sql(), r#""users"."id" = ?"#);
/// ```
///
/// # Type safety
///
/// Comparing an integer column with text does not compile:
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
/// let wrong = eq(users.id, "hello");
/// ```
#[allow(clippy::type_complexity)]
pub fn eq<'a, V, L, R>(
    left: L,
    right: R,
) -> SQLExpr<
    'a,
    V,
    <V::DialectMarker as DialectTypes>::Bool,
    NonNull,
    <L::Aggregate as AggregateKind>::Or<<R as ComparisonOperand<'a, V, L>>::Aggregate>,
    CmpSources<'a, V, L, R>,
>
where
    V: SQLParam + 'a,
    L: Expr<'a, V>,
    R: ComparisonOperand<'a, V, L>,
    L::SQLType: Compatible<<R as ComparisonOperand<'a, V, L>>::SQLType>,
{
    SQLExpr::new(binary_op(left, Token::EQ, right))
}

/// Inequality comparison (`<>`).
///
/// Renders `left <> right`. Both sides must have compatible SQL types. The
/// result is the dialect's boolean, typed as non-null, and is an aggregate if
/// either side is.
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
/// assert_eq!(ne(users.name, "admin").sql(), r#""users"."name" <> ?"#);
/// ```
#[allow(clippy::type_complexity)]
pub fn ne<'a, V, L, R>(
    left: L,
    right: R,
) -> SQLExpr<
    'a,
    V,
    <V::DialectMarker as DialectTypes>::Bool,
    NonNull,
    <L::Aggregate as AggregateKind>::Or<<R as ComparisonOperand<'a, V, L>>::Aggregate>,
    CmpSources<'a, V, L, R>,
>
where
    V: SQLParam + 'a,
    L: Expr<'a, V>,
    R: ComparisonOperand<'a, V, L>,
    L::SQLType: Compatible<<R as ComparisonOperand<'a, V, L>>::SQLType>,
{
    SQLExpr::new(binary_op(left, Token::NE, right))
}

/// Inequality comparison (`<>`); same as [`ne`].
#[allow(clippy::type_complexity)]
pub fn neq<'a, V, L, R>(
    left: L,
    right: R,
) -> SQLExpr<
    'a,
    V,
    <V::DialectMarker as DialectTypes>::Bool,
    NonNull,
    <L::Aggregate as AggregateKind>::Or<<R as ComparisonOperand<'a, V, L>>::Aggregate>,
    CmpSources<'a, V, L, R>,
>
where
    V: SQLParam + 'a,
    L: Expr<'a, V>,
    R: ComparisonOperand<'a, V, L>,
    L::SQLType: Compatible<<R as ComparisonOperand<'a, V, L>>::SQLType>,
{
    ne(left, right)
}

// =============================================================================
// Ordering Comparisons
// =============================================================================

/// Greater-than comparison (`>`).
///
/// Renders `left > right`. Both sides must have compatible SQL types. The
/// result is the dialect's boolean, typed as non-null, and is an aggregate if
/// either side is.
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
/// assert_eq!(gt(users.age, 18).sql(), r#""users"."age" > ?"#);
/// ```
#[allow(clippy::type_complexity)]
pub fn gt<'a, V, L, R>(
    left: L,
    right: R,
) -> SQLExpr<
    'a,
    V,
    <V::DialectMarker as DialectTypes>::Bool,
    NonNull,
    <L::Aggregate as AggregateKind>::Or<<R as ComparisonOperand<'a, V, L>>::Aggregate>,
    CmpSources<'a, V, L, R>,
>
where
    V: SQLParam + 'a,
    L: Expr<'a, V>,
    R: ComparisonOperand<'a, V, L>,
    L::SQLType: Compatible<<R as ComparisonOperand<'a, V, L>>::SQLType>,
{
    SQLExpr::new(binary_op(left, Token::GT, right))
}

/// Greater-than-or-equal comparison (`>=`).
///
/// Renders `left >= right`. Both sides must have compatible SQL types. The
/// result is the dialect's boolean, typed as non-null, and is an aggregate if
/// either side is.
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
/// assert_eq!(gte(users.age, 18).sql(), r#""users"."age" >= ?"#);
/// ```
#[allow(clippy::type_complexity)]
pub fn gte<'a, V, L, R>(
    left: L,
    right: R,
) -> SQLExpr<
    'a,
    V,
    <V::DialectMarker as DialectTypes>::Bool,
    NonNull,
    <L::Aggregate as AggregateKind>::Or<<R as ComparisonOperand<'a, V, L>>::Aggregate>,
    CmpSources<'a, V, L, R>,
>
where
    V: SQLParam + 'a,
    L: Expr<'a, V>,
    R: ComparisonOperand<'a, V, L>,
    L::SQLType: Compatible<<R as ComparisonOperand<'a, V, L>>::SQLType>,
{
    SQLExpr::new(binary_op(left, Token::GE, right))
}

/// Less-than comparison (`<`).
///
/// Renders `left < right`. Both sides must have compatible SQL types. The
/// result is the dialect's boolean, typed as non-null, and is an aggregate if
/// either side is.
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
/// assert_eq!(lt(users.age, 65).sql(), r#""users"."age" < ?"#);
/// ```
#[allow(clippy::type_complexity)]
pub fn lt<'a, V, L, R>(
    left: L,
    right: R,
) -> SQLExpr<
    'a,
    V,
    <V::DialectMarker as DialectTypes>::Bool,
    NonNull,
    <L::Aggregate as AggregateKind>::Or<<R as ComparisonOperand<'a, V, L>>::Aggregate>,
    CmpSources<'a, V, L, R>,
>
where
    V: SQLParam + 'a,
    L: Expr<'a, V>,
    R: ComparisonOperand<'a, V, L>,
    L::SQLType: Compatible<<R as ComparisonOperand<'a, V, L>>::SQLType>,
{
    SQLExpr::new(binary_op(left, Token::LT, right))
}

/// Less-than-or-equal comparison (`<=`).
///
/// Renders `left <= right`. Both sides must have compatible SQL types. The
/// result is the dialect's boolean, typed as non-null, and is an aggregate if
/// either side is.
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
/// assert_eq!(lte(users.age, 65).sql(), r#""users"."age" <= ?"#);
/// ```
#[allow(clippy::type_complexity)]
pub fn lte<'a, V, L, R>(
    left: L,
    right: R,
) -> SQLExpr<
    'a,
    V,
    <V::DialectMarker as DialectTypes>::Bool,
    NonNull,
    <L::Aggregate as AggregateKind>::Or<<R as ComparisonOperand<'a, V, L>>::Aggregate>,
    CmpSources<'a, V, L, R>,
>
where
    V: SQLParam + 'a,
    L: Expr<'a, V>,
    R: ComparisonOperand<'a, V, L>,
    L::SQLType: Compatible<<R as ComparisonOperand<'a, V, L>>::SQLType>,
{
    SQLExpr::new(binary_op(left, Token::LE, right))
}

// =============================================================================
// Pattern Matching
// =============================================================================

/// Pattern match (`LIKE`).
///
/// Renders `left LIKE pattern`. Both sides must be text. In the pattern, `%`
/// matches any run of characters and `_` matches one character. The result is
/// the dialect's boolean, typed as non-null. Case sensitivity follows the
/// database: SQLite ignores ASCII case by default, PostgreSQL does not.
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
/// let starts_with_a = like(users.name, "A%");
/// assert_eq!(starts_with_a.sql(), r#""users"."name" LIKE ?"#);
/// ```
///
/// # Type safety
///
/// `LIKE` on an integer column does not compile:
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
/// let wrong = like(users.id, "1%");
/// ```
#[allow(clippy::type_complexity)]
pub fn like<'a, V, L, R>(
    left: L,
    pattern: R,
) -> SQLExpr<
    'a,
    V,
    <V::DialectMarker as DialectTypes>::Bool,
    NonNull,
    <L::Aggregate as AggregateKind>::Or<<R as ComparisonOperand<'a, V, L>>::Aggregate>,
    CmpSources<'a, V, L, R>,
>
where
    V: SQLParam + 'a,
    L: Expr<'a, V>,
    R: ComparisonOperand<'a, V, L>,
    L::SQLType: Compatible<<R as ComparisonOperand<'a, V, L>>::SQLType>,
    L::SQLType: Textual,
    <R as ComparisonOperand<'a, V, L>>::SQLType: Textual,
{
    SQLExpr::new(
        operand_sql(left)
            .push(Token::LIKE)
            .append(ComparisonOperand::into_comparison_sql(pattern)),
    )
}

/// Negated pattern match (`NOT LIKE`).
///
/// Renders `left NOT LIKE pattern`. Both sides must be text. See [`like`].
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
/// assert_eq!(not_like(users.name, "%bot%").sql(), r#""users"."name" NOT LIKE ?"#);
/// ```
#[allow(clippy::type_complexity)]
pub fn not_like<'a, V, L, R>(
    left: L,
    pattern: R,
) -> SQLExpr<
    'a,
    V,
    <V::DialectMarker as DialectTypes>::Bool,
    NonNull,
    <L::Aggregate as AggregateKind>::Or<<R as ComparisonOperand<'a, V, L>>::Aggregate>,
    CmpSources<'a, V, L, R>,
>
where
    V: SQLParam + 'a,
    L: Expr<'a, V>,
    R: ComparisonOperand<'a, V, L>,
    L::SQLType: Compatible<<R as ComparisonOperand<'a, V, L>>::SQLType>,
    L::SQLType: Textual,
    <R as ComparisonOperand<'a, V, L>>::SQLType: Textual,
{
    SQLExpr::new(
        operand_sql(left)
            .push(Token::NOT)
            .push(Token::LIKE)
            .append(ComparisonOperand::into_comparison_sql(pattern)),
    )
}

// =============================================================================
// Range Comparisons
// =============================================================================

/// Range check (`BETWEEN`), inclusive on both ends.
///
/// Renders `(expr BETWEEN low AND high)`. Both bounds must have a SQL type
/// compatible with `expr`. The result is the dialect's boolean, typed as
/// non-null.
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
/// let working_age = between(users.age, 18, 65);
/// assert_eq!(working_age.sql(), r#"("users"."age" BETWEEN ? AND ?)"#);
/// ```
#[allow(clippy::type_complexity)]
pub fn between<'a, V, E, L, H>(
    expr: E,
    low: L,
    high: H,
) -> SQLExpr<
    'a,
    V,
    <V::DialectMarker as DialectTypes>::Bool,
    NonNull,
    <<E::Aggregate as AggregateKind>::Or<<L as ComparisonOperand<'a, V, E>>::Aggregate> as AggregateKind>::Or<<H as ComparisonOperand<'a, V, E>>::Aggregate,>,
    BetweenSources<'a, V, E, L, H>,
>
where
    V: SQLParam + 'a,
    E: Expr<'a, V>,
    L: ComparisonOperand<'a, V, E>,
    H: ComparisonOperand<'a, V, E>,
    E::SQLType: Compatible<<L as ComparisonOperand<'a, V, E>>::SQLType>,
    E::SQLType: Compatible<<H as ComparisonOperand<'a, V, E>>::SQLType>,
{
    SQLExpr::new(
        SQL::from(Token::LPAREN)
            .append(operand_sql(expr))
            .push(Token::BETWEEN)
            .append(ComparisonOperand::into_comparison_sql(low))
            .push(Token::AND)
            .append(ComparisonOperand::into_comparison_sql(high))
            .push(Token::RPAREN),
    )
}

/// Negated range check (`NOT BETWEEN`).
///
/// Renders `(expr NOT BETWEEN low AND high)`. See [`between`].
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
/// let minors = not_between(users.age, 18, 120);
/// assert_eq!(minors.sql(), r#"("users"."age" NOT BETWEEN ? AND ?)"#);
/// ```
#[allow(clippy::type_complexity)]
pub fn not_between<'a, V, E, L, H>(
    expr: E,
    low: L,
    high: H,
) -> SQLExpr<
    'a,
    V,
    <V::DialectMarker as DialectTypes>::Bool,
    NonNull,
    <<E::Aggregate as AggregateKind>::Or<<L as ComparisonOperand<'a, V, E>>::Aggregate> as AggregateKind>::Or<<H as ComparisonOperand<'a, V, E>>::Aggregate,>,
    BetweenSources<'a, V, E, L, H>,
>
where
    V: SQLParam + 'a,
    E: Expr<'a, V>,
    L: ComparisonOperand<'a, V, E>,
    H: ComparisonOperand<'a, V, E>,
    E::SQLType: Compatible<<L as ComparisonOperand<'a, V, E>>::SQLType>,
    E::SQLType: Compatible<<H as ComparisonOperand<'a, V, E>>::SQLType>,
{
    SQLExpr::new(
        SQL::from(Token::LPAREN)
            .append(operand_sql(expr))
            .push(Token::NOT)
            .push(Token::BETWEEN)
            .append(ComparisonOperand::into_comparison_sql(low))
            .push(Token::AND)
            .append(ComparisonOperand::into_comparison_sql(high))
            .push(Token::RPAREN),
    )
}

// =============================================================================
// NULL Checks
// =============================================================================

/// NULL check (`IS NULL`).
///
/// Renders `expr IS NULL`. Accepts any expression. The result is the
/// dialect's boolean and is never NULL.
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
/// assert_eq!(is_null(users.email).sql(), r#""users"."email" IS NULL"#);
/// ```
#[allow(clippy::type_complexity)]
pub fn is_null<'a, V, E>(
    expr: E,
) -> SQLExpr<
    'a,
    V,
    <V::DialectMarker as DialectTypes>::Bool,
    NonNull,
    E::Aggregate,
    ScopeOnly<E::Sources>,
>
where
    V: SQLParam + 'a,
    E: Expr<'a, V>,
{
    SQLExpr::new(operand_sql(expr).push(Token::IS).push(Token::NULL))
}

/// Non-NULL check (`IS NOT NULL`).
///
/// Renders `expr IS NOT NULL`. Accepts any expression. The result is the
/// dialect's boolean and is never NULL.
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
/// assert_eq!(is_not_null(users.email).sql(), r#""users"."email" IS NOT NULL"#);
/// ```
#[allow(clippy::type_complexity)]
pub fn is_not_null<'a, V, E>(
    expr: E,
) -> SQLExpr<
    'a,
    V,
    <V::DialectMarker as DialectTypes>::Bool,
    NonNull,
    E::Aggregate,
    ScopeOnly<E::Sources>,
>
where
    V: SQLParam + 'a,
    E: Expr<'a, V>,
{
    SQLExpr::new(
        operand_sql(expr)
            .push(Token::IS)
            .push(Token::NOT)
            .push(Token::NULL),
    )
}

// =============================================================================
// IS DISTINCT FROM
// =============================================================================

/// NULL-safe inequality (`IS DISTINCT FROM`).
///
/// Like `<>`, but NULL is compared as an ordinary value, so the result is
/// never NULL:
///
/// - `NULL IS DISTINCT FROM NULL` is false;
/// - `NULL IS DISTINCT FROM 5` is true.
///
/// SQLite and PostgreSQL render `left IS DISTINCT FROM right`; MySQL renders
/// `NOT (left <=> right)`. Both sides must have compatible SQL types.
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
/// let changed = is_distinct_from(users.email, "old@example.com");
/// assert_eq!(changed.sql(), r#""users"."email" IS DISTINCT FROM ?"#);
/// ```
#[allow(clippy::type_complexity)]
pub fn is_distinct_from<'a, V, L, R>(
    left: L,
    right: R,
) -> SQLExpr<
    'a,
    V,
    <V::DialectMarker as DialectTypes>::Bool,
    NonNull,
    <L::Aggregate as AggregateKind>::Or<<R as ComparisonOperand<'a, V, L>>::Aggregate>,
    NullSafeCmpSources<'a, V, L, R>,
>
where
    V: SQLParam + 'a,
    L: Expr<'a, V>,
    R: ComparisonOperand<'a, V, L>,
    L::SQLType: Compatible<<R as ComparisonOperand<'a, V, L>>::SQLType>,
{
    let left = operand_sql(left);
    let right = ComparisonOperand::into_comparison_sql(right);
    let sql = match V::DIALECT {
        Dialect::MySQL => SQL::from(Token::NOT)
            .push(Token::LPAREN)
            .append(left)
            .append(SQL::raw("<=>"))
            .append(right)
            .push(Token::RPAREN),
        Dialect::SQLite | Dialect::PostgreSQL => left
            .push(Token::IS)
            .push(Token::DISTINCT)
            .push(Token::FROM)
            .append(right),
    };
    SQLExpr::new(sql)
}

/// NULL-safe equality (`IS NOT DISTINCT FROM`).
///
/// Like `=`, but NULL is compared as an ordinary value, so the result is
/// never NULL:
///
/// - `NULL IS NOT DISTINCT FROM NULL` is true;
/// - `NULL IS NOT DISTINCT FROM 5` is false.
///
/// SQLite and PostgreSQL render `left IS NOT DISTINCT FROM right`; MySQL
/// renders `left <=> right`. Both sides must have compatible SQL types.
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
/// let same = is_not_distinct_from(users.email, "a@example.com");
/// assert_eq!(same.sql(), r#""users"."email" IS NOT DISTINCT FROM ?"#);
/// ```
#[allow(clippy::type_complexity)]
pub fn is_not_distinct_from<'a, V, L, R>(
    left: L,
    right: R,
) -> SQLExpr<
    'a,
    V,
    <V::DialectMarker as DialectTypes>::Bool,
    NonNull,
    <L::Aggregate as AggregateKind>::Or<<R as ComparisonOperand<'a, V, L>>::Aggregate>,
    NullSafeCmpSources<'a, V, L, R>,
>
where
    V: SQLParam + 'a,
    L: Expr<'a, V>,
    R: ComparisonOperand<'a, V, L>,
    L::SQLType: Compatible<<R as ComparisonOperand<'a, V, L>>::SQLType>,
{
    let left = operand_sql(left);
    let right = ComparisonOperand::into_comparison_sql(right);
    let sql = match V::DIALECT {
        Dialect::MySQL => left.append(SQL::raw("<=>")).append(right),
        Dialect::SQLite | Dialect::PostgreSQL => left
            .push(Token::IS)
            .push(Token::NOT)
            .push(Token::DISTINCT)
            .push(Token::FROM)
            .append(right),
    };
    SQLExpr::new(sql)
}

// =============================================================================
// Boolean Testing
// =============================================================================

/// Truth test (`IS TRUE`) that never returns NULL.
///
/// Renders `expr IS TRUE`. Unlike `= TRUE`, a NULL input gives false instead
/// of NULL.
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
/// assert_eq!(is_true(users.active).sql(), r#""users"."active" IS TRUE"#);
/// ```
#[allow(clippy::type_complexity)]
pub fn is_true<'a, V, E>(
    expr: E,
) -> SQLExpr<
    'a,
    V,
    <V::DialectMarker as DialectTypes>::Bool,
    NonNull,
    E::Aggregate,
    ScopeOnly<E::Sources>,
>
where
    V: SQLParam + 'a,
    E: Expr<'a, V>,
{
    SQLExpr::new(operand_sql(expr).push(Token::IS).append(SQL::raw("TRUE")))
}

/// Falsity test (`IS FALSE`) that never returns NULL.
///
/// Renders `expr IS FALSE`. Unlike `= FALSE`, a NULL input gives false
/// instead of NULL.
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
/// assert_eq!(is_false(users.active).sql(), r#""users"."active" IS FALSE"#);
/// ```
#[allow(clippy::type_complexity)]
pub fn is_false<'a, V, E>(
    expr: E,
) -> SQLExpr<
    'a,
    V,
    <V::DialectMarker as DialectTypes>::Bool,
    NonNull,
    E::Aggregate,
    ScopeOnly<E::Sources>,
>
where
    V: SQLParam + 'a,
    E: Expr<'a, V>,
{
    SQLExpr::new(operand_sql(expr).push(Token::IS).append(SQL::raw("FALSE")))
}

// =============================================================================
// Method-based Comparison API (Extension Trait)
// =============================================================================

/// Method syntax for comparisons: `users.age.gt(18)` instead of `gt(users.age, 18)`.
///
/// Implemented for every [`Expr`]. Each method calls the function of the same
/// name in this module (`ge` and `le` call [`gte`] and [`lte`]) and has the
/// same type checks.
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
/// let filter = users.age.ge(18) & users.email.is_not_null();
/// assert_eq!(
///     filter.sql(),
///     r#"("users"."age" >= ? AND "users"."email" IS NOT NULL)"#
/// );
/// ```
pub trait ExprExt<'a, V: SQLParam>: Expr<'a, V> + Sized {
    /// Equality comparison (`=`); see [`eq`].
    #[allow(clippy::type_complexity)]
    fn eq<R>(
        self,
        other: R,
    ) -> SQLExpr<
        'a,
        V,
        <V::DialectMarker as DialectTypes>::Bool,
        NonNull,
        <Self::Aggregate as AggregateKind>::Or<<R as ComparisonOperand<'a, V, Self>>::Aggregate>,
        CmpSources<'a, V, Self, R>,
    >
    where
        R: ComparisonOperand<'a, V, Self>,
        Self::SQLType: Compatible<<R as ComparisonOperand<'a, V, Self>>::SQLType>,
    {
        eq(self, other)
    }

    /// Inequality comparison (`<>`); see [`ne`].
    #[allow(clippy::type_complexity)]
    fn ne<R>(
        self,
        other: R,
    ) -> SQLExpr<
        'a,
        V,
        <V::DialectMarker as DialectTypes>::Bool,
        NonNull,
        <Self::Aggregate as AggregateKind>::Or<<R as ComparisonOperand<'a, V, Self>>::Aggregate>,
        CmpSources<'a, V, Self, R>,
    >
    where
        R: ComparisonOperand<'a, V, Self>,
        Self::SQLType: Compatible<<R as ComparisonOperand<'a, V, Self>>::SQLType>,
    {
        ne(self, other)
    }

    /// Greater-than comparison (`>`); see [`gt`].
    #[allow(clippy::type_complexity)]
    fn gt<R>(
        self,
        other: R,
    ) -> SQLExpr<
        'a,
        V,
        <V::DialectMarker as DialectTypes>::Bool,
        NonNull,
        <Self::Aggregate as AggregateKind>::Or<<R as ComparisonOperand<'a, V, Self>>::Aggregate>,
        CmpSources<'a, V, Self, R>,
    >
    where
        R: ComparisonOperand<'a, V, Self>,
        Self::SQLType: Compatible<<R as ComparisonOperand<'a, V, Self>>::SQLType>,
    {
        gt(self, other)
    }

    /// Greater-than-or-equal comparison (`>=`); see [`gte`].
    #[allow(clippy::type_complexity)]
    fn ge<R>(
        self,
        other: R,
    ) -> SQLExpr<
        'a,
        V,
        <V::DialectMarker as DialectTypes>::Bool,
        NonNull,
        <Self::Aggregate as AggregateKind>::Or<<R as ComparisonOperand<'a, V, Self>>::Aggregate>,
        CmpSources<'a, V, Self, R>,
    >
    where
        R: ComparisonOperand<'a, V, Self>,
        Self::SQLType: Compatible<<R as ComparisonOperand<'a, V, Self>>::SQLType>,
    {
        gte(self, other)
    }

    /// Less-than comparison (`<`); see [`lt`].
    #[allow(clippy::type_complexity)]
    fn lt<R>(
        self,
        other: R,
    ) -> SQLExpr<
        'a,
        V,
        <V::DialectMarker as DialectTypes>::Bool,
        NonNull,
        <Self::Aggregate as AggregateKind>::Or<<R as ComparisonOperand<'a, V, Self>>::Aggregate>,
        CmpSources<'a, V, Self, R>,
    >
    where
        R: ComparisonOperand<'a, V, Self>,
        Self::SQLType: Compatible<<R as ComparisonOperand<'a, V, Self>>::SQLType>,
    {
        lt(self, other)
    }

    /// Less-than-or-equal comparison (`<=`); see [`lte`].
    #[allow(clippy::type_complexity)]
    fn le<R>(
        self,
        other: R,
    ) -> SQLExpr<
        'a,
        V,
        <V::DialectMarker as DialectTypes>::Bool,
        NonNull,
        <Self::Aggregate as AggregateKind>::Or<<R as ComparisonOperand<'a, V, Self>>::Aggregate>,
        CmpSources<'a, V, Self, R>,
    >
    where
        R: ComparisonOperand<'a, V, Self>,
        Self::SQLType: Compatible<<R as ComparisonOperand<'a, V, Self>>::SQLType>,
    {
        lte(self, other)
    }

    /// Pattern match (`LIKE`); see [`like`].
    #[allow(clippy::type_complexity)]
    fn like<R>(
        self,
        pattern: R,
    ) -> SQLExpr<
        'a,
        V,
        <V::DialectMarker as DialectTypes>::Bool,
        NonNull,
        <Self::Aggregate as AggregateKind>::Or<<R as ComparisonOperand<'a, V, Self>>::Aggregate>,
        CmpSources<'a, V, Self, R>,
    >
    where
        R: ComparisonOperand<'a, V, Self>,
        Self::SQLType: Compatible<<R as ComparisonOperand<'a, V, Self>>::SQLType>,
        Self::SQLType: Textual,
        <R as ComparisonOperand<'a, V, Self>>::SQLType: Textual,
    {
        like(self, pattern)
    }

    /// Negated pattern match (`NOT LIKE`); see [`not_like`].
    #[allow(clippy::type_complexity)]
    fn not_like<R>(
        self,
        pattern: R,
    ) -> SQLExpr<
        'a,
        V,
        <V::DialectMarker as DialectTypes>::Bool,
        NonNull,
        <Self::Aggregate as AggregateKind>::Or<<R as ComparisonOperand<'a, V, Self>>::Aggregate>,
        CmpSources<'a, V, Self, R>,
    >
    where
        R: ComparisonOperand<'a, V, Self>,
        Self::SQLType: Compatible<<R as ComparisonOperand<'a, V, Self>>::SQLType>,
        Self::SQLType: Textual,
        <R as ComparisonOperand<'a, V, Self>>::SQLType: Textual,
    {
        not_like(self, pattern)
    }

    /// NULL check (`IS NULL`); see [`is_null`].
    #[allow(clippy::wrong_self_convention)]
    #[allow(clippy::type_complexity)]
    fn is_null(
        self,
    ) -> SQLExpr<
        'a,
        V,
        <V::DialectMarker as DialectTypes>::Bool,
        NonNull,
        Self::Aggregate,
        ScopeOnly<Self::Sources>,
    > {
        is_null(self)
    }

    /// Non-NULL check (`IS NOT NULL`); see [`is_not_null`].
    #[allow(clippy::wrong_self_convention)]
    #[allow(clippy::type_complexity)]
    fn is_not_null(
        self,
    ) -> SQLExpr<
        'a,
        V,
        <V::DialectMarker as DialectTypes>::Bool,
        NonNull,
        Self::Aggregate,
        ScopeOnly<Self::Sources>,
    > {
        is_not_null(self)
    }

    /// Inclusive range check (`BETWEEN`); see [`between`].
    #[allow(clippy::type_complexity)]
    fn between<L, H>(
        self,
        low: L,
        high: H,
    ) -> SQLExpr<
        'a,
        V,
        <V::DialectMarker as DialectTypes>::Bool,
        NonNull,
        <<Self::Aggregate as AggregateKind>::Or<<L as ComparisonOperand<'a, V, Self>>::Aggregate,> as AggregateKind>::Or<<H as ComparisonOperand<'a, V, Self>>::Aggregate,>,
        BetweenSources<'a, V, Self, L, H>,
    >
    where
        L: ComparisonOperand<'a, V, Self>,
        H: ComparisonOperand<'a, V, Self>,
        Self::SQLType: Compatible<<L as ComparisonOperand<'a, V, Self>>::SQLType>,
        Self::SQLType: Compatible<<H as ComparisonOperand<'a, V, Self>>::SQLType>,
{
        between(self, low, high)
    }

    /// Negated range check (`NOT BETWEEN`); see [`not_between`].
    #[allow(clippy::type_complexity)]
    fn not_between<L, H>(
        self,
        low: L,
        high: H,
    ) -> SQLExpr<
        'a,
        V,
        <V::DialectMarker as DialectTypes>::Bool,
        NonNull,
        <<Self::Aggregate as AggregateKind>::Or<<L as ComparisonOperand<'a, V, Self>>::Aggregate,> as AggregateKind>::Or<<H as ComparisonOperand<'a, V, Self>>::Aggregate,>,
        BetweenSources<'a, V, Self, L, H>,
    >
    where
        L: ComparisonOperand<'a, V, Self>,
        H: ComparisonOperand<'a, V, Self>,
        Self::SQLType: Compatible<<L as ComparisonOperand<'a, V, Self>>::SQLType>,
        Self::SQLType: Compatible<<H as ComparisonOperand<'a, V, Self>>::SQLType>,
{
        not_between(self, low, high)
    }

    /// Membership in a list of values (`IN (...)`); see [`in_array`](super::in_array).
    #[allow(clippy::type_complexity)]
    fn in_array<I, R>(
        self,
        values: I,
    ) -> SQLExpr<
        'a,
        V,
        <V::DialectMarker as DialectTypes>::Bool,
        NonNull,
        Self::Aggregate,
        (
            Arg<Self::Nullable, Self::Sources>,
            Arg<R::Nullable, R::Sources>,
        ),
    >
    where
        I: IntoIterator<Item = R>,
        R: Expr<'a, V>,
        Self::SQLType: Compatible<R::SQLType>,
    {
        crate::expr::in_array(self, values)
    }

    /// Non-membership in a list of values (`NOT IN (...)`); see [`not_in_array`](super::not_in_array).
    #[allow(clippy::type_complexity)]
    fn not_in_array<I, R>(
        self,
        values: I,
    ) -> SQLExpr<
        'a,
        V,
        <V::DialectMarker as DialectTypes>::Bool,
        NonNull,
        Self::Aggregate,
        (
            Arg<Self::Nullable, Self::Sources>,
            Arg<R::Nullable, R::Sources>,
        ),
    >
    where
        I: IntoIterator<Item = R>,
        R: Expr<'a, V>,
        Self::SQLType: Compatible<R::SQLType>,
    {
        crate::expr::not_in_array(self, values)
    }

    /// Membership in a subquery's rows (`IN (SELECT ...)`); see
    /// [`in_subquery`](super::in_subquery).
    #[allow(clippy::type_complexity)]
    fn in_subquery<S>(
        self,
        subquery: S,
    ) -> SQLExpr<
        'a,
        V,
        <V::DialectMarker as DialectTypes>::Bool,
        NonNull,
        Self::Aggregate,
        (
            Arg<Self::Nullable, Self::Sources>,
            Arg<S::Nullable, S::Sources>,
        ),
    >
    where
        S: Expr<'a, V>,
        Self::SQLType: Compatible<S::SQLType> + Compatible<Self::SQLType>,
    {
        crate::expr::in_subquery(self, subquery)
    }

    /// Non-membership in a subquery's rows (`NOT IN (SELECT ...)`); see
    /// [`not_in_subquery`](super::not_in_subquery).
    #[allow(clippy::type_complexity)]
    fn not_in_subquery<S>(
        self,
        subquery: S,
    ) -> SQLExpr<
        'a,
        V,
        <V::DialectMarker as DialectTypes>::Bool,
        NonNull,
        Self::Aggregate,
        (
            Arg<Self::Nullable, Self::Sources>,
            Arg<S::Nullable, S::Sources>,
        ),
    >
    where
        S: Expr<'a, V>,
        Self::SQLType: Compatible<S::SQLType> + Compatible<Self::SQLType>,
    {
        crate::expr::not_in_subquery(self, subquery)
    }

    /// NULL-safe inequality (`IS DISTINCT FROM`); see [`is_distinct_from`].
    #[allow(clippy::type_complexity, clippy::wrong_self_convention)]
    fn is_distinct_from<R>(
        self,
        other: R,
    ) -> SQLExpr<
        'a,
        V,
        <V::DialectMarker as DialectTypes>::Bool,
        NonNull,
        <Self::Aggregate as AggregateKind>::Or<<R as ComparisonOperand<'a, V, Self>>::Aggregate>,
        NullSafeCmpSources<'a, V, Self, R>,
    >
    where
        R: ComparisonOperand<'a, V, Self>,
        Self::SQLType: Compatible<<R as ComparisonOperand<'a, V, Self>>::SQLType>,
    {
        is_distinct_from(self, other)
    }

    /// NULL-safe equality (`IS NOT DISTINCT FROM`); see [`is_not_distinct_from`].
    #[allow(clippy::type_complexity, clippy::wrong_self_convention)]
    fn is_not_distinct_from<R>(
        self,
        other: R,
    ) -> SQLExpr<
        'a,
        V,
        <V::DialectMarker as DialectTypes>::Bool,
        NonNull,
        <Self::Aggregate as AggregateKind>::Or<<R as ComparisonOperand<'a, V, Self>>::Aggregate>,
        NullSafeCmpSources<'a, V, Self, R>,
    >
    where
        R: ComparisonOperand<'a, V, Self>,
        Self::SQLType: Compatible<<R as ComparisonOperand<'a, V, Self>>::SQLType>,
    {
        is_not_distinct_from(self, other)
    }

    /// Truth test (`IS TRUE`); see [`is_true`].
    #[allow(clippy::wrong_self_convention)]
    #[allow(clippy::type_complexity)]
    fn is_true(
        self,
    ) -> SQLExpr<
        'a,
        V,
        <V::DialectMarker as DialectTypes>::Bool,
        NonNull,
        Self::Aggregate,
        ScopeOnly<Self::Sources>,
    > {
        is_true(self)
    }

    /// Falsity test (`IS FALSE`); see [`is_false`].
    #[allow(clippy::wrong_self_convention)]
    #[allow(clippy::type_complexity)]
    fn is_false(
        self,
    ) -> SQLExpr<
        'a,
        V,
        <V::DialectMarker as DialectTypes>::Bool,
        NonNull,
        Self::Aggregate,
        ScopeOnly<Self::Sources>,
    > {
        is_false(self)
    }
}

impl<'a, V: SQLParam, E: Expr<'a, V>> ExprExt<'a, V> for E {}

//! Membership tests: `IN`, `NOT IN`, `EXISTS` and `NOT EXISTS`.
//!
//! Each returns the dialect's boolean. `IN` and `NOT IN` are NULL when the
//! left side or a compared value is NULL (tracked through their sources, as
//! for [comparisons](super::eq)); `EXISTS` is never NULL. Values compared with
//! `IN` must have a SQL type compatible with the left-hand side.

use crate::dialect::DialectTypes;
use crate::sql::{SQL, Token};
use crate::traits::{SQLParam, ToSQL};
use crate::types::{Compatible, DataType};

use super::{AggregateKind, ComparisonOperand, Expr, ExprSources, NonNull, SQLExpr, Scalar};
use crate::scope::{Arg, ScopeOnly};

/// Sources of `expr IN (values)`: NULL when the operand or a value is.
type InArraySources<'a, V, E, R> = (
    Arg<<E as Expr<'a, V>>::Nullable, <E as ExprSources>::Sources>,
    Arg<<R as ComparisonOperand<'a, V, E>>::Nullable, <R as ComparisonOperand<'a, V, E>>::Sources>,
);

/// Sources of `lhs IN (subquery)`: NULL when the operand or a subquery value is.
type InSubquerySources<'a, V, L, S, M> = (
    <L as InSubqueryLhs<'a, V, M>>::Sources,
    Arg<<S as Expr<'a, V>>::Nullable, <S as ExprSources>::Sources>,
);

#[inline]
fn operand_sql<'a, V, T>(value: T) -> SQL<'a, V>
where
    V: SQLParam + 'a,
    T: Expr<'a, V>,
{
    value.into_expr_sql()
}

// =============================================================================
// InSubqueryLhs — marker-parameterized trait for single exprs and tuples
// =============================================================================

/// Marker: the left side of `IN (subquery)` is one expression.
#[doc(hidden)]
pub enum Single {}

/// Marker: the left side of `IN (subquery)` is a tuple (a row value).
#[doc(hidden)]
pub enum Multi {}

/// Left side of [`in_subquery`] and [`not_in_subquery`].
///
/// Implemented for single expressions (`users.id`) and for tuples of
/// expressions (`(users.id, users.name)`), which render as a row value. The
/// marker `M` is inferred; callers never name it.
pub trait InSubqueryLhs<'a, V: SQLParam, M>: Sized {
    /// SQL type of the left side (a tuple of types for a row value).
    type SQLType: DataType;
    /// Aggregate kind of the left side.
    type Aggregate: AggregateKind;
    /// Operand sources, with each element's declared nullability.
    type Sources;
    /// Renders the left side.
    fn into_lhs_sql(self) -> SQL<'a, V>;
}

/// A single expression: `in_subquery(users.id, sub)`.
///
/// The self-compatibility bound is what keeps condition tuples out of this
/// impl. A tuple of conditions is an expression too, so without it a tuple of
/// boolean-typed columns would match both this impl and the row-value impl
/// below and the marker `M` could not be inferred. Every SQL type that names a
/// column is compatible with itself; `Conjunction`, the SQL type of a condition
/// list, deliberately is not.
impl<'a, V, E> InSubqueryLhs<'a, V, Single> for E
where
    V: SQLParam + 'a,
    E: Expr<'a, V>,
    E::SQLType: Compatible<E::SQLType>,
{
    type SQLType = E::SQLType;
    type Aggregate = E::Aggregate;
    type Sources = Arg<E::Nullable, E::Sources>;
    fn into_lhs_sql(self) -> SQL<'a, V> {
        self.into_expr_sql()
    }
}

// Tuples: `in_subquery((users.id, users.name), sub)`.
macro_rules! impl_in_subquery_lhs_tuple {
    ($($E:ident),+; $($idx:tt),+) => {
        impl<'a, V, $($E),+> InSubqueryLhs<'a, V, Multi> for ($($E,)+)
        where
            V: SQLParam + 'a,
            $($E: Expr<'a, V>,)+
        {
            type SQLType = ($($E::SQLType,)+);
            type Aggregate = Scalar;
            type Sources = impl_in_subquery_lhs_tuple!(@sources $($E),+);
            fn into_lhs_sql(self) -> SQL<'a, V> {
                ToSQL::into_sql(self).parens()
            }
        }
    };
    (@sources $E:ident) => {
        Arg<<$E as Expr<'a, V>>::Nullable, <$E as ExprSources>::Sources>
    };
    (@sources $E:ident, $($rest:ident),+) => {
        (
            Arg<<$E as Expr<'a, V>>::Nullable, <$E as ExprSources>::Sources>,
            impl_in_subquery_lhs_tuple!(@sources $($rest),+),
        )
    };
}

with_col_sizes_8!(impl_in_subquery_lhs_tuple);

#[cfg(any(
    feature = "col16",
    feature = "col32",
    feature = "col64",
    feature = "col128",
    feature = "col200"
))]
with_col_sizes_16!(impl_in_subquery_lhs_tuple);

#[cfg(any(
    feature = "col32",
    feature = "col64",
    feature = "col128",
    feature = "col200"
))]
with_col_sizes_32!(impl_in_subquery_lhs_tuple);

#[cfg(any(feature = "col64", feature = "col128", feature = "col200"))]
with_col_sizes_64!(impl_in_subquery_lhs_tuple);

#[cfg(any(feature = "col128", feature = "col200"))]
with_col_sizes_128!(impl_in_subquery_lhs_tuple);

#[cfg(feature = "col200")]
with_col_sizes_200!(impl_in_subquery_lhs_tuple);

// =============================================================================
// IN Array
// =============================================================================

/// Membership in a list of values (`expr IN (v1, v2, ...)`).
///
/// `values` is any iterator, such as an array or `Vec`; each value must have
/// a SQL type compatible with `expr`. An empty list renders `FALSE`, since
/// nothing is in an empty list. The result is the dialect's boolean, NULL
/// when `expr` or a value is.
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
/// let cond = in_array(users.name, ["alice", "bob"]);
/// assert_eq!(cond.sql(), r#""users"."name" IN (?, ?)"#);
///
/// let none = in_array(users.name, Vec::<&str>::new());
/// assert_eq!(none.sql(), "FALSE");
/// ```
///
/// # Type safety
///
/// Values of an incompatible type do not compile:
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
/// let wrong = in_array(users.id, ["a", "b"]);
/// ```
#[allow(clippy::type_complexity)]
pub fn in_array<'a, V, E, I, R>(
    expr: E,
    values: I,
) -> SQLExpr<
    'a,
    V,
    <V::DialectMarker as DialectTypes>::Bool,
    NonNull,
    E::Aggregate,
    InArraySources<'a, V, E, R>,
>
where
    V: SQLParam + 'a,
    E: Expr<'a, V>,
    I: IntoIterator<Item = R>,
    R: ComparisonOperand<'a, V, E>,
    E::SQLType: Compatible<<R as ComparisonOperand<'a, V, E>>::SQLType>,
{
    SQLExpr::new(in_array_impl(expr, values, false))
}

/// Non-membership in a list of values (`expr NOT IN (v1, v2, ...)`).
///
/// Same rules as [`in_array`]. An empty list renders `TRUE`.
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
/// let cond = not_in_array(users.age, [0, 1, 2]);
/// assert_eq!(cond.sql(), r#""users"."age" NOT IN (?, ?, ?)"#);
/// ```
#[allow(clippy::type_complexity)]
pub fn not_in_array<'a, V, E, I, R>(
    expr: E,
    values: I,
) -> SQLExpr<
    'a,
    V,
    <V::DialectMarker as DialectTypes>::Bool,
    NonNull,
    E::Aggregate,
    InArraySources<'a, V, E, R>,
>
where
    V: SQLParam + 'a,
    E: Expr<'a, V>,
    I: IntoIterator<Item = R>,
    R: ComparisonOperand<'a, V, E>,
    E::SQLType: Compatible<<R as ComparisonOperand<'a, V, E>>::SQLType>,
{
    SQLExpr::new(in_array_impl(expr, values, true))
}

fn in_array_impl<'a, V, E, I, R>(expr: E, values: I, negated: bool) -> SQL<'a, V>
where
    V: SQLParam + 'a,
    E: Expr<'a, V>,
    I: IntoIterator<Item = R>,
    R: ComparisonOperand<'a, V, E>,
    E::SQLType: Compatible<<R as ComparisonOperand<'a, V, E>>::SQLType>,
{
    let mut values_iter = values.into_iter();

    match values_iter.next() {
        None => SQL::raw(if negated { "TRUE" } else { "FALSE" }),
        Some(first_value) => {
            let mut result = operand_sql(expr);
            if negated {
                result = result.push(Token::NOT);
            }

            result = result
                .push(Token::IN)
                .push(Token::LPAREN)
                .append(ComparisonOperand::into_comparison_sql(first_value));

            for value in values_iter {
                result = result
                    .push(Token::COMMA)
                    .append(ComparisonOperand::into_comparison_sql(value));
            }
            result.push(Token::RPAREN)
        }
    }
}

/// Membership in a subquery's rows (`lhs IN (SELECT ...)`).
///
/// `lhs` is one expression or a tuple of expressions (a row value, such as
/// `(users.id, users.name)`). The subquery's SQL type must be compatible
/// with `lhs`. Pass a select query built with the dialect's query builder;
/// its single column (or tuple of columns) gives the subquery's type. The
/// result is the dialect's boolean, NULL when `lhs` or a subquery value is.
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
/// # let banned_ids: SQLExpr<'_, Value, Int> = SQLExpr::new(SQL::raw("SELECT user_id FROM bans"));
/// // `banned_ids` is a one-column subquery of integers.
/// let cond = in_subquery(users.id, banned_ids);
/// assert_eq!(cond.sql(), r#""users"."id" IN (SELECT user_id FROM bans)"#);
/// ```
#[allow(clippy::type_complexity)]
pub fn in_subquery<'a, V, L, S, M>(
    lhs: L,
    subquery: S,
) -> SQLExpr<
    'a,
    V,
    <V::DialectMarker as DialectTypes>::Bool,
    NonNull,
    L::Aggregate,
    InSubquerySources<'a, V, L, S, M>,
>
where
    V: SQLParam + 'a,
    L: InSubqueryLhs<'a, V, M>,
    S: Expr<'a, V>,
    L::SQLType: Compatible<S::SQLType>,
{
    SQLExpr::new(
        lhs.into_lhs_sql()
            .push(Token::IN)
            .append(subquery.into_sql().parens()),
    )
}

/// Non-membership in a subquery's rows (`lhs NOT IN (SELECT ...)`).
///
/// Same rules as [`in_subquery`]. Note that SQL's `NOT IN` gives no rows
/// when the subquery returns any NULL.
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
/// # let banned_ids: SQLExpr<'_, Value, Int> = SQLExpr::new(SQL::raw("SELECT user_id FROM bans"));
/// let cond = not_in_subquery(users.id, banned_ids);
/// assert_eq!(cond.sql(), r#""users"."id" NOT IN (SELECT user_id FROM bans)"#);
/// ```
#[allow(clippy::type_complexity)]
pub fn not_in_subquery<'a, V, L, S, M>(
    lhs: L,
    subquery: S,
) -> SQLExpr<
    'a,
    V,
    <V::DialectMarker as DialectTypes>::Bool,
    NonNull,
    L::Aggregate,
    InSubquerySources<'a, V, L, S, M>,
>
where
    V: SQLParam + 'a,
    L: InSubqueryLhs<'a, V, M>,
    S: Expr<'a, V>,
    L::SQLType: Compatible<S::SQLType>,
{
    SQLExpr::new(
        lhs.into_lhs_sql()
            .push(Token::NOT)
            .push(Token::IN)
            .append(subquery.into_sql().parens()),
    )
}

// =============================================================================
// EXISTS
// =============================================================================

/// A complete `SELECT` statement, accepted by [`exists`] and [`not_exists`].
///
/// The dialect select builders implement it once they hold a finished
/// `SELECT`. `INSERT`, `UPDATE` and `DELETE` builders never do, even with
/// `RETURNING`. Raw [`SQL`] is also accepted.
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not a SELECT query",
    label = "EXISTS takes a subquery built with `select(...).from(...)`"
)]
pub trait SelectQuery {}

impl<V: SQLParam> SelectQuery for SQL<'_, V> {}
impl<T: SelectQuery + ?Sized> SelectQuery for &T {}

/// Whether a subquery returns any row (`EXISTS (SELECT ...)`).
///
/// `subquery` must be a `SELECT` (see [`SelectQuery`]). The result is the
/// dialect's boolean, never NULL, and scalar.
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
/// let has_posts = exists::<Value, _>(SQL::raw("SELECT 1 FROM posts"));
/// assert_eq!(has_posts.sql(), "EXISTS (SELECT 1 FROM posts)");
/// ```
pub fn exists<'a, V, S>(
    subquery: S,
) -> SQLExpr<'a, V, <V::DialectMarker as DialectTypes>::Bool, NonNull, Scalar, ScopeOnly<S::Sources>>
where
    V: SQLParam + 'a,
    S: ToSQL<'a, V> + ExprSources + SelectQuery,
{
    SQLExpr::new(
        SQL::from_iter([Token::EXISTS, Token::LPAREN])
            .append(subquery.into_sql())
            .push(Token::RPAREN),
    )
}

/// Whether a subquery returns no rows (`NOT EXISTS (SELECT ...)`).
///
/// Same rules as [`exists`].
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
/// let no_posts = not_exists::<Value, _>(SQL::raw("SELECT 1 FROM posts"));
/// assert_eq!(no_posts.sql(), "NOT EXISTS (SELECT 1 FROM posts)");
/// ```
pub fn not_exists<'a, V, S>(
    subquery: S,
) -> SQLExpr<'a, V, <V::DialectMarker as DialectTypes>::Bool, NonNull, Scalar, ScopeOnly<S::Sources>>
where
    V: SQLParam + 'a,
    S: ToSQL<'a, V> + ExprSources + SelectQuery,
{
    SQLExpr::new(
        SQL::from_iter([Token::NOT, Token::EXISTS, Token::LPAREN])
            .append(subquery.into_sql())
            .push(Token::RPAREN),
    )
}

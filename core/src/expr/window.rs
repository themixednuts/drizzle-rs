//! Window functions and the `OVER (...)` clause.
//!
//! - [`window`] starts a window specification ([`WindowSpec`]) with
//!   `PARTITION BY`, `ORDER BY` and a frame.
//! - `.over(spec)` on an aggregate (such as [`sum`](super::sum) or
//!   [`count`](super::count)) makes it a window function. The result is
//!   scalar, so it can sit next to plain columns without `GROUP BY`.
//! - Pure window functions ([`row_number`], [`rank`], [`lag`], ...) return a
//!   [`WindowFnExpr`], which cannot be used until `.over(...)` is called.
//!
//! # Examples
//!
//! ```rust
//! # use drizzle_core::asc;
//! # use drizzle_core::desc;
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
//! let running_total = sum(users.score).over(window().order_by(asc(users.id)));
//! assert_eq!(
//!     running_total.sql(),
//!     r#"SUM ("users"."score") OVER (ORDER BY "users"."id" ASC)"#
//! );
//!
//! let position = row_number().over(window().partition_by([users.name]).order_by(desc(users.age)));
//! assert_eq!(
//!     position.sql(),
//!     r#"ROW_NUMBER() OVER (PARTITION BY "users"."name" ORDER BY "users"."age" DESC)"#
//! );
//! ```

use crate::dialect::{DialectSupports, feature};
use core::marker::PhantomData;

use crate::sql::{SQL, Token};
use crate::traits::{SQLParam, ToSQL};
use crate::types::{BooleanLike, Compatible, DataType};

use super::{Agg, Expr, ExprSources, NonNull, Null, Nullability, SQLExpr, Scalar};
use crate::dialect::DialectTypes;
use crate::scope::ScopeOnly;

impl DialectSupports<feature::AggregateFilter> for crate::SQLiteDialect {}
impl DialectSupports<feature::AggregateFilter> for crate::PostgresDialect {}

// =============================================================================
// Frame Bounds
// =============================================================================

/// One end of a window frame, for [`WindowSpec::rows_between`] and
/// [`WindowSpec::range_between`].
#[derive(Debug, Clone, Copy)]
pub enum FrameBound {
    /// `UNBOUNDED PRECEDING`: the first row of the partition.
    UnboundedPreceding,
    /// `n PRECEDING`: `n` rows (or, for `RANGE`, values) before the current row.
    Preceding(u64),
    /// `CURRENT ROW`.
    CurrentRow,
    /// `n FOLLOWING`: `n` rows (or, for `RANGE`, values) after the current row.
    Following(u64),
    /// `UNBOUNDED FOLLOWING`: the last row of the partition.
    UnboundedFollowing,
}

impl FrameBound {
    fn write_sql<'a, V: SQLParam>(&self) -> SQL<'a, V> {
        match self {
            Self::UnboundedPreceding => SQL::from(Token::UNBOUNDED).push(Token::PRECEDING),
            Self::Preceding(n) => {
                SQL::number(usize::try_from(*n).unwrap_or(usize::MAX)).push(Token::PRECEDING)
            }
            Self::CurrentRow => SQL::from(Token::CURRENT).push(Token::ROW),
            Self::Following(n) => {
                SQL::number(usize::try_from(*n).unwrap_or(usize::MAX)).push(Token::FOLLOWING)
            }
            Self::UnboundedFollowing => SQL::from(Token::UNBOUNDED).push(Token::FOLLOWING),
        }
    }
}

// =============================================================================
// WindowSpec
// =============================================================================

/// The contents of an `OVER (...)` clause; start one with [`window`].
///
/// `S` records the tables that `PARTITION BY` and `ORDER BY` read, for the
/// query's scope check.
///
/// # Examples
///
/// ```rust
/// # use drizzle_core::asc;
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
/// let spec = window()
///     .partition_by([users.name])
///     .order_by(asc(users.created_at))
///     .rows_between(FrameBound::UnboundedPreceding, FrameBound::CurrentRow);
/// let total = sum(users.score).over(spec);
/// assert_eq!(
///     total.sql(),
///     r#"SUM ("users"."score") OVER (PARTITION BY "users"."name" ORDER BY "users"."created_at" ASC ROWS BETWEEN UNBOUNDED PRECEDING AND CURRENT ROW)"#
/// );
/// ```
#[derive(Debug, Clone)]
pub struct WindowSpec<'a, V: SQLParam, S = ()> {
    partition: Option<SQL<'a, V>>,
    order: Option<SQL<'a, V>>,
    frame: Option<SQL<'a, V>>,
    /// Sources read by `PARTITION BY` / `ORDER BY` (never NULL-making).
    sources: PhantomData<fn() -> S>,
}

/// Starts an empty window specification (`OVER ()`).
///
/// An empty window covers the whole result set.
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
/// let share = count::<Value, _>(()).over(window());
/// assert_eq!(share.sql(), "COUNT(*) OVER ()");
/// ```
#[must_use]
pub const fn window<'a, V: SQLParam>() -> WindowSpec<'a, V> {
    WindowSpec {
        partition: None,
        order: None,
        frame: None,
        sources: PhantomData,
    }
}

impl<'a, V: SQLParam + 'a, S> WindowSpec<'a, V, S> {
    fn with_sources<S2>(self) -> WindowSpec<'a, V, S2> {
        WindowSpec {
            partition: self.partition,
            order: self.order,
            frame: self.frame,
            sources: PhantomData,
        }
    }

    /// Sets `PARTITION BY`; the window restarts for each distinct value.
    ///
    /// Takes an array or other iterator of expressions of one Rust type.
    #[must_use]
    #[allow(clippy::type_complexity)]
    pub fn partition_by<I>(
        mut self,
        exprs: I,
    ) -> WindowSpec<'a, V, (S, ScopeOnly<<I::Item as ExprSources>::Sources>)>
    where
        I: IntoIterator,
        I::Item: ToSQL<'a, V> + ExprSources,
    {
        self.partition = Some(
            SQL::from(Token::PARTITION)
                .push(Token::BY)
                .append(SQL::join(exprs, Token::COMMA)),
        );
        self.with_sources()
    }

    /// Sets `ORDER BY` inside the window.
    ///
    /// Takes one ordering term such as [`asc`](crate::asc)`(col)`, or a tuple or
    /// array of terms.
    #[must_use]
    pub fn order_by<T: ToSQL<'a, V> + ExprSources>(
        mut self,
        exprs: T,
    ) -> WindowSpec<'a, V, (S, ScopeOnly<T::Sources>)> {
        self.order = Some(
            SQL::from(Token::ORDER)
                .push(Token::BY)
                .append(exprs.into_sql()),
        );
        self.with_sources()
    }

    /// Sets a `ROWS BETWEEN start AND end` frame, counted in rows.
    #[must_use]
    pub fn rows_between(mut self, start: FrameBound, end: FrameBound) -> Self {
        self.frame = Some(
            SQL::from(Token::ROWS)
                .push(Token::BETWEEN)
                .append(start.write_sql())
                .push(Token::AND)
                .append(end.write_sql()),
        );
        self
    }

    /// Sets a `RANGE BETWEEN start AND end` frame, measured in `ORDER BY` values.
    #[must_use]
    pub fn range_between(mut self, start: FrameBound, end: FrameBound) -> Self {
        self.frame = Some(
            SQL::from(Token::RANGE)
                .push(Token::BETWEEN)
                .append(start.write_sql())
                .push(Token::AND)
                .append(end.write_sql()),
        );
        self
    }

    /// Renders the clause contents, without the surrounding `OVER (...)`.
    fn into_sql(self) -> SQL<'a, V> {
        let mut sql = SQL::empty();
        if let Some(p) = self.partition {
            sql.append_mut(p);
        }
        if let Some(o) = self.order {
            sql.append_mut(o);
        }
        if let Some(f) = self.frame {
            sql.append_mut(f);
        }
        sql
    }
}

// =============================================================================
// .over() on aggregate expressions — Agg → Scalar
// =============================================================================

impl<'a, V, T, N, S> SQLExpr<'a, V, T, N, Agg, S>
where
    V: SQLParam + 'a,
    T: DataType,
    N: Nullability,
{
    /// Turns this aggregate into a window function (`agg OVER (...)`).
    ///
    /// The result keeps the SQL type and nullability but is scalar, so it can
    /// be selected next to plain columns.
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
    /// let per_name = sum(users.score).over(window().partition_by([users.name]));
    /// assert_eq!(
    ///     per_name.sql(),
    ///     r#"SUM ("users"."score") OVER (PARTITION BY "users"."name")"#
    /// );
    /// ```
    pub fn over<W>(self, spec: WindowSpec<'a, V, W>) -> SQLExpr<'a, V, T, N, Scalar, (S, W)> {
        let sql = self
            .into_sql()
            .push(Token::OVER)
            .push(Token::LPAREN)
            .append(spec.into_sql())
            .push(Token::RPAREN);
        SQLExpr::new(sql)
    }

    /// Limits the rows an aggregate sees (`agg FILTER (WHERE condition)`), on
    /// SQLite and PostgreSQL.
    ///
    /// The condition must be boolean. The result is still an aggregate.
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
    /// let adults = count(users.id).filter(gt(users.age, 18));
    /// assert_eq!(
    ///     adults.sql(),
    ///     r#"COUNT ("users"."id") FILTER (WHERE "users"."age" > ?)"#
    /// );
    /// ```
    #[allow(clippy::type_complexity)]
    pub fn filter<C>(self, condition: C) -> SQLExpr<'a, V, T, N, Agg, (S, ScopeOnly<C::Sources>)>
    where
        C: Expr<'a, V>,
        C::SQLType: BooleanLike,
        V::DialectMarker: DialectSupports<feature::AggregateFilter>,
    {
        let sql = self
            .into_sql()
            .push(Token::FILTER)
            .push(Token::LPAREN)
            .push(Token::WHERE)
            .append(condition.into_expr_sql())
            .push(Token::RPAREN);
        SQLExpr::new(sql)
    }
}

// =============================================================================
// WindowFnExpr — pure window functions that require .over()
// =============================================================================

/// A window function that still needs its `OVER (...)` clause.
///
/// [`row_number`], [`rank`], [`lag`] and the other pure window functions
/// return this type. It implements neither [`Expr`] nor [`ToSQL`], so it
/// cannot be used in a query until [`over`](Self::over) is called.
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
/// // ROW_NUMBER() without OVER is not an expression.
/// let wrong = gt(row_number::<Value>(), 1);
/// ```
#[derive(Debug, Clone)]
pub struct WindowFnExpr<'a, V: SQLParam, T: DataType, N: Nullability, S = ()> {
    sql: SQL<'a, V>,
    _marker: super::TypeMarker<(T, N, S)>,
}

impl<'a, V, T, N, S> WindowFnExpr<'a, V, T, N, S>
where
    V: SQLParam + 'a,
    T: DataType,
    N: Nullability,
{
    const fn new(sql: SQL<'a, V>) -> Self {
        Self {
            sql,
            _marker: PhantomData,
        }
    }

    /// Adds the window clause, producing a scalar expression (`fn OVER (...)`).
    pub fn over<W>(self, spec: WindowSpec<'a, V, W>) -> SQLExpr<'a, V, T, N, Scalar, (S, W)> {
        let sql = self
            .sql
            .push(Token::OVER)
            .push(Token::LPAREN)
            .append(spec.into_sql())
            .push(Token::RPAREN);
        SQLExpr::new(sql)
    }
}

// =============================================================================
// Pure Window Functions
// =============================================================================

/// Number of the row within its partition, from 1 (`ROW_NUMBER()`).
///
/// The result is the dialect's big-integer type and never NULL.
///
/// # Examples
///
/// ```rust
/// # use drizzle_core::asc;
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
/// let n = row_number().over(window().order_by(asc(users.age)));
/// assert_eq!(n.sql(), r#"ROW_NUMBER() OVER (ORDER BY "users"."age" ASC)"#);
/// ```
#[must_use]
pub fn row_number<'a, V>()
-> WindowFnExpr<'a, V, <V::DialectMarker as DialectTypes>::BigInt, NonNull>
where
    V: SQLParam + 'a,
{
    WindowFnExpr::new(SQL::raw("ROW_NUMBER()"))
}

/// Rank of the row, with gaps after ties (`RANK()`).
///
/// Tied rows share a rank and the next rank skips ahead (1, 1, 3). The result
/// is the dialect's big-integer type and never NULL.
///
/// # Examples
///
/// ```rust
/// # use drizzle_core::asc;
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
/// let n = rank().over(window().order_by(asc(users.age)));
/// assert_eq!(n.sql(), r#"RANK() OVER (ORDER BY "users"."age" ASC)"#);
/// ```
#[must_use]
pub fn rank<'a, V>() -> WindowFnExpr<'a, V, <V::DialectMarker as DialectTypes>::BigInt, NonNull>
where
    V: SQLParam + 'a,
{
    WindowFnExpr::new(SQL::raw("RANK()"))
}

/// Rank of the row, without gaps after ties (`DENSE_RANK()`).
///
/// Tied rows share a rank and the next rank follows on (1, 1, 2). The result
/// is the dialect's big-integer type and never NULL.
///
/// # Examples
///
/// ```rust
/// # use drizzle_core::asc;
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
/// let n = dense_rank().over(window().order_by(asc(users.age)));
/// assert_eq!(n.sql(), r#"DENSE_RANK() OVER (ORDER BY "users"."age" ASC)"#);
/// ```
#[must_use]
pub fn dense_rank<'a, V>()
-> WindowFnExpr<'a, V, <V::DialectMarker as DialectTypes>::BigInt, NonNull>
where
    V: SQLParam + 'a,
{
    WindowFnExpr::new(SQL::raw("DENSE_RANK()"))
}

/// Splits the partition into `n` groups of nearly equal size (`NTILE(n)`).
///
/// Returns the group number, from 1. The result is the dialect's integer type
/// and never NULL.
///
/// # Examples
///
/// ```rust
/// # use drizzle_core::asc;
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
/// let n = ntile(4).over(window().order_by(asc(users.age)));
/// assert_eq!(n.sql(), r#"NTILE (4) OVER (ORDER BY "users"."age" ASC)"#);
/// ```
#[must_use]
pub fn ntile<'a, V>(
    n: usize,
) -> WindowFnExpr<'a, V, <V::DialectMarker as DialectTypes>::Int, NonNull>
where
    V: SQLParam + 'a,
{
    WindowFnExpr::new(SQL::func("NTILE", SQL::number(n)))
}

/// Relative rank: `(rank - 1) / (rows - 1)` (`PERCENT_RANK()`).
///
/// The result is the dialect's double type, between 0 and 1, and never NULL.
///
/// # Examples
///
/// ```rust
/// # use drizzle_core::asc;
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
/// let n = percent_rank().over(window().order_by(asc(users.age)));
/// assert_eq!(n.sql(), r#"PERCENT_RANK() OVER (ORDER BY "users"."age" ASC)"#);
/// ```
#[must_use]
pub fn percent_rank<'a, V>()
-> WindowFnExpr<'a, V, <V::DialectMarker as DialectTypes>::Double, NonNull>
where
    V: SQLParam + 'a,
{
    WindowFnExpr::new(SQL::raw("PERCENT_RANK()"))
}

/// Fraction of rows ordered at or before this row (`CUME_DIST()`).
///
/// The result is the dialect's double type, greater than 0 and at most 1, and
/// never NULL.
///
/// # Examples
///
/// ```rust
/// # use drizzle_core::asc;
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
/// let n = cume_dist().over(window().order_by(asc(users.age)));
/// assert_eq!(n.sql(), r#"CUME_DIST() OVER (ORDER BY "users"."age" ASC)"#);
/// ```
#[must_use]
pub fn cume_dist<'a, V>() -> WindowFnExpr<'a, V, <V::DialectMarker as DialectTypes>::Double, NonNull>
where
    V: SQLParam + 'a,
{
    WindowFnExpr::new(SQL::raw("CUME_DIST()"))
}

/// The value of `expr` in the previous row (`LAG(expr)`).
///
/// The result has `expr`'s SQL type and is always nullable: the first row has
/// no previous row.
///
/// # Examples
///
/// ```rust
/// # use drizzle_core::asc;
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
/// let n = lag(users.name).over(window().order_by(asc(users.age)));
/// assert_eq!(n.sql(), r#"LAG ("users"."name") OVER (ORDER BY "users"."age" ASC)"#);
/// ```
pub fn lag<'a, V, E>(expr: E) -> WindowFnExpr<'a, V, E::SQLType, Null, E::Sources>
where
    V: SQLParam + 'a,
    E: Expr<'a, V>,
{
    WindowFnExpr::new(SQL::func("LAG", expr.into_sql()))
}

/// The value of `expr` `offset` rows back, or `default` (`LAG(expr, offset, default)`).
///
/// `default` must have a type compatible with `expr`. The result has `expr`'s
/// SQL type and is nullable if `expr` or `default` is.
///
/// # Examples
///
/// ```rust
/// # use drizzle_core::asc;
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
/// let n = lag_with_default(users.name, 2, "none").over(window().order_by(asc(users.age)));
/// assert_eq!(n.sql(), r#"LAG ("users"."name", 2, ?) OVER (ORDER BY "users"."age" ASC)"#);
/// ```
#[allow(clippy::type_complexity)]
pub fn lag_with_default<'a, V, E, D>(
    expr: E,
    offset: usize,
    default: D,
) -> WindowFnExpr<
    'a,
    V,
    E::SQLType,
    <E::Nullable as Nullability>::Or<D::Nullable>,
    (E::Sources, D::Sources),
>
where
    V: SQLParam + 'a,
    E: Expr<'a, V>,
    D: Expr<'a, V>,
    E::SQLType: Compatible<D::SQLType>,
    D::Nullable: Nullability,
{
    let args = expr
        .into_sql()
        .push(Token::COMMA)
        .append(SQL::number(offset))
        .push(Token::COMMA)
        .append(default.into_sql());
    WindowFnExpr::new(SQL::func("LAG", args))
}

/// The value of `expr` in the next row (`LEAD(expr)`).
///
/// The result has `expr`'s SQL type and is always nullable: the last row has
/// no next row.
///
/// # Examples
///
/// ```rust
/// # use drizzle_core::asc;
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
/// let n = lead(users.name).over(window().order_by(asc(users.age)));
/// assert_eq!(n.sql(), r#"LEAD ("users"."name") OVER (ORDER BY "users"."age" ASC)"#);
/// ```
pub fn lead<'a, V, E>(expr: E) -> WindowFnExpr<'a, V, E::SQLType, Null, E::Sources>
where
    V: SQLParam + 'a,
    E: Expr<'a, V>,
{
    WindowFnExpr::new(SQL::func("LEAD", expr.into_sql()))
}

/// The value of `expr` `offset` rows ahead, or `default` (`LEAD(expr, offset, default)`).
///
/// `default` must have a type compatible with `expr`. The result has `expr`'s
/// SQL type and is nullable if `expr` or `default` is.
///
/// # Examples
///
/// ```rust
/// # use drizzle_core::asc;
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
/// let n = lead_with_default(users.name, 1, "none").over(window().order_by(asc(users.age)));
/// assert_eq!(n.sql(), r#"LEAD ("users"."name", 1, ?) OVER (ORDER BY "users"."age" ASC)"#);
/// ```
#[allow(clippy::type_complexity)]
pub fn lead_with_default<'a, V, E, D>(
    expr: E,
    offset: usize,
    default: D,
) -> WindowFnExpr<
    'a,
    V,
    E::SQLType,
    <E::Nullable as Nullability>::Or<D::Nullable>,
    (E::Sources, D::Sources),
>
where
    V: SQLParam + 'a,
    E: Expr<'a, V>,
    D: Expr<'a, V>,
    E::SQLType: Compatible<D::SQLType>,
    D::Nullable: Nullability,
{
    let args = expr
        .into_sql()
        .push(Token::COMMA)
        .append(SQL::number(offset))
        .push(Token::COMMA)
        .append(default.into_sql());
    WindowFnExpr::new(SQL::func("LEAD", args))
}

/// The value of `expr` in the first row of the frame (`FIRST_VALUE(expr)`).
///
/// The result has `expr`'s SQL type and is typed as nullable.
///
/// # Examples
///
/// ```rust
/// # use drizzle_core::asc;
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
/// let n = first_value(users.name).over(window().order_by(asc(users.age)));
/// assert_eq!(n.sql(), r#"FIRST_VALUE ("users"."name") OVER (ORDER BY "users"."age" ASC)"#);
/// ```
pub fn first_value<'a, V, E>(expr: E) -> WindowFnExpr<'a, V, E::SQLType, Null, E::Sources>
where
    V: SQLParam + 'a,
    E: Expr<'a, V>,
{
    WindowFnExpr::new(SQL::func("FIRST_VALUE", expr.into_sql()))
}

/// The value of `expr` in the last row of the frame (`LAST_VALUE(expr)`).
///
/// With an `ORDER BY` and the default frame, the frame ends at the current
/// row (and its ties); use [`WindowSpec::rows_between`] to look further.
/// The result has `expr`'s SQL type and is typed as nullable.
///
/// # Examples
///
/// ```rust
/// # use drizzle_core::asc;
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
/// let n = last_value(users.name).over(window().order_by(asc(users.age)));
/// assert_eq!(n.sql(), r#"LAST_VALUE ("users"."name") OVER (ORDER BY "users"."age" ASC)"#);
/// ```
pub fn last_value<'a, V, E>(expr: E) -> WindowFnExpr<'a, V, E::SQLType, Null, E::Sources>
where
    V: SQLParam + 'a,
    E: Expr<'a, V>,
{
    WindowFnExpr::new(SQL::func("LAST_VALUE", expr.into_sql()))
}

/// The value of `expr` in the `n`-th row of the frame, from 1 (`NTH_VALUE(expr, n)`).
///
/// The result has `expr`'s SQL type and is always nullable: the frame may
/// have fewer than `n` rows.
///
/// # Examples
///
/// ```rust
/// # use drizzle_core::asc;
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
/// let n = nth_value(users.name, 2).over(window().order_by(asc(users.age)));
/// assert_eq!(n.sql(), r#"NTH_VALUE ("users"."name", 2) OVER (ORDER BY "users"."age" ASC)"#);
/// ```
pub fn nth_value<'a, V, E>(expr: E, n: usize) -> WindowFnExpr<'a, V, E::SQLType, Null, E::Sources>
where
    V: SQLParam + 'a,
    E: Expr<'a, V>,
{
    let args = expr.into_sql().push(Token::COMMA).append(SQL::number(n));
    WindowFnExpr::new(SQL::func("NTH_VALUE", args))
}

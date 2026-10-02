//! Window functions and OVER clause support.
//!
//! Provides:
//! - `WindowSpec` builder for PARTITION BY, ORDER BY, and frame clauses
//! - `.over()` method on aggregate `SQLExpr` to convert Agg → Scalar
//! - Pure window functions: `row_number`, `rank`, `dense_rank`, `ntile`,
//!   `percent_rank`, `cume_dist`, `lag`, `lead`, `first_value`, `last_value`,
//!   `nth_value`
//!
//! # Example
//!
//! ```rust
//! # let _ = r####"
//! use drizzle_core::expr::*;
//!
//! // Aggregate as window function
//! count(()).over(window().partition_by([users.dept]))
//! // → SQLExpr<CountType, NonNull, Scalar>
//!
//! // Pure window function
//! row_number().over(window().order_by([asc(users.id)]))
//! // → SQLExpr<CountType, NonNull, Scalar>
//! # "####;
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

/// Specifies a bound for a window frame (ROWS/RANGE BETWEEN).
#[derive(Debug, Clone, Copy)]
pub enum FrameBound {
    /// UNBOUNDED PRECEDING
    UnboundedPreceding,
    /// N PRECEDING
    Preceding(u64),
    /// CURRENT ROW
    CurrentRow,
    /// N FOLLOWING
    Following(u64),
    /// UNBOUNDED FOLLOWING
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

/// Builder for a window specification (the content inside `OVER (...)`).
///
/// # Example
///
/// ```rust
/// # let _ = r####"
/// window()
///     .partition_by([users.dept])
///     .order_by([asc(users.salary)])
///     .rows_between(FrameBound::UnboundedPreceding, FrameBound::CurrentRow)
/// # "####;
/// ```
#[derive(Debug, Clone)]
pub struct WindowSpec<'a, V: SQLParam, S = ()> {
    partition: Option<SQL<'a, V>>,
    order: Option<SQL<'a, V>>,
    frame: Option<SQL<'a, V>>,
    /// Sources read by `PARTITION BY` / `ORDER BY` (never NULL-making).
    sources: PhantomData<fn() -> S>,
}

/// Create an empty window specification.
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

    /// Set the PARTITION BY clause.
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

    /// Set the ORDER BY clause.
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

    /// Set a ROWS frame specification.
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

    /// Set a RANGE frame specification.
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

    /// Build the window spec into SQL (contents inside the OVER parentheses).
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
    /// Apply a window specification to this aggregate expression.
    ///
    /// Converts the expression from `Agg` to `Scalar`, generating
    /// `<expr> OVER (...)`.
    ///
    /// # Example
    ///
    /// ```rust
    /// # let _ = r####"
    /// sum(orders.amount).over(
    ///     window()
    ///         .partition_by([orders.customer_id])
    ///         .order_by([asc(orders.date)])
    /// )
    /// # "####;
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

    /// Apply a FILTER clause to this aggregate (`PostgreSQL` extension).
    ///
    /// Generates `<agg> FILTER (WHERE <condition>)`.
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

/// A window function expression that is not yet valid SQL.
///
/// Pure window functions like `ROW_NUMBER`, RANK, LAG, etc. MUST have an
/// `.over()` call before they can be used in a query. This type enforces
/// that at compile time by not implementing `Expr` or `ToSQL`.
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

    /// Apply a window specification, producing a usable scalar expression.
    ///
    /// Generates `<fn> OVER (...)`.
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

/// `ROW_NUMBER()` — sequential row number within the partition.
///
/// Returns an integer, never NULL.
#[must_use]
pub fn row_number<'a, V>()
-> WindowFnExpr<'a, V, <V::DialectMarker as DialectTypes>::BigInt, NonNull>
where
    V: SQLParam + 'a,
{
    WindowFnExpr::new(SQL::raw("ROW_NUMBER()"))
}

/// `RANK()` — rank with gaps for ties.
///
/// Returns an integer, never NULL.
#[must_use]
pub fn rank<'a, V>() -> WindowFnExpr<'a, V, <V::DialectMarker as DialectTypes>::BigInt, NonNull>
where
    V: SQLParam + 'a,
{
    WindowFnExpr::new(SQL::raw("RANK()"))
}

/// `DENSE_RANK()` — rank without gaps.
///
/// Returns an integer, never NULL.
#[must_use]
pub fn dense_rank<'a, V>()
-> WindowFnExpr<'a, V, <V::DialectMarker as DialectTypes>::BigInt, NonNull>
where
    V: SQLParam + 'a,
{
    WindowFnExpr::new(SQL::raw("DENSE_RANK()"))
}

/// NTILE(n) — divide rows into n roughly equal groups.
///
/// Returns an integer, never NULL.
#[must_use]
pub fn ntile<'a, V>(
    n: usize,
) -> WindowFnExpr<'a, V, <V::DialectMarker as DialectTypes>::Int, NonNull>
where
    V: SQLParam + 'a,
{
    WindowFnExpr::new(SQL::func("NTILE", SQL::number(n)))
}

/// `PERCENT_RANK()` — relative rank of the current row: (rank - 1) / (total rows - 1).
///
/// Returns a float between 0.0 and 1.0, never NULL.
#[must_use]
pub fn percent_rank<'a, V>()
-> WindowFnExpr<'a, V, <V::DialectMarker as DialectTypes>::Double, NonNull>
where
    V: SQLParam + 'a,
{
    WindowFnExpr::new(SQL::raw("PERCENT_RANK()"))
}

/// `CUME_DIST()` — cumulative distribution: fraction of rows <= current row.
///
/// Returns a float between 0.0 and 1.0 (exclusive of 0), never NULL.
#[must_use]
pub fn cume_dist<'a, V>() -> WindowFnExpr<'a, V, <V::DialectMarker as DialectTypes>::Double, NonNull>
where
    V: SQLParam + 'a,
{
    WindowFnExpr::new(SQL::raw("CUME_DIST()"))
}

/// LAG(expr) — value of expr from the previous row.
///
/// Returns the same type as expr, always nullable (no previous row → NULL).
pub fn lag<'a, V, E>(expr: E) -> WindowFnExpr<'a, V, E::SQLType, Null, E::Sources>
where
    V: SQLParam + 'a,
    E: Expr<'a, V>,
{
    WindowFnExpr::new(SQL::func("LAG", expr.into_sql()))
}

/// LAG(expr, offset, default) — value of expr from N rows back with a default.
///
/// Nullability is the combination of the expression's and default's nullability.
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

/// LEAD(expr) — value of expr from the next row.
///
/// Returns the same type as expr, always nullable (no next row → NULL).
pub fn lead<'a, V, E>(expr: E) -> WindowFnExpr<'a, V, E::SQLType, Null, E::Sources>
where
    V: SQLParam + 'a,
    E: Expr<'a, V>,
{
    WindowFnExpr::new(SQL::func("LEAD", expr.into_sql()))
}

/// LEAD(expr, offset, default) — value of expr from N rows ahead with a default.
///
/// Nullability is the combination of the expression's and default's nullability.
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

/// `FIRST_VALUE(expr)` — value of expr from the first row of the frame.
///
/// Always nullable (frame may be empty for some edge cases).
pub fn first_value<'a, V, E>(expr: E) -> WindowFnExpr<'a, V, E::SQLType, Null, E::Sources>
where
    V: SQLParam + 'a,
    E: Expr<'a, V>,
{
    WindowFnExpr::new(SQL::func("FIRST_VALUE", expr.into_sql()))
}

/// `LAST_VALUE(expr)` — value of expr from the last row of the frame.
///
/// Always nullable (frame boundaries affect result).
pub fn last_value<'a, V, E>(expr: E) -> WindowFnExpr<'a, V, E::SQLType, Null, E::Sources>
where
    V: SQLParam + 'a,
    E: Expr<'a, V>,
{
    WindowFnExpr::new(SQL::func("LAST_VALUE", expr.into_sql()))
}

/// `NTH_VALUE(expr`, n) — value of expr from the nth row of the frame.
///
/// Always nullable (n may exceed frame size).
pub fn nth_value<'a, V, E>(expr: E, n: usize) -> WindowFnExpr<'a, V, E::SQLType, Null, E::Sources>
where
    V: SQLParam + 'a,
    E: Expr<'a, V>,
{
    let args = expr.into_sql().push(Token::COMMA).append(SQL::number(n));
    WindowFnExpr::new(SQL::func("NTH_VALUE", args))
}

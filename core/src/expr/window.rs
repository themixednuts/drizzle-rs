//! Window functions and OVER clause support.
//!
//! Provides:
//! - `WindowSpec` builder for PARTITION BY, ORDER BY, and frame clauses
//! - `.over()` method on aggregate `SQLExpr` to convert Agg → Scalar
//! - Pure window functions: `row_number`, `rank`, `dense_rank`, `ntile`,
//!   `lag`, `lead`, `first_value`, `last_value`, `nth_value`
//!
//! # Example
//!
//! ```ignore
//! use drizzle_core::expr::*;
//!
//! // Aggregate as window function
//! count_all().over(window().partition_by([users.dept]))
//! // → SQLExpr<CountType, NonNull, Scalar>
//!
//! // Pure window function
//! row_number().over(window().order_by([asc(users.id)]))
//! // → SQLExpr<CountType, NonNull, Scalar>
//! ```

use core::marker::PhantomData;

use crate::sql::{SQL, Token};
use crate::traits::{SQLParam, ToSQL};
use crate::types::{BooleanLike, Compatible, DataType};

use super::agg::CountPolicy;
use super::null::NullOr;
use super::{Agg, Expr, NonNull, Null, Nullability, SQLExpr, Scalar};

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
            FrameBound::UnboundedPreceding => SQL::from(Token::UNBOUNDED).push(Token::PRECEDING),
            FrameBound::Preceding(n) => SQL::number(*n as usize).push(Token::PRECEDING),
            FrameBound::CurrentRow => SQL::from(Token::CURRENT).push(Token::ROW),
            FrameBound::Following(n) => SQL::number(*n as usize).push(Token::FOLLOWING),
            FrameBound::UnboundedFollowing => SQL::from(Token::UNBOUNDED).push(Token::FOLLOWING),
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
/// ```ignore
/// window()
///     .partition_by([users.dept])
///     .order_by([asc(users.salary)])
///     .rows_between(FrameBound::UnboundedPreceding, FrameBound::CurrentRow)
/// ```
#[derive(Debug, Clone)]
pub struct WindowSpec<'a, V: SQLParam> {
    partition: Option<SQL<'a, V>>,
    order: Option<SQL<'a, V>>,
    frame: Option<SQL<'a, V>>,
}

/// Create an empty window specification.
pub fn window<'a, V: SQLParam>() -> WindowSpec<'a, V> {
    WindowSpec {
        partition: None,
        order: None,
        frame: None,
    }
}

impl<'a, V: SQLParam + 'a> WindowSpec<'a, V> {
    /// Set the PARTITION BY clause.
    pub fn partition_by<I>(mut self, exprs: I) -> Self
    where
        I: IntoIterator,
        I::Item: ToSQL<'a, V>,
    {
        self.partition = Some(
            SQL::from(Token::PARTITION)
                .push(Token::BY)
                .append(SQL::join(exprs, Token::COMMA)),
        );
        self
    }

    /// Set the ORDER BY clause.
    pub fn order_by<T: ToSQL<'a, V>>(mut self, exprs: T) -> Self {
        self.order = Some(
            SQL::from(Token::ORDER)
                .push(Token::BY)
                .append(exprs.into_sql()),
        );
        self
    }

    /// Set a ROWS frame specification.
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

impl<'a, V, T, N> SQLExpr<'a, V, T, N, Agg>
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
    /// ```ignore
    /// sum(orders.amount).over(
    ///     window()
    ///         .partition_by([orders.customer_id])
    ///         .order_by([asc(orders.date)])
    /// )
    /// ```
    pub fn over(self, spec: WindowSpec<'a, V>) -> SQLExpr<'a, V, T, N, Scalar> {
        let sql = self
            .into_sql()
            .push(Token::OVER)
            .push(Token::LPAREN)
            .append(spec.into_sql())
            .push(Token::RPAREN);
        SQLExpr::new(sql)
    }

    /// Apply a FILTER clause to this aggregate (PostgreSQL extension).
    ///
    /// Generates `<agg> FILTER (WHERE <condition>)`.
    pub fn filter<C>(self, condition: C) -> SQLExpr<'a, V, T, N, Agg>
    where
        C: Expr<'a, V>,
        C::SQLType: BooleanLike,
    {
        let sql = self
            .into_sql()
            .push(Token::FILTER)
            .push(Token::LPAREN)
            .push(Token::WHERE)
            .append(condition.into_sql())
            .push(Token::RPAREN);
        SQLExpr::new(sql)
    }
}

// =============================================================================
// WindowFnExpr — pure window functions that require .over()
// =============================================================================

/// A window function expression that is not yet valid SQL.
///
/// Pure window functions like ROW_NUMBER, RANK, LAG, etc. MUST have an
/// `.over()` call before they can be used in a query. This type enforces
/// that at compile time by not implementing `Expr` or `ToSQL`.
#[derive(Debug, Clone)]
pub struct WindowFnExpr<'a, V: SQLParam, T: DataType, N: Nullability> {
    sql: SQL<'a, V>,
    _marker: PhantomData<(T, N)>,
}

impl<'a, V, T, N> WindowFnExpr<'a, V, T, N>
where
    V: SQLParam + 'a,
    T: DataType,
    N: Nullability,
{
    fn new(sql: SQL<'a, V>) -> Self {
        Self {
            sql,
            _marker: PhantomData,
        }
    }

    /// Apply a window specification, producing a usable scalar expression.
    ///
    /// Generates `<fn> OVER (...)`.
    pub fn over(self, spec: WindowSpec<'a, V>) -> SQLExpr<'a, V, T, N, Scalar> {
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

/// ROW_NUMBER() — sequential row number within the partition.
///
/// Returns an integer, never NULL.
pub fn row_number<'a, V>() -> WindowFnExpr<'a, V, <V::DialectMarker as CountPolicy>::Count, NonNull>
where
    V: SQLParam + 'a,
    V::DialectMarker: CountPolicy,
{
    WindowFnExpr::new(SQL::raw("ROW_NUMBER()"))
}

/// RANK() — rank with gaps for ties.
///
/// Returns an integer, never NULL.
pub fn rank<'a, V>() -> WindowFnExpr<'a, V, <V::DialectMarker as CountPolicy>::Count, NonNull>
where
    V: SQLParam + 'a,
    V::DialectMarker: CountPolicy,
{
    WindowFnExpr::new(SQL::raw("RANK()"))
}

/// DENSE_RANK() — rank without gaps.
///
/// Returns an integer, never NULL.
pub fn dense_rank<'a, V>() -> WindowFnExpr<'a, V, <V::DialectMarker as CountPolicy>::Count, NonNull>
where
    V: SQLParam + 'a,
    V::DialectMarker: CountPolicy,
{
    WindowFnExpr::new(SQL::raw("DENSE_RANK()"))
}

/// NTILE(n) — divide rows into n roughly equal groups.
///
/// Returns an integer, never NULL.
pub fn ntile<'a, V>(
    n: usize,
) -> WindowFnExpr<'a, V, <V::DialectMarker as CountPolicy>::Count, NonNull>
where
    V: SQLParam + 'a,
    V::DialectMarker: CountPolicy,
{
    WindowFnExpr::new(SQL::func("NTILE", SQL::number(n)))
}

/// LAG(expr) — value of expr from the previous row.
///
/// Returns the same type as expr, always nullable (no previous row → NULL).
pub fn lag<'a, V, E>(expr: E) -> WindowFnExpr<'a, V, E::SQLType, Null>
where
    V: SQLParam + 'a,
    E: Expr<'a, V>,
{
    WindowFnExpr::new(SQL::func("LAG", expr.into_sql()))
}

/// LAG(expr, offset, default) — value of expr from N rows back with a default.
///
/// Nullability is the combination of the expression's and default's nullability.
pub fn lag_with_default<'a, V, E, D>(
    expr: E,
    offset: usize,
    default: D,
) -> WindowFnExpr<'a, V, E::SQLType, <E::Nullable as NullOr<D::Nullable>>::Output>
where
    V: SQLParam + 'a,
    E: Expr<'a, V>,
    D: Expr<'a, V>,
    E::SQLType: Compatible<D::SQLType>,
    E::Nullable: NullOr<D::Nullable>,
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
pub fn lead<'a, V, E>(expr: E) -> WindowFnExpr<'a, V, E::SQLType, Null>
where
    V: SQLParam + 'a,
    E: Expr<'a, V>,
{
    WindowFnExpr::new(SQL::func("LEAD", expr.into_sql()))
}

/// LEAD(expr, offset, default) — value of expr from N rows ahead with a default.
///
/// Nullability is the combination of the expression's and default's nullability.
pub fn lead_with_default<'a, V, E, D>(
    expr: E,
    offset: usize,
    default: D,
) -> WindowFnExpr<'a, V, E::SQLType, <E::Nullable as NullOr<D::Nullable>>::Output>
where
    V: SQLParam + 'a,
    E: Expr<'a, V>,
    D: Expr<'a, V>,
    E::SQLType: Compatible<D::SQLType>,
    E::Nullable: NullOr<D::Nullable>,
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

/// FIRST_VALUE(expr) — value of expr from the first row of the frame.
///
/// Always nullable (frame may be empty for some edge cases).
pub fn first_value<'a, V, E>(expr: E) -> WindowFnExpr<'a, V, E::SQLType, Null>
where
    V: SQLParam + 'a,
    E: Expr<'a, V>,
{
    WindowFnExpr::new(SQL::func("FIRST_VALUE", expr.into_sql()))
}

/// LAST_VALUE(expr) — value of expr from the last row of the frame.
///
/// Always nullable (frame boundaries affect result).
pub fn last_value<'a, V, E>(expr: E) -> WindowFnExpr<'a, V, E::SQLType, Null>
where
    V: SQLParam + 'a,
    E: Expr<'a, V>,
{
    WindowFnExpr::new(SQL::func("LAST_VALUE", expr.into_sql()))
}

/// NTH_VALUE(expr, n) — value of expr from the nth row of the frame.
///
/// Always nullable (n may exceed frame size).
pub fn nth_value<'a, V, E>(expr: E, n: usize) -> WindowFnExpr<'a, V, E::SQLType, Null>
where
    V: SQLParam + 'a,
    E: Expr<'a, V>,
{
    let args = expr.into_sql().push(Token::COMMA).append(SQL::number(n));
    WindowFnExpr::new(SQL::func("NTH_VALUE", args))
}

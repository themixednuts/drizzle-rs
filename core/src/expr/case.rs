//! `CASE WHEN ... THEN ... ELSE ... END` expressions.
//!
//! [`case`] starts a builder. The first `.when(condition, result)` fixes the
//! result type; later branches and the `ELSE` value must have a compatible
//! type. Finish with [`r#else`](CaseBuilder#method.else) or
//! [`end`](CaseBuilder::end).

use core::marker::PhantomData;

use crate::sql::{SQL, Token};
use crate::traits::SQLParam;
use crate::types::{BooleanLike, Compatible, DataType};

use super::{AggregateKind, Expr, Null, Nullability, SQLExpr};
use crate::scope::ScopeOnly;

// =============================================================================
// Entry Point
// =============================================================================

/// Starts a searched `CASE` expression.
///
/// Add at least one branch with [`when`](CaseInit::when), then finish with
/// [`r#else`](CaseBuilder#method.else) or [`end`](CaseBuilder::end). The first
/// branch's result sets the type of the whole expression; later results must
/// have a compatible type.
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
/// let group = case()
///     .when(gt(users.age, 65), "senior")
///     .when(gt(users.age, 17), "adult")
///     .r#else("minor");
/// assert_eq!(
///     group.sql(),
///     r#"CASE WHEN "users"."age" > ? THEN ? WHEN "users"."age" > ? THEN ? ELSE ? END"#
/// );
/// ```
///
/// # Type safety
///
/// Branches with different result types do not compile:
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
/// let wrong = case().when(gt(users.age, 65), "senior").r#else(0);
/// ```
#[must_use]
pub fn case<'a, V: SQLParam>() -> CaseInit<'a, V> {
    CaseInit {
        sql: SQL::from(Token::CASE),
        _marker: PhantomData,
    }
}

// =============================================================================
// CaseInit — before the first WHEN (no type established yet)
// =============================================================================

/// A `CASE` builder before its first branch; created by [`case`].
///
/// The result type is set by the first [`when`](Self::when).
pub struct CaseInit<'a, V: SQLParam> {
    sql: SQL<'a, V>,
    _marker: PhantomData<V>,
}

impl<'a, V: SQLParam + 'a> CaseInit<'a, V> {
    /// Adds the first `WHEN condition THEN result` branch.
    ///
    /// `condition` must be boolean. `result` sets the type of the whole `CASE`.
    #[allow(clippy::type_complexity)]
    pub fn when<C, R>(
        self,
        condition: C,
        result: R,
    ) -> CaseBuilder<
        'a,
        V,
        R::SQLType,
        R::Nullable,
        <C::Aggregate as AggregateKind>::Or<R::Aggregate>,
        (ScopeOnly<C::Sources>, R::Sources),
    >
    where
        C: Expr<'a, V>,
        R: Expr<'a, V>,
        C::SQLType: BooleanLike,
    {
        let sql = self
            .sql
            .push(Token::WHEN)
            .append(condition.into_expr_sql())
            .push(Token::THEN)
            .append(result.into_expr_sql());

        CaseBuilder {
            sql,
            _marker: PhantomData,
        }
    }
}

// =============================================================================
// CaseBuilder — after at least one WHEN (type T established)
// =============================================================================

/// A `CASE` builder with at least one branch.
///
/// `T` is the result type set by the first branch and `N` the nullability of
/// the results so far. `S` records the tables read so far: `WHEN` conditions
/// are only scope-checked (a NULL condition falls through to the next
/// branch), while `THEN` results can make the result NULL.
pub struct CaseBuilder<'a, V: SQLParam, T: DataType, N: Nullability, A: AggregateKind, S = ()> {
    sql: SQL<'a, V>,
    _marker: super::TypeMarker<(V, T, N, A, S)>,
}

impl<'a, V, T, N, A, S> CaseBuilder<'a, V, T, N, A, S>
where
    V: SQLParam + 'a,
    T: DataType,
    N: Nullability,
    A: AggregateKind,
{
    /// Adds another `WHEN condition THEN result` branch.
    ///
    /// `condition` must be boolean and `result` must have a type compatible with
    /// the first branch. The result becomes nullable if this branch's result is.
    #[allow(clippy::type_complexity)]
    pub fn when<C, R>(
        self,
        condition: C,
        result: R,
    ) -> CaseBuilder<
        'a,
        V,
        T,
        <N as Nullability>::Or<R::Nullable>,
        <<A as AggregateKind>::Or<C::Aggregate> as AggregateKind>::Or<R::Aggregate>,
        (S, (ScopeOnly<C::Sources>, R::Sources)),
    >
    where
        C: Expr<'a, V>,
        R: Expr<'a, V>,
        C::SQLType: BooleanLike,
        T: Compatible<R::SQLType>,
        N: Nullability,
        R::Nullable: Nullability,
        A: AggregateKind,
        C::Aggregate: AggregateKind,
        R::Aggregate: AggregateKind,
    {
        let sql = self
            .sql
            .push(Token::WHEN)
            .append(condition.into_expr_sql())
            .push(Token::THEN)
            .append(result.into_expr_sql());

        CaseBuilder {
            sql,
            _marker: PhantomData,
        }
    }

    /// Finishes the expression without `ELSE` (`... END`).
    ///
    /// Rows that match no branch give NULL, so the result is always nullable.
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
    /// let label = case().when(users.active, "active").end();
    /// assert_eq!(label.sql(), r#"CASE WHEN "users"."active" THEN ? END"#);
    /// ```
    pub fn end(self) -> SQLExpr<'a, V, T, Null, A, S> {
        let sql = self.sql.push(Token::END);
        SQLExpr::new(sql)
    }

    /// Finishes the expression with a default (`... ELSE default END`).
    ///
    /// `default` must have a type compatible with the first branch. The result is
    /// nullable if any branch result or the default is.
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
    /// let label = case().when(users.active, "active").r#else("inactive");
    /// assert_eq!(label.sql(), r#"CASE WHEN "users"."active" THEN ? ELSE ? END"#);
    /// ```
    #[allow(clippy::type_complexity)]
    pub fn r#else<D>(
        self,
        default: D,
    ) -> SQLExpr<
        'a,
        V,
        T,
        <N as Nullability>::Or<D::Nullable>,
        <A as AggregateKind>::Or<D::Aggregate>,
        (S, D::Sources),
    >
    where
        D: Expr<'a, V>,
        T: Compatible<D::SQLType>,
        N: Nullability,
        D::Nullable: Nullability,
        A: AggregateKind,
        D::Aggregate: AggregateKind,
    {
        let sql = self
            .sql
            .push(Token::ELSE)
            .append(default.into_expr_sql())
            .push(Token::END);
        SQLExpr::new(sql)
    }
}

//! Type-safe CASE/WHEN expressions.
//!
//! Provides a typestate builder for SQL CASE expressions that tracks the
//! result type and nullability through each WHEN branch and the optional
//! ELSE clause.
//!
//! # Example
//!
//! ```ignore
//! use drizzle_core::expr::*;
//!
//! // Searched CASE with ELSE — result is NonNull Text
//! case()
//!     .when(gt(users.age, 65), "Senior")
//!     .when(gt(users.age, 18), "Adult")
//!     .r#else("Minor")
//!
//! // Without ELSE — result is always Null
//! case()
//!     .when(gt(users.age, 18), "Adult")
//!     .end()
//! ```

use core::marker::PhantomData;

use crate::sql::{SQL, Token};
use crate::traits::SQLParam;
use crate::types::{BooleanLike, Compatible, DataType};

use super::null::NullOr;
use super::{AggOr, AggregateKind, Expr, Null, Nullability, SQLExpr};

// =============================================================================
// Entry Point
// =============================================================================

/// Start building a searched CASE expression.
///
/// Returns a `CaseInit` which requires at least one `.when()` call before
/// it can be finished with `.end()` or `.r#else()`.
pub fn case<'a, V: SQLParam>() -> CaseInit<'a, V> {
    CaseInit {
        sql: SQL::from(Token::CASE),
        _marker: PhantomData,
    }
}

// =============================================================================
// CaseInit — before the first WHEN (no type established yet)
// =============================================================================

/// Builder state before the first WHEN branch.
///
/// The result type is not yet known — it will be set by the first `.when()`.
pub struct CaseInit<'a, V: SQLParam> {
    sql: SQL<'a, V>,
    _marker: PhantomData<V>,
}

impl<'a, V: SQLParam + 'a> CaseInit<'a, V> {
    /// Add the first WHEN branch. This establishes the result type.
    ///
    /// ```ignore
    /// case().when(gt(users.age, 65), "Senior")
    /// // Type T = Text, Nullability N = NonNull (from &str literal)
    /// ```
    #[allow(clippy::type_complexity)]
    pub fn when<C, R>(
        self,
        condition: C,
        result: R,
    ) -> CaseBuilder<'a, V, R::SQLType, R::Nullable, <C::Aggregate as AggOr<R::Aggregate>>::Output>
    where
        C: Expr<'a, V>,
        R: Expr<'a, V>,
        C::SQLType: BooleanLike,
        C::Aggregate: AggOr<R::Aggregate>,
    {
        let sql = self
            .sql
            .push(Token::WHEN)
            .append(condition.into_sql())
            .push(Token::THEN)
            .append(result.into_sql());

        CaseBuilder {
            sql,
            _marker: PhantomData,
        }
    }
}

// =============================================================================
// CaseBuilder — after at least one WHEN (type T established)
// =============================================================================

/// Builder state after at least one WHEN branch has been added.
///
/// The result type `T` and accumulated nullability `N` are tracked.
pub struct CaseBuilder<'a, V: SQLParam, T: DataType, N: Nullability, A: AggregateKind> {
    sql: SQL<'a, V>,
    _marker: PhantomData<(V, T, N, A)>,
}

impl<'a, V, T, N, A> CaseBuilder<'a, V, T, N, A>
where
    V: SQLParam + 'a,
    T: DataType,
    N: Nullability,
    A: AggregateKind,
{
    /// Add another WHEN branch.
    ///
    /// The result type must be compatible with the type established by the
    /// first branch. Nullability is accumulated via `NullOr`.
    #[allow(clippy::type_complexity)]
    pub fn when<C, R>(
        self,
        condition: C,
        result: R,
    ) -> CaseBuilder<
        'a,
        V,
        T,
        <N as NullOr<R::Nullable>>::Output,
        <<A as AggOr<C::Aggregate>>::Output as AggOr<R::Aggregate>>::Output,
    >
    where
        C: Expr<'a, V>,
        R: Expr<'a, V>,
        C::SQLType: BooleanLike,
        T: Compatible<R::SQLType>,
        N: NullOr<R::Nullable>,
        R::Nullable: Nullability,
        A: AggOr<C::Aggregate>,
        <A as AggOr<C::Aggregate>>::Output: AggOr<R::Aggregate>,
        C::Aggregate: AggregateKind,
        R::Aggregate: AggregateKind,
    {
        let sql = self
            .sql
            .push(Token::WHEN)
            .append(condition.into_sql())
            .push(Token::THEN)
            .append(result.into_sql());

        CaseBuilder {
            sql,
            _marker: PhantomData,
        }
    }

    /// Finish the CASE expression without an ELSE clause.
    ///
    /// Without ELSE, unmatched rows produce NULL, so the result is always
    /// `Null` regardless of branch nullability.
    pub fn end(self) -> SQLExpr<'a, V, T, Null, A> {
        let sql = self.sql.push(Token::END);
        SQLExpr::new(sql)
    }

    /// Finish the CASE expression with an ELSE clause.
    ///
    /// The ELSE value must have a compatible type. Nullability is the
    /// combination of all branch nullabilities and the default's nullability.
    #[allow(clippy::type_complexity)]
    pub fn r#else<D>(
        self,
        default: D,
    ) -> SQLExpr<'a, V, T, <N as NullOr<D::Nullable>>::Output, <A as AggOr<D::Aggregate>>::Output>
    where
        D: Expr<'a, V>,
        T: Compatible<D::SQLType>,
        N: NullOr<D::Nullable>,
        D::Nullable: Nullability,
        A: AggOr<D::Aggregate>,
        D::Aggregate: AggregateKind,
    {
        let sql = self
            .sql
            .push(Token::ELSE)
            .append(default.into_sql())
            .push(Token::END);
        SQLExpr::new(sql)
    }
}

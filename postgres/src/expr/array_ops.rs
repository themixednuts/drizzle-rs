//! `PostgreSQL` array operators.
//!
//! This module provides PostgreSQL-specific array operators:
//! - `@>` (contains)
//! - `<@` (contained by)
//! - `&&` (overlaps)
//!
//! # Example
//!
//! ```
//! # use drizzle_postgres::expr::{array_contains, PgArray};
//! # use drizzle_core::{SQL, ToSQL};
//! # use drizzle_postgres::values::PostgresValue;
//! let tags = SQL::<PostgresValue>::raw("tags");
//! let condition = array_contains(tags, PgArray(vec!["test"]));
//! assert!(condition.to_sql().sql().contains("@>"));
//! ```

#[cfg(not(feature = "std"))]
use crate::prelude::*;
use crate::values::PostgresValue;
use drizzle_core::ToSQL;
use drizzle_core::expr::{AggOr, Expr, ExprSources, NonNull, SQLExpr, Scalar};
use drizzle_core::scope::Arg;
use drizzle_core::sql::{SQL, SQLChunk};
use drizzle_types::postgres::types::{Any, Boolean};
use drizzle_types::{Array, Compatible, DataType, Placeholder};

/// Left operand type `Self` of an array operator accepts the right operand
/// type `Rhs`: both are arrays of compatible element types, or one side is
/// untyped SQL or a placeholder.
#[diagnostic::on_unimplemented(
    message = "PostgreSQL array operators cannot combine `{Self}` with `{Rhs}`",
    label = "both operands must be arrays with compatible element types",
    note = "pass a bound array with `PgArray(vec![...])`; a bare value is not an array"
)]
pub trait ArrayOperand<Rhs> {}

impl<T: DataType, U: DataType> ArrayOperand<Array<U>> for Array<T> where T: Compatible<U> {}
impl<T: DataType> ArrayOperand<Any> for Array<T> {}
impl<T: DataType> ArrayOperand<Placeholder> for Array<T> {}
impl<R> ArrayOperand<R> for Any {}

/// Wrapper for passing a `Vec<T>` as a single `PostgreSQL` array parameter.
///
/// Without this wrapper, `Vec<T>` implements `ToSQL` by joining elements
/// with commas (`$1, $2, $3`), which is correct for `IN (...)` clauses
/// but wrong for array operators like `@>`, `<@`, and `&&` which expect
/// a single array parameter.
///
/// # Example
///
/// ```
/// # use drizzle_postgres::expr::{array_contains, PgArray};
/// # use drizzle_core::{SQL, ToSQL};
/// # use drizzle_postgres::values::PostgresValue;
/// let tags = SQL::<PostgresValue>::raw("tags");
/// // Correct: passes as a single array parameter
/// let condition = array_contains(tags, PgArray(vec!["rust", "python"]));
/// ```
pub struct PgArray<T>(pub Vec<T>);

impl<'a, T> ToSQL<'a, PostgresValue<'a>> for PgArray<T>
where
    T: Into<PostgresValue<'a>> + Clone,
{
    fn to_sql(&self) -> SQL<'a, PostgresValue<'a>> {
        let array: Vec<PostgresValue<'a>> = self.0.iter().map(|v| v.clone().into()).collect();
        SQL::param(PostgresValue::Array(array))
    }
}

impl<T> ExprSources for PgArray<T> {
    type Sources = ();
}

impl<'a, T> Expr<'a, PostgresValue<'a>> for PgArray<T>
where
    T: Expr<'a, PostgresValue<'a>> + Into<PostgresValue<'a>> + Clone,
{
    type SQLType = Array<T::SQLType>;
    type Nullable = NonNull;
    type Aggregate = Scalar;
}

/// `PostgreSQL` `@>` operator - array contains.
///
/// Returns true if the left array contains all elements of the right array.
///
/// # Example
///
/// ```
/// # use drizzle_postgres::expr::{array_contains, PgArray};
/// # use drizzle_core::{SQL, ToSQL};
/// # use drizzle_postgres::values::PostgresValue;
/// let tags = SQL::<PostgresValue>::raw("tags");
/// let condition = array_contains(tags, PgArray(vec!["rust"]));
/// assert!(condition.to_sql().sql().contains("@>"));
/// // Generates: tags @> $1
/// ```
pub fn array_contains<'a, L, R>(
    left: L,
    right: R,
) -> SQLExpr<
    'a,
    PostgresValue<'a>,
    Boolean,
    NonNull,
    <L::Aggregate as AggOr<R::Aggregate>>::Output,
    (Arg<L::Nullable, L::Sources>, Arg<R::Nullable, R::Sources>),
>
where
    L: Expr<'a, PostgresValue<'a>>,
    L::Aggregate: AggOr<R::Aggregate>,
    R: Expr<'a, PostgresValue<'a>>,
    L::SQLType: ArrayOperand<R::SQLType>,
{
    SQLExpr::new(
        left.to_sql()
            .push(SQLChunk::Raw("@>".into()))
            .append(right.to_sql()),
    )
}

/// `PostgreSQL` `<@` operator - array is contained by.
///
/// Returns true if the left array is contained by the right array
/// (i.e., all elements of left are in right).
///
/// # Example
///
/// ```
/// # use drizzle_postgres::expr::{array_contained, PgArray};
/// # use drizzle_core::{SQL, ToSQL};
/// # use drizzle_postgres::values::PostgresValue;
/// let tags = SQL::<PostgresValue>::raw("tags");
/// let condition = array_contained(tags, PgArray(vec!["rust"]));
/// assert!(condition.to_sql().sql().contains("<@"));
/// // Generates: tags <@ $1
/// ```
pub fn array_contained<'a, L, R>(
    left: L,
    right: R,
) -> SQLExpr<
    'a,
    PostgresValue<'a>,
    Boolean,
    NonNull,
    <L::Aggregate as AggOr<R::Aggregate>>::Output,
    (Arg<L::Nullable, L::Sources>, Arg<R::Nullable, R::Sources>),
>
where
    L: Expr<'a, PostgresValue<'a>>,
    L::Aggregate: AggOr<R::Aggregate>,
    R: Expr<'a, PostgresValue<'a>>,
    L::SQLType: ArrayOperand<R::SQLType>,
{
    SQLExpr::new(
        left.to_sql()
            .push(SQLChunk::Raw("<@".into()))
            .append(right.to_sql()),
    )
}

/// `PostgreSQL` `&&` operator - arrays overlap.
///
/// Returns true if the arrays have any elements in common.
///
/// # Example
///
/// ```
/// # use drizzle_postgres::expr::{array_overlaps, PgArray};
/// # use drizzle_core::{SQL, ToSQL};
/// # use drizzle_postgres::values::PostgresValue;
/// let tags = SQL::<PostgresValue>::raw("tags");
/// let condition = array_overlaps(tags, PgArray(vec!["rust"]));
/// assert!(condition.to_sql().sql().contains("&&"));
/// // Generates: tags && $1
/// ```
pub fn array_overlaps<'a, L, R>(
    left: L,
    right: R,
) -> SQLExpr<
    'a,
    PostgresValue<'a>,
    Boolean,
    NonNull,
    <L::Aggregate as AggOr<R::Aggregate>>::Output,
    (Arg<L::Nullable, L::Sources>, Arg<R::Nullable, R::Sources>),
>
where
    L: Expr<'a, PostgresValue<'a>>,
    L::Aggregate: AggOr<R::Aggregate>,
    R: Expr<'a, PostgresValue<'a>>,
    L::SQLType: ArrayOperand<R::SQLType>,
{
    SQLExpr::new(
        left.to_sql()
            .push(SQLChunk::Raw("&&".into()))
            .append(right.to_sql()),
    )
}

/// Extension trait providing method-based array operators for `PostgreSQL` expressions.
///
/// This trait provides `.array_contains()`, `.array_contained()`, and `.array_overlaps()`
/// methods on any expression type.
///
/// # Example
///
/// ```
/// # use drizzle_postgres::expr::{ArrayExprExt, PgArray};
/// # use drizzle_core::{SQL, ToSQL};
/// # use drizzle_postgres::values::PostgresValue;
/// let tags = SQL::<PostgresValue>::raw("tags");
/// let condition = tags.array_contains(PgArray(vec!["rust"]));
/// assert!(condition.to_sql().sql().contains("@>"));
/// ```
pub trait ArrayExprExt<'a>: Expr<'a, PostgresValue<'a>> + Sized {
    /// `PostgreSQL` `@>` operator - array contains.
    ///
    /// Returns true if self contains all elements of the other array.
    fn array_contains<R>(
        self,
        other: R,
    ) -> SQLExpr<
        'a,
        PostgresValue<'a>,
        Boolean,
        NonNull,
        <Self::Aggregate as AggOr<R::Aggregate>>::Output,
        (
            Arg<Self::Nullable, Self::Sources>,
            Arg<R::Nullable, R::Sources>,
        ),
    >
    where
        Self::Aggregate: AggOr<R::Aggregate>,
        R: Expr<'a, PostgresValue<'a>>,
        Self::SQLType: ArrayOperand<R::SQLType>,
    {
        array_contains(self, other)
    }

    /// `PostgreSQL` `<@` operator - array is contained by.
    ///
    /// Returns true if self is contained by the other array.
    fn array_contained<R>(
        self,
        other: R,
    ) -> SQLExpr<
        'a,
        PostgresValue<'a>,
        Boolean,
        NonNull,
        <Self::Aggregate as AggOr<R::Aggregate>>::Output,
        (
            Arg<Self::Nullable, Self::Sources>,
            Arg<R::Nullable, R::Sources>,
        ),
    >
    where
        Self::Aggregate: AggOr<R::Aggregate>,
        R: Expr<'a, PostgresValue<'a>>,
        Self::SQLType: ArrayOperand<R::SQLType>,
    {
        array_contained(self, other)
    }

    /// `PostgreSQL` `&&` operator - arrays overlap.
    ///
    /// Returns true if self and the other array have any elements in common.
    fn array_overlaps<R>(
        self,
        other: R,
    ) -> SQLExpr<
        'a,
        PostgresValue<'a>,
        Boolean,
        NonNull,
        <Self::Aggregate as AggOr<R::Aggregate>>::Output,
        (
            Arg<Self::Nullable, Self::Sources>,
            Arg<R::Nullable, R::Sources>,
        ),
    >
    where
        Self::Aggregate: AggOr<R::Aggregate>,
        R: Expr<'a, PostgresValue<'a>>,
        Self::SQLType: ArrayOperand<R::SQLType>,
    {
        array_overlaps(self, other)
    }
}

/// Blanket implementation for all `PostgreSQL` `Expr` types.
impl<'a, E: Expr<'a, PostgresValue<'a>>> ArrayExprExt<'a> for E {}

//! PostgreSQL array operators.
//!
//! This module provides PostgreSQL-specific array operators:
//! - `@>` (contains)
//! - `<@` (contained by)
//! - `&&` (overlaps)
//!
//! # Example
//!
//! ```
//! # use drizzle_postgres::expr::array_contains;
//! # use drizzle_core::{SQL, ToSQL};
//! # use drizzle_postgres::values::PostgresValue;
//! let tags = SQL::<PostgresValue>::raw("tags");
//! let condition = array_contains(tags, "test");
//! assert!(condition.to_sql().sql().contains("@>"));
//! ```

use crate::values::PostgresValue;
use drizzle_core::ToSQL;
use drizzle_core::expr::{Expr, NonNull, SQLExpr, Scalar};
use drizzle_core::sql::SQLChunk;
use drizzle_core::types::Bool;

/// PostgreSQL `@>` operator - array contains.
///
/// Returns true if the left array contains all elements of the right array.
///
/// # Example
///
/// ```
/// # use drizzle_postgres::expr::array_contains;
/// # use drizzle_core::{SQL, ToSQL};
/// # use drizzle_postgres::values::PostgresValue;
/// let tags = SQL::<PostgresValue>::raw("tags");
/// let condition = array_contains(tags, "rust");
/// assert!(condition.to_sql().sql().contains("@>"));
/// // Generates: tags @> $1
/// ```
pub fn array_contains<'a, L, R>(
    left: L,
    right: R,
) -> SQLExpr<'a, PostgresValue<'a>, Bool, NonNull, Scalar>
where
    L: Expr<'a, PostgresValue<'a>>,
    R: ToSQL<'a, PostgresValue<'a>>,
{
    SQLExpr::new(
        left.to_sql()
            .push(SQLChunk::Raw("@>".into()))
            .append(right.to_sql()),
    )
}

/// PostgreSQL `<@` operator - array is contained by.
///
/// Returns true if the left array is contained by the right array
/// (i.e., all elements of left are in right).
///
/// # Example
///
/// ```
/// # use drizzle_postgres::expr::array_contained;
/// # use drizzle_core::{SQL, ToSQL};
/// # use drizzle_postgres::values::PostgresValue;
/// let tags = SQL::<PostgresValue>::raw("tags");
/// let condition = array_contained(tags, "rust");
/// assert!(condition.to_sql().sql().contains("<@"));
/// // Generates: tags <@ $1
/// ```
pub fn array_contained<'a, L, R>(
    left: L,
    right: R,
) -> SQLExpr<'a, PostgresValue<'a>, Bool, NonNull, Scalar>
where
    L: Expr<'a, PostgresValue<'a>>,
    R: ToSQL<'a, PostgresValue<'a>>,
{
    SQLExpr::new(
        left.to_sql()
            .push(SQLChunk::Raw("<@".into()))
            .append(right.to_sql()),
    )
}

/// PostgreSQL `&&` operator - arrays overlap.
///
/// Returns true if the arrays have any elements in common.
///
/// # Example
///
/// ```
/// # use drizzle_postgres::expr::array_overlaps;
/// # use drizzle_core::{SQL, ToSQL};
/// # use drizzle_postgres::values::PostgresValue;
/// let tags = SQL::<PostgresValue>::raw("tags");
/// let condition = array_overlaps(tags, "rust");
/// assert!(condition.to_sql().sql().contains("&&"));
/// // Generates: tags && $1
/// ```
pub fn array_overlaps<'a, L, R>(
    left: L,
    right: R,
) -> SQLExpr<'a, PostgresValue<'a>, Bool, NonNull, Scalar>
where
    L: Expr<'a, PostgresValue<'a>>,
    R: ToSQL<'a, PostgresValue<'a>>,
{
    SQLExpr::new(
        left.to_sql()
            .push(SQLChunk::Raw("&&".into()))
            .append(right.to_sql()),
    )
}

/// Extension trait providing method-based array operators for PostgreSQL expressions.
///
/// This trait provides `.array_contains()`, `.array_contained()`, and `.array_overlaps()`
/// methods on any expression type.
///
/// # Example
///
/// ```
/// # use drizzle_postgres::expr::ArrayExprExt;
/// # use drizzle_core::{SQL, ToSQL};
/// # use drizzle_postgres::values::PostgresValue;
/// let tags = SQL::<PostgresValue>::raw("tags");
/// let condition = tags.array_contains("rust");
/// assert!(condition.to_sql().sql().contains("@>"));
/// ```
pub trait ArrayExprExt<'a>: Expr<'a, PostgresValue<'a>> + Sized {
    /// PostgreSQL `@>` operator - array contains.
    ///
    /// Returns true if self contains all elements of the other array.
    fn array_contains<R>(self, other: R) -> SQLExpr<'a, PostgresValue<'a>, Bool, NonNull, Scalar>
    where
        R: ToSQL<'a, PostgresValue<'a>>,
    {
        array_contains(self, other)
    }

    /// PostgreSQL `<@` operator - array is contained by.
    ///
    /// Returns true if self is contained by the other array.
    fn array_contained<R>(self, other: R) -> SQLExpr<'a, PostgresValue<'a>, Bool, NonNull, Scalar>
    where
        R: ToSQL<'a, PostgresValue<'a>>,
    {
        array_contained(self, other)
    }

    /// PostgreSQL `&&` operator - arrays overlap.
    ///
    /// Returns true if self and the other array have any elements in common.
    fn array_overlaps<R>(self, other: R) -> SQLExpr<'a, PostgresValue<'a>, Bool, NonNull, Scalar>
    where
        R: ToSQL<'a, PostgresValue<'a>>,
    {
        array_overlaps(self, other)
    }
}

/// Blanket implementation for all PostgreSQL `Expr` types.
impl<'a, E: Expr<'a, PostgresValue<'a>>> ArrayExprExt<'a> for E {}

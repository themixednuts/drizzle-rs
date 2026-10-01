//! Type-safe SQL expressions: comparisons, logic, aggregates and functions.
//!
//! Every expression carries three facts in its type:
//!
//! - its SQL type, such as the dialect's integer or text type;
//! - whether it can be NULL ([`NonNull`] or [`Null`]);
//! - whether it is a plain value or an aggregate ([`Scalar`] or [`Agg`]).
//!
//! The functions in this module read those facts from their arguments and
//! compute them for their result. Comparing a number with text, summing a text
//! column, or calling a PostgreSQL-only function on SQLite fails to compile.
//! Expressions also record which tables they read ([`ExprSources`]); a query
//! checks that against its `FROM`/`JOIN` scope.
//!
//! Rust values (`i32`, `&str`, `bool`, ...) can be used wherever an expression
//! is expected. They are sent as bound parameters (`?` on SQLite and MySQL,
//! `$1`, `$2`, ... on PostgreSQL).
//!
//! The examples in this module use a `users` table with the columns `id`,
//! `age` (integer), `name` (text), `email` (nullable text), `score` (nullable
//! real), `active` (boolean) and `created_at` (timestamp).
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
//! let adults = and(gt(users.age, 18), like(users.name, "A%"));
//! assert_eq!(adults.sql(), r#"("users"."age" > ? AND "users"."name" LIKE ?)"#);
//!
//! // Arithmetic uses the Rust operators.
//! let next_year = users.age.clone() + 1;
//! assert_eq!(next_year.sql(), r#""users"."age" + ?"#);
//! ```
//!
//! Comparing an integer column with text does not compile:
//!
//! ```rust,compile_fail
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
//! let wrong = eq(users.age, "hello");
//! ```

mod agg;
mod case;
mod cmp;
mod column_ops;
mod cond;
mod datetime;
mod logical;
mod math;
mod null;
mod ops;
mod primitives;
mod seq;
mod set;
mod string;
mod subquery;
mod typed;
mod util;
mod window;

/// Zero-sized marker for type parameters a struct only carries at the type
/// level. `fn() -> T` keeps the struct `Send`, `Sync` and covariant in `T`
/// whatever `T` is.
pub(crate) type TypeMarker<T> = core::marker::PhantomData<fn() -> T>;

pub use agg::*;
pub use case::*;
pub use cmp::*;
#[doc(hidden)]
pub use column_ops::*;
pub use cond::*;
pub use datetime::*;
pub use logical::*;
pub use math::*;
pub use null::*;
pub use seq::*;
pub use set::*;
pub use string::*;
pub use subquery::*;
// ops has only trait impls - no items to re-export
pub use typed::*;
pub use util::*;
pub use window::*;

use crate::traits::{SQLParam, ToSQL};
use crate::types::DataType;

// =============================================================================
// Sealed Trait Pattern
// =============================================================================

mod private {
    pub trait Sealed {}
}

// =============================================================================
// Nullability Markers
// =============================================================================

/// Type-level marker saying whether an expression can be NULL.
///
/// Implemented only by [`NonNull`] and [`Null`]. The associated types compute
/// the nullability of combined expressions.
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not a valid nullability marker",
    label = "expected `NonNull` or `Null`"
)]
pub trait Nullability: private::Sealed + Copy + Default + 'static {
    /// How a decoded value of Rust type `T` is wrapped: `T` for [`NonNull`],
    /// [`MaybeNull<T>`](crate::row::MaybeNull) for [`Null`].
    type Decoded<T>;

    /// NULL propagation: nullable when either side is (`a + b`, `f(a, b)`).
    ///
    /// | Self | Rhs | Or |
    /// |------|-----|----|
    /// | NonNull | NonNull | NonNull |
    /// | NonNull | Null | Null |
    /// | Null | _ | Null |
    type Or<Rhs: Nullability>: Nullability;

    /// NULL absorption: nullable only when both sides are (`COALESCE(a, b)`).
    ///
    /// | Self | Rhs | And |
    /// |------|-----|-----|
    /// | NonNull | _ | NonNull |
    /// | Null | NonNull | NonNull |
    /// | Null | Null | Null |
    type And<Rhs: Nullability>: Nullability;
}

/// Nullability marker: the expression is never NULL.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct NonNull;

/// Nullability marker: the expression can be NULL.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct Null;

impl private::Sealed for NonNull {}
impl private::Sealed for Null {}
impl Nullability for NonNull {
    type Decoded<T> = T;
    type Or<Rhs: Nullability> = Rhs;
    type And<Rhs: Nullability> = Self;
}
impl Nullability for Null {
    type Decoded<T> = crate::row::MaybeNull<T>;
    type Or<Rhs: Nullability> = Self;
    type And<Rhs: Nullability> = Rhs;
}

/// Says whether a value of nullability `Source` may be assigned to a column
/// with nullability `Self`.
///
/// Non-null values can be assigned to any column. Nullable values can only be
/// assigned to nullable columns.
#[doc(hidden)]
#[diagnostic::on_unimplemented(
    message = "a nullable expression cannot be assigned to a non-null column",
    label = "this assignment could produce NULL",
    note = "handle the NULL case in the expression or make the target column nullable"
)]
pub trait AcceptsNullability<Source: Nullability>: Nullability {}

impl AcceptsNullability<NonNull> for NonNull {}
impl AcceptsNullability<NonNull> for Null {}
impl AcceptsNullability<Null> for Null {}

// =============================================================================
// Aggregate Kind Markers
// =============================================================================

/// Type-level marker saying whether an expression is an aggregate.
///
/// Implemented only by [`Scalar`] and [`Agg`]. Query builders use it to check
/// that a SELECT list does not mix aggregates and plain columns without a
/// `GROUP BY`.
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not a valid aggregate marker",
    label = "expected `Scalar` or `Agg`"
)]
pub trait AggregateKind: private::Sealed + Copy + Default + 'static {
    /// Aggregate propagation: an expression over any aggregate is itself
    /// aggregate (`SUM(x) + 5`).
    ///
    /// | Self | Rhs | Or |
    /// |------|-----|----|
    /// | Scalar | Scalar | Scalar |
    /// | Scalar | Agg | Agg |
    /// | Agg | _ | Agg |
    type Or<Rhs: AggregateKind>: AggregateKind;

    /// The SELECT-list status of a single expression of this kind
    /// ([`AllScalar`] or [`AllAgg`]).
    type Status;
}

/// Aggregate marker: a plain per-row expression (not an aggregate).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct Scalar;

/// Aggregate marker: an aggregate expression such as `COUNT(...)` or `SUM(...)`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct Agg;

impl private::Sealed for Scalar {}
impl private::Sealed for Agg {}
impl AggregateKind for Scalar {
    type Or<Rhs: AggregateKind> = Rhs;
    type Status = AllScalar;
}
impl AggregateKind for Agg {
    type Or<Rhs: AggregateKind> = Self;
    type Status = AllAgg;
}

// =============================================================================
// Aggregate Status (for SELECT list validation)
// =============================================================================

/// SELECT-list status: every selected expression is scalar.
#[derive(Debug, Clone, Copy, Default)]
pub struct AllScalar;

/// SELECT-list status: every selected expression is an aggregate.
#[derive(Debug, Clone, Copy, Default)]
pub struct AllAgg;

/// SELECT-list status: the list mixes scalar and aggregate expressions.
#[derive(Debug, Clone, Copy, Default)]
pub struct MixedAgg;

/// Combines the SELECT-list statuses of two expressions.
///
/// | Left | Right | Output |
/// |------|-------|--------|
/// | AllScalar | AllScalar | AllScalar |
/// | AllAgg | AllAgg | AllAgg |
/// | anything else | _ | MixedAgg |
pub trait CombineAggStatus<Rhs> {
    type Output;
}

impl CombineAggStatus<Self> for AllScalar {
    type Output = Self;
}
impl CombineAggStatus<AllAgg> for AllScalar {
    type Output = MixedAgg;
}
impl CombineAggStatus<MixedAgg> for AllScalar {
    type Output = MixedAgg;
}
impl CombineAggStatus<AllScalar> for AllAgg {
    type Output = MixedAgg;
}
impl CombineAggStatus<Self> for AllAgg {
    type Output = Self;
}
impl CombineAggStatus<MixedAgg> for AllAgg {
    type Output = MixedAgg;
}
impl CombineAggStatus<AllScalar> for MixedAgg {
    type Output = Self;
}
impl CombineAggStatus<AllAgg> for MixedAgg {
    type Output = Self;
}
impl CombineAggStatus<Self> for MixedAgg {
    type Output = Self;
}

/// The SELECT-list status of a type that can appear in a SELECT list.
///
/// Implemented for columns (always [`AllScalar`]), [`SQLExpr`], and the
/// expression wrappers in this module.
pub trait HasAggStatus {
    type Status;
}

impl<T: HasAggStatus + ?Sized> HasAggStatus for &T {
    type Status = T::Status;
}

// =============================================================================
// Core Expression Trait
// =============================================================================

/// The tables an expression reads, as a type-level tree.
///
/// Columns record their table, operators combine their operands' trees, and
/// literals, placeholders and raw SQL read nothing (`()`). Queries check the
/// tree against their `FROM`/`JOIN` scope when they are run with `.all()`,
/// `.get()` or `.rows()`, so using a column of a table that was never joined
/// is a compile error. See [`crate::scope`] for the node types.
pub trait ExprSources {
    /// Type-level tree with one [`Src`](crate::scope::Src) leaf per column read.
    type Sources;
}

impl<T: ExprSources + ?Sized> ExprSources for &T {
    type Sources = T::Sources;
}

/// Expression lists (`Cons<E, ...>`) read every element's sources.
impl ExprSources for crate::Nil {
    type Sources = ();
}

impl<Head: ExprSources, Tail: ExprSources> ExprSources for crate::Cons<Head, Tail> {
    type Sources = (Head::Sources, Tail::Sources);
}

/// `()` reads nothing (`COUNT(*)`).
impl ExprSources for () {
    type Sources = ();
}

// Tuples (condition lists, GROUP BY keys, ORDER BY terms) read the sources of
// every element, nested as NULL-propagating pairs.
macro_rules! impl_tuple_expr_sources {
    ($($T:ident),+; $($i:tt),+) => {
        impl<$($T: ExprSources),+> ExprSources for ($($T,)+) {
            type Sources = impl_tuple_expr_sources!(@nest $($T),+);
        }
    };
    (@nest $T:ident) => { <$T as ExprSources>::Sources };
    (@nest $T:ident, $($rest:ident),+) => {
        (<$T as ExprSources>::Sources, impl_tuple_expr_sources!(@nest $($rest),+))
    };
}

with_col_sizes_8!(impl_tuple_expr_sources);

#[cfg(any(
    feature = "col16",
    feature = "col32",
    feature = "col64",
    feature = "col128",
    feature = "col200"
))]
with_col_sizes_16!(impl_tuple_expr_sources);

/// A typed SQL expression.
///
/// Columns, Rust literals, [`SQLExpr`] values and the results of the functions
/// in this module implement `Expr`. The associated types describe the result:
///
/// - `SQLType`: the SQL data type, such as the dialect's integer or text type;
/// - `Nullable`: [`NonNull`] or [`Null`];
/// - `Aggregate`: [`Scalar`] or [`Agg`].
///
/// `V` is the dialect's value type (for example `SQLiteValue` or
/// `PostgresValue`), and `'a` is the lifetime of borrowed values inside the
/// expression. The table macros implement this trait for generated columns;
/// you rarely implement it yourself.
///
/// # Examples
///
/// A helper that accepts any integer expression:
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
/// fn is_adult<'a, E>(age: E) -> impl Expr<'a, Value>
/// where
///     E: Expr<'a, Value, SQLType = Int>,
/// {
///     gt(age, 18)
/// }
///
/// assert_eq!(is_adult(users.age).into_expr_sql().sql(), r#""users"."age" > ?"#);
/// ```
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not a valid SQL expression",
    label = "expected a column, literal, or expression — does this type implement Expr?",
    note = "SQL expressions must have an associated SQLType, Nullable, and Aggregate kind"
)]
pub trait Expr<'a, V: SQLParam>: ToSQL<'a, V> + ExprSources {
    /// The SQL data type this expression evaluates to.
    type SQLType: DataType;

    /// Whether this expression can be NULL.
    type Nullable: Nullability;

    /// Whether this is an aggregate (COUNT, SUM) or scalar expression.
    type Aggregate: AggregateKind;

    /// Renders this value as a scalar expression, borrowing it.
    ///
    /// Most expressions use their `ToSQL` implementation. A few Rust container
    /// types, notably byte buffers, need expression-specific rendering because
    /// their generic `ToSQL` form is a comma-separated list.
    fn to_expr_sql(&self) -> crate::SQL<'a, V> {
        self.to_sql().parens_if_subquery()
    }

    /// Renders this value as a scalar expression, consuming it.
    fn into_expr_sql(self) -> crate::SQL<'a, V>
    where
        Self: Sized,
    {
        self.into_sql().parens_if_subquery()
    }

    /// Renders this value as one element of a [`ConditionList`].
    ///
    /// `None` means the element contributes no condition and is dropped from
    /// the combined SQL. Only `Option::None` does this; every other expression
    /// renders through [`Expr::to_expr_sql`].
    fn to_condition_sql(&self) -> Option<crate::SQL<'a, V>> {
        Some(self.to_expr_sql())
    }

    /// Consuming counterpart of [`Expr::to_condition_sql`].
    fn into_condition_sql(self) -> Option<crate::SQL<'a, V>>
    where
        Self: Sized,
    {
        Some(self.into_expr_sql())
    }
}

// Note: Columns implement Expr via explicit impls generated by macros,
// not via a blanket impl, to avoid conflicts with `impl Expr for &T`.

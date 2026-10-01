//! Condition lists: tuples as conjunctions, plus the [`all`] and [`any`] combinators.
//!
//! A tuple of conditions *is* a condition. It renders as the parenthesized AND
//! of its elements and has the same type as a chain of [`and`](super::and)
//! calls, so `and(a, and(b, c))` can be written `(a, b, c)` anywhere a
//! condition is accepted (for example in `.r#where(...)`). Bare tuples work up
//! to 8 elements; use [`all`] or nested tuples for longer lists.
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
//! let filter = (gt(users.age, 18), is_not_null(users.email), like(users.name, "A%"));
//! assert_eq!(
//!     filter.into_expr_sql().sql(),
//!     r#"("users"."age" > ? AND "users"."email" IS NOT NULL AND "users"."name" LIKE ?)"#
//! );
//!
//! // OR lists use `any`.
//! let staff = any((eq(users.name, "admin"), eq(users.name, "moderator")));
//! assert_eq!(staff.sql(), r#"("users"."name" = ? OR "users"."name" = ?)"#);
//! ```
//!
//! # Optional elements
//!
//! Any element may be an [`Option`]. `None` adds nothing to the SQL, which
//! makes optional filters easy to build:
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
//! let name: Option<&str> = None;
//! let filter = (gt(users.age, 18), name.map(|n| eq(users.name, n)));
//! assert_eq!(filter.into_expr_sql().sql(), r#"("users"."age" > ?)"#);
//! ```
//!
//! When *every* element is `None`, the list renders as the identity of its
//! operator: `TRUE` for a conjunction (a tuple or [`all`]) and `FALSE` for a
//! disjunction ([`any`]). An empty conjunction therefore filters nothing,
//! while an empty disjunction matches no rows instead of silently matching
//! every row.

use crate::dialect::DialectTypes;
use crate::sql::{SQL, Token};
use crate::traits::SQLParam;
use crate::types::BooleanLike;

use super::{AggregateKind, Expr, Nullability, SQLExpr};

/// Rendered SQL for a conjunction whose elements are all absent.
const EMPTY_CONJUNCTION: &str = "TRUE";

/// Rendered SQL for a disjunction whose elements are all absent.
const EMPTY_DISJUNCTION: &str = "FALSE";

mod sealed {
    pub trait Sealed {}
}

// =============================================================================
// ConditionSink
// =============================================================================

/// Collects rendered conditions while a [`ConditionList`] renders.
/// Internal to the crate.
#[doc(hidden)]
#[derive(Debug)]
pub struct ConditionSink<'a, V: SQLParam> {
    sql: SQL<'a, V>,
    separator: Token,
    len: usize,
}

impl<'a, V: SQLParam + 'a> ConditionSink<'a, V> {
    fn new(separator: Token) -> Self {
        Self {
            sql: SQL::empty(),
            separator,
            len: 0,
        }
    }

    /// Append one rendered condition. `None` contributes nothing.
    pub fn push(&mut self, condition: Option<SQL<'a, V>>) {
        let Some(condition) = condition else { return };
        if self.len > 0 {
            self.sql.push_mut(self.separator);
        }
        self.sql.append_mut(condition);
        self.len += 1;
    }

    fn finish(self, empty: &'static str) -> SQL<'a, V> {
        if self.len == 0 {
            SQL::raw(empty)
        } else {
            self.sql.parens()
        }
    }
}

// =============================================================================
// ConditionList
// =============================================================================

/// A tuple of conditions that [`all`] and [`any`] can combine.
///
/// Implemented for tuples of 1 to 8 elements (16 with the `col16` feature,
/// which is on by default). Each element is a boolean expression or an
/// [`Option`] of one. The list is nullable if any element is nullable, and is
/// an aggregate if any element is.
///
/// Tuples of up to 8 elements are also conditions themselves (they implement
/// [`Expr`](super::Expr)). For longer lists, use [`all`] or [`any`], or nest
/// tuples.
///
/// This trait is sealed.
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not a list of SQL conditions",
    label = "expected a tuple of boolean expressions",
    note = "every element must be a boolean-typed expression, or an `Option` of one"
)]
pub trait ConditionList<'a, V: SQLParam>: sealed::Sealed + super::ExprSources {
    /// Nullability folded across every element.
    type Nullable: Nullability;

    /// Aggregate kind folded across every element.
    type Aggregate: AggregateKind;

    /// Render every present element into `sink`, consuming the list.
    #[doc(hidden)]
    fn push_conditions(self, sink: &mut ConditionSink<'a, V>);

    /// Render every present element into `sink`, borrowing the list.
    #[doc(hidden)]
    fn push_conditions_ref(&self, sink: &mut ConditionSink<'a, V>);
}

fn combine<'a, V, L>(conditions: L, separator: Token, empty: &'static str) -> SQL<'a, V>
where
    V: SQLParam + 'a,
    L: ConditionList<'a, V>,
{
    let mut sink = ConditionSink::new(separator);
    conditions.push_conditions(&mut sink);
    sink.finish(empty)
}

fn combine_ref<'a, V, L>(conditions: &L, separator: Token, empty: &'static str) -> SQL<'a, V>
where
    V: SQLParam + 'a,
    L: ConditionList<'a, V>,
{
    let mut sink = ConditionSink::new(separator);
    conditions.push_conditions_ref(&mut sink);
    sink.finish(empty)
}

// =============================================================================
// all / any
// =============================================================================

/// Logical AND of every condition in a tuple.
///
/// Renders `(a AND b AND c)`. `None` elements are skipped, and a list with no
/// present element renders as `TRUE`. The result is nullable if any element
/// is, and is an aggregate if any element is.
///
/// A bare tuple means the same thing where a condition is expected. Use `all`
/// where a tuple would be read as a list of columns instead (for example as a
/// join's `ON` condition), or for lists longer than 8.
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
/// let cond = all((users.active, gt(users.age, 18)));
/// assert_eq!(cond.sql(), r#"("users"."active" AND "users"."age" > ?)"#);
///
/// let nothing = all((None::<SQLExpr<'_, Value, Int>>,));
/// assert_eq!(nothing.sql(), "TRUE");
/// ```
#[allow(clippy::type_complexity)]
pub fn all<'a, V, L>(
    conditions: L,
) -> SQLExpr<'a, V, <V::DialectMarker as DialectTypes>::Bool, L::Nullable, L::Aggregate, L::Sources>
where
    V: SQLParam + 'a,
    L: ConditionList<'a, V>,
{
    SQLExpr::new(combine(conditions, Token::AND, EMPTY_CONJUNCTION))
}

/// Logical OR of every condition in a tuple.
///
/// Renders `(a OR b OR c)`. `None` elements are skipped, and a list with no
/// present element renders as `FALSE`, so an empty OR matches no rows. The
/// result is nullable if any element is, and is an aggregate if any element
/// is.
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
/// let cond = any((eq(users.name, "admin"), lt(users.age, 13)));
/// assert_eq!(cond.sql(), r#"("users"."name" = ? OR "users"."age" < ?)"#);
/// ```
#[allow(clippy::type_complexity)]
pub fn any<'a, V, L>(
    conditions: L,
) -> SQLExpr<'a, V, <V::DialectMarker as DialectTypes>::Bool, L::Nullable, L::Aggregate, L::Sources>
where
    V: SQLParam + 'a,
    L: ConditionList<'a, V>,
{
    SQLExpr::new(combine(conditions, Token::OR, EMPTY_DISJUNCTION))
}

// =============================================================================
// Tuple implementations
// =============================================================================

/// `ConditionList` for a 1-tuple: the markers are the single element's.
macro_rules! impl_condition_one {
    ($T:ident, $i:tt) => {
        impl<'a, V, $T> ConditionList<'a, V> for ($T,)
        where
            V: SQLParam + 'a,
            $T: Expr<'a, V>,
            <$T as Expr<'a, V>>::SQLType: BooleanLike,
        {
            type Nullable = <$T as Expr<'a, V>>::Nullable;
            type Aggregate = <$T as Expr<'a, V>>::Aggregate;

            fn push_conditions(self, sink: &mut ConditionSink<'a, V>) {
                sink.push(self.$i.into_condition_sql());
            }

            fn push_conditions_ref(&self, sink: &mut ConditionSink<'a, V>) {
                sink.push(self.$i.to_condition_sql());
            }
        }
    };
}

/// `ConditionList` for an N-tuple: delegate the marker fold to the
/// (N-1)-tuple and combine it with the last element, mirroring how
/// `RowColumnList` folds its column lists.
macro_rules! impl_condition_many {
    ([$($all:ident),+] [$($i:tt),+] [$($prev:ident),+] $last:ident) => {
        impl<'a, V, $($all),+> ConditionList<'a, V> for ($($all,)+)
        where
            V: SQLParam + 'a,
            $($all: Expr<'a, V>,)+
            <$last as Expr<'a, V>>::SQLType: BooleanLike,
            ($($prev,)+): ConditionList<'a, V>,
        {
            type Nullable = <<($($prev,)+) as ConditionList<'a, V>>::Nullable as Nullability>::Or<<$last as Expr<'a, V>>::Nullable>;
            type Aggregate = <<($($prev,)+) as ConditionList<'a, V>>::Aggregate as AggregateKind>::Or<<$last as Expr<'a, V>>::Aggregate>;

            fn push_conditions(self, sink: &mut ConditionSink<'a, V>) {
                $( sink.push(self.$i.into_condition_sql()); )+
            }

            fn push_conditions_ref(&self, sink: &mut ConditionSink<'a, V>) {
                $( sink.push(self.$i.to_condition_sql()); )+
            }
        }
    };
}

/// Recursive accumulator splitting the last element off the type list while
/// carrying the full type and index lists through to the impl.
macro_rules! impl_condition_split {
    ([$A:ident] [$i:tt] [] $only:ident) => {
        impl_condition_one!($A, $i);
    };
    ([$($all:ident),+] [$($i:tt),+] [$($prev:ident),+] $last:ident) => {
        impl_condition_many!([$($all),+] [$($i),+] [$($prev),+] $last);
    };
    ([$($all:ident),+] [$($i:tt),+] [] $head:ident, $($rest:ident),+) => {
        impl_condition_split!([$($all),+] [$($i),+] [$head] $($rest),+);
    };
    ([$($all:ident),+] [$($i:tt),+] [$($prev:ident),+] $head:ident, $($rest:ident),+) => {
        impl_condition_split!([$($all),+] [$($i),+] [$($prev),+, $head] $($rest),+);
    };
}

/// `Expr` for a tuple of conditions: the tuple *is* the conjunction.
///
/// The tuple's `ToSQL` impl still renders a comma-separated list — that is what
/// a SELECT or GROUP BY list needs — so the conjunction is produced by the
/// expression-rendering hooks, which every condition site goes through.
macro_rules! impl_condition_expr {
    ($($T:ident),+) => {
        impl<'a, V, $($T),+> Expr<'a, V> for ($($T,)+)
        where
            V: SQLParam + 'a,
            Self: ConditionList<'a, V> + crate::traits::ToSQL<'a, V>,
        {
            type SQLType = crate::types::Conjunction;
            type Nullable = <Self as ConditionList<'a, V>>::Nullable;
            type Aggregate = <Self as ConditionList<'a, V>>::Aggregate;

            fn to_expr_sql(&self) -> SQL<'a, V> {
                combine_ref(self, Token::AND, EMPTY_CONJUNCTION)
            }

            fn into_expr_sql(self) -> SQL<'a, V> {
                combine(self, Token::AND, EMPTY_CONJUNCTION)
            }
        }
    };
}

/// Callback for `with_col_sizes_*!`: seals the tuple and generates its
/// `ConditionList` impl.
macro_rules! impl_condition_tuple {
    ($($T:ident),+; $($i:tt),+) => {
        impl<$($T),+> sealed::Sealed for ($($T,)+) {}
        impl_condition_split!([$($T),+] [$($i),+] [] $($T),+);
    };
}

/// Callback for `with_col_sizes_8!`: makes a tuple usable as an expression.
macro_rules! impl_condition_tuple_expr {
    ($($T:ident),+; $($i:tt),+) => {
        impl_condition_expr!($($T),+);
    };
}

with_col_sizes_8!(impl_condition_tuple);

// Only the first ladder rung gets an `Expr` impl. `Expr` sits at the centre of
// the trait graph — every literal, reference, `Option`, and column competes as
// a candidate — and adding tuple candidates past arity 8 makes trait selection
// blow past any usable memory budget while rustc well-formedness-checks the
// nested marker fold. Longer lists still combine through `all`/`any`, which
// need only `ConditionList`, or by nesting tuples.
with_col_sizes_8!(impl_condition_tuple_expr);

// The ladder stops at 16 even when `col32` and above are enabled. The marker
// fold nests one projection per rung, and past 16 rungs rustc exhausts memory
// well-formedness-checking the impls. Column lists need the higher rungs
// because tables get wide; condition lists do not — `all`/`any` and nested
// tuples cover anything longer, and produce the same flat AND.
#[cfg(any(
    feature = "col16",
    feature = "col32",
    feature = "col64",
    feature = "col128",
    feature = "col200"
))]
with_col_sizes_16!(impl_condition_tuple);

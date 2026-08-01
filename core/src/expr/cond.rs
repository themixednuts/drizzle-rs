//! Condition lists: tuples as conjunctions, plus the [`all`] and [`any`] combinators.
//!
//! A tuple of conditions *is* a condition. It renders as the parenthesized AND
//! of its elements and carries exactly the guarantees a chain of
//! [`and`](super::and) calls would, so `and(a, and(b, c))` can be written
//! `(a, b, c)` anywhere a condition is accepted:
//!
//! ```rust
//! # let _ = r####"
//! use drizzle_core::expr::{all, any, eq, gt};
//!
//! // WHERE ("users"."active" = TRUE AND "users"."age" > 18 AND "users"."role" = 'admin')
//! query.r#where((eq(users.active, true), gt(users.age, 18), eq(users.role, "admin")));
//!
//! // Flat OR lists too
//! query.r#where(any((eq(users.role, "admin"), eq(users.role, "moderator"))));
//!
//! // Tuples nest inside or(), and inside each other
//! query.r#where(or((a, b), (c, d)));
//! # "####;
//! ```
//!
//! # Optional elements
//!
//! Any element may be an [`Option`]. `None` contributes nothing to the rendered
//! SQL, which makes dynamic filters composable without building the condition
//! by hand:
//!
//! ```rust
//! # let _ = r####"
//! let name_filter = name.map(|n| eq(users.name, n));
//! query.r#where((gt(users.age, 18), name_filter));
//! // Some("bob") => ("users"."age" > ? AND "users"."name" = ?)
//! // None        => ("users"."age" > ?)
//! # "####;
//! ```
//!
//! When *every* element is `None` the list has nothing to combine, so it
//! renders as the identity of its operator: `TRUE` for a conjunction (a tuple
//! or [`all`]) and `FALSE` for a disjunction ([`any`]). A conjunction that
//! filters nothing therefore behaves like an absent `WHERE` clause, while an
//! empty disjunction fails closed rather than silently matching every row.

use crate::dialect::DialectTypes;
use crate::sql::{SQL, Token};
use crate::traits::SQLParam;
use crate::types::BooleanLike;

use super::{AggOr, AggregateKind, Expr, NullOr, Nullability, SQLExpr};

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

/// Accumulator threaded through a [`ConditionList`] while it renders.
///
/// Only [`ConditionList`] implementations interact with this type, and the
/// trait is sealed, so it is an implementation detail of the crate.
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

/// A list of SQL conditions combined under a single logical operator.
///
/// Implemented for tuples whose every element is a boolean expression — or an
/// [`Option`] of one — for arities 1..=8, extended to 16 by the `col16`
/// feature. The associated markers fold across the elements
/// exactly as chained [`and`](super::and) calls would: the list is nullable if
/// any element is nullable, and aggregate if any element is aggregate.
///
/// A bare tuple additionally *is* a condition (it implements
/// [`Expr`](super::Expr)) up to arity 8. Past that, combine through [`all`] or
/// [`any`], or nest tuples — the result is the same flat AND.
///
/// This trait is sealed; the crate provides every implementation.
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not a list of SQL conditions",
    label = "expected a tuple of boolean expressions",
    note = "every element must be a boolean-typed expression, or an `Option` of one"
)]
pub trait ConditionList<'a, V: SQLParam>: sealed::Sealed {
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

/// Logical AND of every condition in a list.
///
/// The flat form of [`and`](super::and): `all((a, b, c))` renders
/// `(a AND b AND c)` instead of nesting `and(a, and(b, c))`. `None` elements
/// are skipped, and a list with no present element renders as `TRUE`.
///
/// A bare tuple already means the same thing in condition position
/// (`.r#where((a, b, c))`); reach for `all` where a tuple would be read as a
/// column list instead — most notably a join's `ON` condition, which accepts
/// any SQL fragment rather than a typed condition.
///
/// ```rust
/// # let _ = r####"
/// use drizzle_core::expr::{all, eq, gt};
///
/// all((eq(users.active, true), gt(users.age, 18)))
/// // ("users"."active" = ? AND "users"."age" > ?)
/// # "####;
/// ```
#[allow(clippy::type_complexity)]
pub fn all<'a, V, L>(
    conditions: L,
) -> SQLExpr<'a, V, <V::DialectMarker as DialectTypes>::Bool, L::Nullable, L::Aggregate>
where
    V: SQLParam + 'a,
    L: ConditionList<'a, V>,
{
    SQLExpr::new(combine(conditions, Token::AND, EMPTY_CONJUNCTION))
}

/// Logical OR of every condition in a list.
///
/// The flat form of [`or`](super::or): `any((a, b, c))` renders
/// `(a OR b OR c)` instead of nesting `or(a, or(b, c))`. `None` elements are
/// skipped, and a list with no present element renders as `FALSE` — an empty
/// disjunction matches nothing, which fails closed rather than quietly
/// dropping the filter.
///
/// ```rust
/// # let _ = r####"
/// use drizzle_core::expr::{any, eq};
///
/// any((eq(users.role, "admin"), eq(users.role, "moderator")))
/// // ("users"."role" = ? OR "users"."role" = ?)
/// # "####;
/// ```
#[allow(clippy::type_complexity)]
pub fn any<'a, V, L>(
    conditions: L,
) -> SQLExpr<'a, V, <V::DialectMarker as DialectTypes>::Bool, L::Nullable, L::Aggregate>
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
            <($($prev,)+) as ConditionList<'a, V>>::Nullable:
                NullOr<<$last as Expr<'a, V>>::Nullable>,
            <($($prev,)+) as ConditionList<'a, V>>::Aggregate:
                AggOr<<$last as Expr<'a, V>>::Aggregate>,
        {
            type Nullable = <<($($prev,)+) as ConditionList<'a, V>>::Nullable
                as NullOr<<$last as Expr<'a, V>>::Nullable>>::Output;
            type Aggregate = <<($($prev,)+) as ConditionList<'a, V>>::Aggregate
                as AggOr<<$last as Expr<'a, V>>::Aggregate>>::Output;

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

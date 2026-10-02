pub mod conflict;
pub mod insert_select;
pub mod states;

pub use conflict::*;
pub use insert_select::*;
pub use states::*;

/// Builder states whose query is complete and can be executed.
///
/// Dialect crates implement this for their builder state markers to allow
/// execution, set operations, or prepared statements in those states.
pub trait ExecutableState {}

/// The state of a query builder before any statement has been started.
#[derive(Debug, Clone)]
pub struct BuilderInit;

impl ExecutableState for BuilderInit {}

// =============================================================================
// Capability marker traits for typestate method gating
// =============================================================================
// These allow a single generic impl block per method instead of duplicating
// across every state that supports it.
//
// Used directly by the inner `SelectBuilder` impls in driver crates.
// Wrapper builders (`DrizzleBuilder`, `TransactionBuilder`) use a declarative
// macro to stamp out per-state impls instead, because Rust's inherent impl
// overlap rules prevent trait-gated generics when other builder types
// (insert/update/delete) define methods with the same name.

/// Clause markers for [`ClauseAllowed`].
///
/// Each marker names a builder method (or a use of a whole query). A builder
/// state implements `ClauseAllowed<clause::X>` when that method may be called
/// next.
pub mod clause {
    /// `.r#where()`.
    #[derive(Debug, Clone, Copy, Default)]
    pub struct Where;
    /// `.group_by()`.
    #[derive(Debug, Clone, Copy, Default)]
    pub struct GroupBy;
    /// `.having()` (requires GROUP BY).
    #[derive(Debug, Clone, Copy, Default)]
    pub struct Having;
    /// `.order_by()`.
    #[derive(Debug, Clone, Copy, Default)]
    pub struct OrderBy;
    /// `.limit()`.
    #[derive(Debug, Clone, Copy, Default)]
    pub struct Limit;
    /// `.offset()`.
    #[derive(Debug, Clone, Copy, Default)]
    pub struct Offset;
    /// `.join()` and its variants.
    #[derive(Debug, Clone, Copy, Default)]
    pub struct Join;
    /// Use as a CTE or with a locking clause: a single SELECT that is not a
    /// `UNION`/`INTERSECT`/`EXCEPT`.
    #[derive(Debug, Clone, Copy, Default)]
    pub struct Simple;
    /// Operand of `UNION`/`INTERSECT`/`EXCEPT`: a SELECT, possibly compound,
    /// without a locking clause.
    #[derive(Debug, Clone, Copy, Default)]
    pub struct Compound;
    /// Row source for a derived table or `INSERT ... SELECT`: any completed
    /// SELECT. Never an INSERT/UPDATE/DELETE, even with `RETURNING`.
    #[derive(Debug, Clone, Copy, Default)]
    pub struct Source;
}

/// The builder state `Self` allows the clause `Clause` next.
///
/// SELECT builders track their progress in a state type parameter. Clause
/// methods require `State: ClauseAllowed<clause::X>`, so calling a clause
/// out of order (`.group_by(...)` after `.limit(...)`, `.having(...)`
/// without `.group_by(...)`) is a compile error:
///
/// ```text
/// error[E0277]: builder state `SelectLimitSet` does not allow `drizzle_core::builder::clause::GroupBy`
///   = note: SELECT clauses go in order: FROM, JOIN, WHERE, GROUP BY, HAVING, ORDER BY, LIMIT, OFFSET
/// ```
///
/// The same check stops a statement that is not a SELECT (a
/// `DELETE ... RETURNING`, say) from being used as a subquery, set operand,
/// derived table, or `INSERT ... SELECT` source. Dialect crates implement
/// this for their states and may add their own clause markers for
/// dialect-only clauses. A few methods whose names clash with INSERT /
/// UPDATE / DELETE methods (`.r#where`, `.order_by`) use a dialect-local
/// trait with the same error message instead.
///
/// # Examples
///
/// ```
/// use drizzle_core::{ClauseAllowed, clause};
///
/// struct AfterFrom;
/// impl ClauseAllowed<clause::Where> for AfterFrom {}
///
/// fn where_<S: ClauseAllowed<clause::Where>>(_state: S) {}
///
/// where_(AfterFrom);
/// ```
///
/// ```compile_fail
/// # use drizzle_core::{ClauseAllowed, clause};
/// # struct AfterLimit;
/// fn where_<S: ClauseAllowed<clause::Where>>(_state: S) {}
///
/// // error: builder state `AfterLimit` does not allow `Where`
/// where_(AfterLimit);
/// ```
#[diagnostic::on_unimplemented(
    message = "builder state `{Self}` does not allow `{Clause}`",
    label = "not available at this point of the query",
    note = "SELECT clauses go in order: FROM, JOIN, WHERE, GROUP BY, HAVING, ORDER BY, LIMIT, OFFSET",
    note = "only a SELECT can be a set operand, a subquery, a derived table, or an INSERT source"
)]
pub trait ClauseAllowed<Clause> {}

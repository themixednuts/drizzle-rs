pub mod conflict;
pub mod insert_select;
pub mod states;

pub use conflict::*;
pub use insert_select::*;
pub use states::*;

/// Marker trait for executable builder states.
///
/// This is an extension point for driver crates to opt in builder state
/// markers that represent complete, executable queries (for example, to
/// enable set operations or prepared statements on those states).
pub trait ExecutableState {}

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
pub mod clause {
    /// `.where()`.
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
    /// Use as a CTE or a locking read: a single, non-compound SELECT.
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

/// The builder state `Self` accepts the clause `Clause` next.
///
/// Dialect crates add their own clause markers for dialect-only clauses.
#[diagnostic::on_unimplemented(
    message = "builder state `{Self}` does not allow `{Clause}`",
    label = "not available at this point of the query",
    note = "SELECT clauses go in order: FROM, JOIN, WHERE, GROUP BY, HAVING, ORDER BY, LIMIT, OFFSET",
    note = "only a SELECT can be a set operand, a subquery, a derived table, or an INSERT source"
)]
pub trait ClauseAllowed<Clause> {}

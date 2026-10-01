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
    /// Use as a CTE, a derived table, or a locking read: a single,
    /// non-compound SELECT.
    #[derive(Debug, Clone, Copy, Default)]
    pub struct Simple;
}

/// The builder state `Self` accepts the clause `Clause` next.
///
/// Dialect crates add their own clause markers for dialect-only clauses.
#[diagnostic::on_unimplemented(
    message = "`{Clause}` cannot be added in builder state `{Self}`",
    label = "this clause is not available at this point of the query",
    note = "SELECT clauses go in order: FROM, JOIN, WHERE, GROUP BY, HAVING, ORDER BY, LIMIT, OFFSET"
)]
pub trait ClauseAllowed<Clause> {}

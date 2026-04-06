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

/// States where `.where()` is available.
pub trait WhereAllowed {}

/// States where `.group_by()` is available.
pub trait GroupByAllowed {}

/// States where `.order_by()` is available.
pub trait OrderByAllowed {}

/// States where `.limit()` is available.
pub trait LimitAllowed {}

/// States where `.offset()` is available.
pub trait OffsetAllowed {}

/// States where `.join()` and variant joins are available.
pub trait JoinAllowed {}

/// States where `.having()` is available (requires GROUP BY).
pub trait HavingAllowed {}

/// States where GROUP BY has been applied (allows mixed agg/scalar selects).
#[diagnostic::on_unimplemented(
    message = "SELECT mixes aggregate and non-aggregate expressions without GROUP BY",
    label = "add .group_by(...) before executing this query"
)]
pub trait GroupByApplied {}

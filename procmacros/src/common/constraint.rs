//! Cross-dialect column-constraint state.
//!
//! Schema columns historically carried a pair of independent booleans
//! (`is_primary`, `is_unique`) plus a separately-threaded
//! `is_composite_pk` flag from the table-level pass. Reading them together
//! always required at least two facts, and several common patterns
//! (`is_primary && !is_composite_pk`, `is_unique && !is_primary`) encoded
//! a small state machine inline at every call site.
//!
//! [`Constraint`] folds that state machine into a single enum, set during
//! the table-level second pass once the primary-key cardinality is known.
//! Codegen sites that care about "is this *the* single-column primary key"
//! ask one question instead of three.

/// Column-level primary-key / unique state.
///
/// Set in a second pass over the field list, after the primary-key count is
/// known. Captures the four states that the legacy
/// `(is_primary, is_unique, is_composite_pk)` triple could encode without
/// requiring callers to derive them.
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[allow(dead_code)] // postgres path may not consume every variant yet
pub enum Constraint {
    /// No primary/unique constraint on this column.
    #[default]
    None,
    /// `UNIQUE` standalone constraint (not a primary key).
    Unique,
    /// The *single* primary key column. Eligible for inline `PRIMARY KEY`
    /// in the column definition.
    StandalonePrimaryKey,
    /// One of two or more columns that together form a composite primary
    /// key. The `CONSTRAINT ... PRIMARY KEY(...)` clause is emitted at the
    /// table level, not inline.
    CompositePrimaryKey,
}

impl Constraint {
    /// Build a `Constraint` from the raw inputs the parser already has
    /// (the field's own `primary`/`unique` flags) plus the table-level
    /// `is_composite_pk` decision.
    #[allow(dead_code)] // wired through SQLite first; postgres adoption later
    pub fn from_flags(is_primary: bool, is_unique: bool, is_composite_pk: bool) -> Self {
        match (is_primary, is_unique, is_composite_pk) {
            (true, _, true) => Self::CompositePrimaryKey,
            (true, _, false) => Self::StandalonePrimaryKey,
            (false, true, _) => Self::Unique,
            (false, false, _) => Self::None,
        }
    }

    /// True for both standalone and composite PK columns.
    #[allow(dead_code)]
    pub const fn is_primary(&self) -> bool {
        matches!(self, Self::StandalonePrimaryKey | Self::CompositePrimaryKey)
    }

    /// True only when this column is *the* sole primary key — the case
    /// that's eligible for inline `PRIMARY KEY` in the column definition.
    #[allow(dead_code)]
    pub const fn is_inline_primary(&self) -> bool {
        matches!(self, Self::StandalonePrimaryKey)
    }

    /// True only when this column carries a standalone `UNIQUE` (not as
    /// part of a primary key).
    #[allow(dead_code)]
    pub const fn is_inline_unique(&self) -> bool {
        matches!(self, Self::Unique)
    }
}

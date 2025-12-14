//! Dialect type re-exported from drizzle-types with core-specific extensions.

use crate::prelude::*;

/// Re-export the unified Dialect enum from drizzle-types
pub use drizzle_types::Dialect;

/// Extension trait for Dialect-specific placeholder rendering
pub trait DialectExt {
    /// Renders a placeholder for this dialect with the given 1-based index.
    ///
    /// Returns `Cow::Borrowed("?")` for SQLite/MySQL (zero allocation),
    /// `Cow::Owned` for PostgreSQL numbered placeholders.
    ///
    /// # Examples
    /// - PostgreSQL: `$1`, `$2`, `$3`
    /// - SQLite/MySQL: `?`
    fn render_placeholder(&self, index: usize) -> Cow<'static, str>;
}

impl DialectExt for Dialect {
    #[inline]
    fn render_placeholder(&self, index: usize) -> Cow<'static, str> {
        match self {
            Dialect::PostgreSQL => Cow::Owned(format!("${}", index)),
            Dialect::SQLite | Dialect::MySQL => Cow::Borrowed("?"),
        }
    }
}


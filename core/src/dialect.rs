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

    /// Appends a placeholder directly into an output buffer.
    #[inline]
    fn write_placeholder(&self, index: usize, out: &mut String) {
        match self.render_placeholder(index) {
            Cow::Borrowed(v) => out.push_str(v),
            Cow::Owned(v) => out.push_str(&v),
        }
    }
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

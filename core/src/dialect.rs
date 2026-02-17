//! Dialect type re-exported from drizzle-types with core-specific extensions.

use crate::prelude::*;

/// Re-export the unified Dialect enum from drizzle-types
pub use drizzle_types::Dialect;

// =============================================================================
// Type-level dialect markers
// =============================================================================

/// Type-level marker for SQLite.
///
/// Used by [`crate::row::SQLTypeToRust`] to provide SQLite-specific type mappings.
/// SQLite stores dates, UUIDs, and JSON as TEXT, so String fallbacks are always available.
#[derive(Debug, Clone, Copy)]
pub struct SQLiteDialect;

/// Type-level marker for PostgreSQL.
///
/// Used by [`crate::row::SQLTypeToRust`] to provide PostgreSQL-specific type mappings.
/// PostgreSQL uses native binary formats for dates, UUIDs, and JSON, so the corresponding
/// feature flags (`chrono`, `uuid`, `serde`) must be enabled.
#[derive(Debug, Clone, Copy)]
pub struct PostgresDialect;

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

/// Writes a dialect-appropriate placeholder directly to a buffer without allocation.
///
/// Unlike `render_placeholder` which returns `Cow<'static, str>` (allocating for PostgreSQL),
/// this writes directly to any `fmt::Write` implementor with zero allocation.
#[inline]
pub fn write_placeholder(dialect: Dialect, index: usize, buf: &mut impl core::fmt::Write) {
    match dialect {
        Dialect::PostgreSQL => {
            let _ = buf.write_char('$');
            let _ = write!(buf, "{}", index);
        }
        Dialect::SQLite | Dialect::MySQL => {
            let _ = buf.write_char('?');
        }
    }
}

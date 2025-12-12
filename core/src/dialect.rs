use crate::prelude::*;

/// SQL dialect for database-specific behavior
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Dialect {
    /// SQLite - uses `?` positional placeholders
    #[default]
    SQLite,
    /// PostgreSQL - uses `$1, $2, ...` numbered placeholders
    PostgreSQL,
    /// MySQL - uses `?` positional placeholders
    MySQL,
}

impl Dialect {
    /// Returns true if this dialect uses numbered placeholders ($1, $2, ...)
    #[inline]
    pub const fn uses_numbered_placeholders(&self) -> bool {
        matches!(self, Dialect::PostgreSQL)
    }

    /// Renders a placeholder for this dialect with the given 1-based index.
    ///
    /// Returns `Cow::Borrowed("?")` for SQLite/MySQL (zero allocation),
    /// `Cow::Owned` for PostgreSQL numbered placeholders.
    ///
    /// # Examples
    /// - PostgreSQL: `$1`, `$2`, `$3`
    /// - SQLite/MySQL: `?`
    #[inline]
    pub fn render_placeholder(&self, index: usize) -> Cow<'static, str> {
        match self {
            Dialect::PostgreSQL => Cow::Owned(format!("${}", index)),
            Dialect::SQLite | Dialect::MySQL => Cow::Borrowed("?"),
        }
    }
}

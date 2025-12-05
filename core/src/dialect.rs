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
}


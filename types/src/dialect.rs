//! Unified database dialect enum
//!
//! This module provides a single source of truth for database dialect identification,
//! replacing the previously duplicated definitions across `drizzle-core`, `migrations/config.rs`,
//! and `migrations/parser.rs`.

/// SQL dialect for database-specific behavior
///
/// This enum represents the supported SQL database dialects in Drizzle ORM.
/// Each dialect has different placeholder syntax, type mappings, and SQL generation rules.
///
/// # Examples
///
/// ```
/// use drizzle_types::Dialect;
///
/// let dialect = Dialect::PostgreSQL;
/// assert!(dialect.uses_numbered_placeholders());
///
/// let sqlite = Dialect::SQLite;
/// assert!(!sqlite.uses_numbered_placeholders());
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "lowercase"))]
pub enum Dialect {
    /// SQLite - uses `?` positional placeholders
    ///
    /// Compatible with: rusqlite, libsql, turso
    #[default]
    SQLite,

    /// PostgreSQL - uses `$1, $2, ...` numbered placeholders
    ///
    /// Compatible with: tokio-postgres, postgres, sqlx
    PostgreSQL,

    /// MySQL - uses `?` positional placeholders
    ///
    /// Compatible with: mysql, sqlx
    MySQL,
}

impl Dialect {
    /// Returns `true` if this dialect uses numbered placeholders (`$1, $2, ...`)
    ///
    /// Currently only PostgreSQL uses numbered placeholders.
    /// SQLite and MySQL use positional `?` placeholders.
    #[inline]
    #[must_use]
    pub const fn uses_numbered_placeholders(&self) -> bool {
        matches!(self, Dialect::PostgreSQL)
    }

    /// Parse a dialect from a string (case-insensitive)
    ///
    /// Supports various common aliases:
    /// - SQLite: `"sqlite"`, `"turso"`, `"libsql"`
    /// - PostgreSQL: `"postgresql"`, `"postgres"`, `"pg"`
    /// - MySQL: `"mysql"`
    ///
    /// # Examples
    ///
    /// ```
    /// use drizzle_types::Dialect;
    ///
    /// assert_eq!(Dialect::parse("sqlite"), Some(Dialect::SQLite));
    /// assert_eq!(Dialect::parse("postgres"), Some(Dialect::PostgreSQL));
    /// assert_eq!(Dialect::parse("pg"), Some(Dialect::PostgreSQL));
    /// assert_eq!(Dialect::parse("unknown"), None);
    /// ```
    #[must_use]
    pub fn parse(s: &str) -> Option<Self> {
        // Use eq_ignore_ascii_case for no_std compatibility (no allocation)
        if s.eq_ignore_ascii_case("sqlite")
            || s.eq_ignore_ascii_case("turso")
            || s.eq_ignore_ascii_case("libsql")
        {
            Some(Dialect::SQLite)
        } else if s.eq_ignore_ascii_case("postgresql")
            || s.eq_ignore_ascii_case("postgres")
            || s.eq_ignore_ascii_case("pg")
        {
            Some(Dialect::PostgreSQL)
        } else if s.eq_ignore_ascii_case("mysql") {
            Some(Dialect::MySQL)
        } else {
            None
        }
    }

    /// Get the table attribute prefix for this dialect in generated code
    ///
    /// Used by schema parsers and code generators.
    #[must_use]
    pub const fn table_prefix(&self) -> &'static str {
        match self {
            Dialect::SQLite => "#[SQLiteTable",
            Dialect::PostgreSQL => "#[PostgresTable",
            Dialect::MySQL => "#[MySQLTable",
        }
    }

    /// Get the index attribute prefix for this dialect in generated code
    #[must_use]
    pub const fn index_prefix(&self) -> &'static str {
        match self {
            Dialect::SQLite => "#[SQLiteIndex",
            Dialect::PostgreSQL => "#[PostgresIndex",
            Dialect::MySQL => "#[MySQLIndex",
        }
    }

    /// Get the schema derive attribute for this dialect
    #[must_use]
    pub const fn schema_derive(&self) -> &'static str {
        match self {
            Dialect::SQLite => "#[derive(SQLiteSchema)]",
            Dialect::PostgreSQL => "#[derive(PostgresSchema)]",
            Dialect::MySQL => "#[derive(MySQLSchema)]",
        }
    }

    /// Get the dialect name as a lowercase string
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Dialect::SQLite => "sqlite",
            Dialect::PostgreSQL => "postgresql",
            Dialect::MySQL => "mysql",
        }
    }
}

impl core::fmt::Display for Dialect {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl core::str::FromStr for Dialect {
    type Err = DialectParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Dialect::parse(s).ok_or(DialectParseError)
    }
}

/// Error returned when parsing an unknown dialect string
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DialectParseError;

impl core::fmt::Display for DialectParseError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("unknown dialect")
    }
}

#[cfg(feature = "std")]
impl std::error::Error for DialectParseError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dialect_parse() {
        assert_eq!(Dialect::parse("sqlite"), Some(Dialect::SQLite));
        assert_eq!(Dialect::parse("SQLite"), Some(Dialect::SQLite));
        assert_eq!(Dialect::parse("turso"), Some(Dialect::SQLite));
        assert_eq!(Dialect::parse("libsql"), Some(Dialect::SQLite));

        assert_eq!(Dialect::parse("postgresql"), Some(Dialect::PostgreSQL));
        assert_eq!(Dialect::parse("postgres"), Some(Dialect::PostgreSQL));
        assert_eq!(Dialect::parse("pg"), Some(Dialect::PostgreSQL));
        assert_eq!(Dialect::parse("PG"), Some(Dialect::PostgreSQL));

        assert_eq!(Dialect::parse("mysql"), Some(Dialect::MySQL));
        assert_eq!(Dialect::parse("MySQL"), Some(Dialect::MySQL));

        assert_eq!(Dialect::parse("unknown"), None);
        assert_eq!(Dialect::parse(""), None);
    }

    #[test]
    fn test_dialect_placeholders() {
        assert!(!Dialect::SQLite.uses_numbered_placeholders());
        assert!(Dialect::PostgreSQL.uses_numbered_placeholders());
        assert!(!Dialect::MySQL.uses_numbered_placeholders());
    }

    #[test]
    fn test_dialect_display() {
        assert_eq!(format!("{}", Dialect::SQLite), "sqlite");
        assert_eq!(format!("{}", Dialect::PostgreSQL), "postgresql");
        assert_eq!(format!("{}", Dialect::MySQL), "mysql");
    }
}

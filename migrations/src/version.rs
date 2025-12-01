//! Version constants for drizzle migrations
//!
//! These match the versions used by drizzle-kit for compatibility.
//! All version constants are centralized here for maintainability.

use crate::config::Dialect;

/// The origin UUID used for the first migration's prev_id
pub const ORIGIN_UUID: &str = "00000000-0000-0000-0000-000000000000";

/// Journal version - used in _journal.json
/// This matches drizzle-kit's snapshotVersion
pub const JOURNAL_VERSION: &str = "7";

/// SQLite/Turso/LibSQL snapshot version (current)
pub const SQLITE_SNAPSHOT_VERSION: &str = "6";

/// PostgreSQL snapshot version (current)
pub const POSTGRES_SNAPSHOT_VERSION: &str = "7";

/// MySQL snapshot version (current)
pub const MYSQL_SNAPSHOT_VERSION: &str = "5";

/// SingleStore snapshot version (current)
pub const SINGLESTORE_SNAPSHOT_VERSION: &str = "1";

/// Minimum supported versions for backwards compatibility
/// (matches drizzle-kit's backwardCompatible* schemas)
pub const SQLITE_MIN_SUPPORTED_VERSION: u32 = 5;
pub const POSTGRES_MIN_SUPPORTED_VERSION: u32 = 5;
pub const MYSQL_MIN_SUPPORTED_VERSION: u32 = 5;

/// Get the current snapshot version for a dialect
pub fn snapshot_version(dialect: Dialect) -> &'static str {
    match dialect {
        Dialect::Sqlite => SQLITE_SNAPSHOT_VERSION,
        Dialect::Postgresql => POSTGRES_SNAPSHOT_VERSION,
        Dialect::Mysql => MYSQL_SNAPSHOT_VERSION,
    }
}

/// Check if a snapshot version is the latest for a given dialect
pub fn is_latest_version(dialect: Dialect, version: &str) -> bool {
    version == snapshot_version(dialect)
}

/// Check if a snapshot version is supported for a given dialect.
/// Older versions below the minimum need to be upgraded with `drizzle up`.
///
/// Supported version ranges (matching drizzle-kit):
/// - SQLite: 5-6 (v4 and below need upgrade)
/// - PostgreSQL: 5-7 (v4 and below need upgrade)
/// - MySQL: 5 (no older versions)
pub fn is_supported_version(dialect: Dialect, version: &str) -> bool {
    let Ok(v) = version.parse::<u32>() else {
        return false;
    };

    let (min, max) = match dialect {
        Dialect::Sqlite => (SQLITE_MIN_SUPPORTED_VERSION, 6),
        Dialect::Postgresql => (POSTGRES_MIN_SUPPORTED_VERSION, 7),
        Dialect::Mysql => (MYSQL_MIN_SUPPORTED_VERSION, 5),
    };

    v >= min && v <= max
}

/// Check if a version needs to be upgraded to work with the current CLI
pub fn needs_upgrade(dialect: Dialect, version: &str) -> bool {
    let Ok(v) = version.parse::<u32>() else {
        return true; // Unknown version, probably needs upgrade
    };

    let min = match dialect {
        Dialect::Sqlite => SQLITE_MIN_SUPPORTED_VERSION,
        Dialect::Postgresql => POSTGRES_MIN_SUPPORTED_VERSION,
        Dialect::Mysql => MYSQL_MIN_SUPPORTED_VERSION,
    };

    v < min
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_snapshot_versions() {
        assert_eq!(snapshot_version(Dialect::Sqlite), "6");
        assert_eq!(snapshot_version(Dialect::Postgresql), "7");
        assert_eq!(snapshot_version(Dialect::Mysql), "5");
    }

    #[test]
    fn test_is_latest_version() {
        assert!(is_latest_version(Dialect::Sqlite, "6"));
        assert!(!is_latest_version(Dialect::Sqlite, "5"));
        assert!(is_latest_version(Dialect::Postgresql, "7"));
        assert!(!is_latest_version(Dialect::Postgresql, "6"));
    }

    #[test]
    fn test_is_supported_version() {
        // SQLite supports v5-6
        assert!(is_supported_version(Dialect::Sqlite, "6"));
        assert!(is_supported_version(Dialect::Sqlite, "5"));
        assert!(!is_supported_version(Dialect::Sqlite, "4")); // Too old
        assert!(!is_supported_version(Dialect::Sqlite, "7")); // Too new

        // PostgreSQL supports v5-7
        assert!(is_supported_version(Dialect::Postgresql, "7"));
        assert!(is_supported_version(Dialect::Postgresql, "6"));
        assert!(is_supported_version(Dialect::Postgresql, "5"));
        assert!(!is_supported_version(Dialect::Postgresql, "4")); // Too old
        assert!(!is_supported_version(Dialect::Postgresql, "8")); // Too new
    }

    #[test]
    fn test_needs_upgrade() {
        // SQLite v4 and below need upgrade
        assert!(needs_upgrade(Dialect::Sqlite, "4"));
        assert!(needs_upgrade(Dialect::Sqlite, "3"));
        assert!(!needs_upgrade(Dialect::Sqlite, "5"));
        assert!(!needs_upgrade(Dialect::Sqlite, "6"));

        // PostgreSQL v4 and below need upgrade
        assert!(needs_upgrade(Dialect::Postgresql, "4"));
        assert!(!needs_upgrade(Dialect::Postgresql, "5"));
    }
}

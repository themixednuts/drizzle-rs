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

/// SQLite/Turso/LibSQL snapshot version
pub const SQLITE_SNAPSHOT_VERSION: &str = "6";

/// PostgreSQL snapshot version
pub const POSTGRES_SNAPSHOT_VERSION: &str = "7";

/// MySQL snapshot version
pub const MYSQL_SNAPSHOT_VERSION: &str = "5";

/// SingleStore snapshot version
pub const SINGLESTORE_SNAPSHOT_VERSION: &str = "1";

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

/// Check if a snapshot version is supported (not newer than current)
pub fn is_supported_version(dialect: Dialect, version: &str) -> bool {
    let current = snapshot_version(dialect);
    match version.parse::<u32>() {
        Ok(v) => {
            let current_v = current.parse::<u32>().unwrap_or(0);
            v <= current_v
        }
        Err(_) => false,
    }
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
        // SQLite supports v1-6
        assert!(is_supported_version(Dialect::Sqlite, "6"));
        assert!(is_supported_version(Dialect::Sqlite, "5"));
        assert!(is_supported_version(Dialect::Sqlite, "1"));
        assert!(!is_supported_version(Dialect::Sqlite, "7")); // Too new

        // PostgreSQL supports v1-7
        assert!(is_supported_version(Dialect::Postgresql, "7"));
        assert!(is_supported_version(Dialect::Postgresql, "6"));
        assert!(!is_supported_version(Dialect::Postgresql, "8")); // Too new
    }
}

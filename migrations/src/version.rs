//! Snapshot and journal format versions, matching drizzle-kit.

use drizzle_types::Dialect;

/// The `prev_id` of the first snapshot in a chain (all zeros).
pub const ORIGIN_UUID: &str = "00000000-0000-0000-0000-000000000000";

/// Version written to legacy `_journal.json` files (drizzle-kit's
/// `snapshotVersion`).
pub const JOURNAL_VERSION: &str = "7";

/// Current SQLite (also Turso and libSQL) snapshot version.
pub const SQLITE_SNAPSHOT_VERSION: &str = "7";

/// Current PostgreSQL snapshot version.
pub const POSTGRES_SNAPSHOT_VERSION: &str = "8";

/// Current MySQL snapshot version.
pub const MYSQL_SNAPSHOT_VERSION: &str = "6";

/// Current SingleStore snapshot version (drizzle-kit compatibility only).
pub const SINGLESTORE_SNAPSHOT_VERSION: &str = "1";

/// Oldest SQLite snapshot version that loads without `drizzle up`
/// (matches drizzle-kit's `backwardCompatible*` schemas).
pub const SQLITE_MIN_SUPPORTED_VERSION: u32 = 5;
/// Oldest PostgreSQL snapshot version that loads without `drizzle up`.
pub const POSTGRES_MIN_SUPPORTED_VERSION: u32 = 5;
/// Oldest MySQL snapshot version that loads without `drizzle up`.
pub const MYSQL_MIN_SUPPORTED_VERSION: u32 = 5;

/// Returns the current snapshot version for `dialect`.
#[must_use]
pub const fn snapshot_version(dialect: Dialect) -> &'static str {
    match dialect {
        Dialect::SQLite => SQLITE_SNAPSHOT_VERSION,
        Dialect::PostgreSQL => POSTGRES_SNAPSHOT_VERSION,
        Dialect::MySQL => MYSQL_SNAPSHOT_VERSION,
    }
}

/// Returns `true` if `version` is the current snapshot version for `dialect`.
#[must_use]
pub const fn is_latest_version(dialect: Dialect, version: &str) -> bool {
    // Use const-compatible byte comparison since str::eq is not const
    let a = version.as_bytes();
    let b = snapshot_version(dialect).as_bytes();
    if a.len() != b.len() {
        return false;
    }
    let mut i = 0;
    while i < a.len() {
        if a[i] != b[i] {
            return false;
        }
        i += 1;
    }
    true
}

/// Returns `true` if `version` is within the supported range for `dialect`.
///
/// Versions below the minimum need `drizzle up`. Supported ranges (matching
/// drizzle-kit beta):
/// - `SQLite`: 5-7 (v4 and below need upgrade)
/// - `PostgreSQL`: 5-8 (v4 and below need upgrade)
/// - `MySQL`: 5-6
#[must_use]
pub fn is_supported_version(dialect: Dialect, version: &str) -> bool {
    let Ok(v) = version.parse::<u32>() else {
        return false;
    };

    let (min, max) = match dialect {
        Dialect::SQLite => (SQLITE_MIN_SUPPORTED_VERSION, 7),
        Dialect::PostgreSQL => (POSTGRES_MIN_SUPPORTED_VERSION, 8),
        Dialect::MySQL => (MYSQL_MIN_SUPPORTED_VERSION, 6),
    };

    v >= min && v <= max
}

/// Returns `true` if `version` is below the supported minimum or is not a
/// number, so it must go through `drizzle up` first.
///
/// # Examples
///
/// ```rust
/// use drizzle_migrations::needs_upgrade;
/// use drizzle_types::Dialect;
///
/// assert!(needs_upgrade(Dialect::SQLite, "4"));
/// assert!(!needs_upgrade(Dialect::SQLite, "7"));
/// ```
#[must_use]
pub fn needs_upgrade(dialect: Dialect, version: &str) -> bool {
    let Ok(v) = version.parse::<u32>() else {
        return true; // Unknown version, probably needs upgrade
    };

    let min = match dialect {
        Dialect::SQLite => SQLITE_MIN_SUPPORTED_VERSION,
        Dialect::PostgreSQL => POSTGRES_MIN_SUPPORTED_VERSION,
        Dialect::MySQL => MYSQL_MIN_SUPPORTED_VERSION,
    };

    v < min
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_snapshot_versions() {
        assert_eq!(snapshot_version(Dialect::SQLite), "7");
        assert_eq!(snapshot_version(Dialect::PostgreSQL), "8");
        assert_eq!(snapshot_version(Dialect::MySQL), "6");
    }

    #[test]
    fn test_is_latest_version() {
        assert!(is_latest_version(Dialect::SQLite, "7"));
        assert!(!is_latest_version(Dialect::SQLite, "6"));
        assert!(is_latest_version(Dialect::PostgreSQL, "8"));
        assert!(!is_latest_version(Dialect::PostgreSQL, "7"));
        assert!(is_latest_version(Dialect::MySQL, "6"));
        assert!(!is_latest_version(Dialect::MySQL, "5"));
    }

    #[test]
    fn test_is_supported_version() {
        // SQLite supports v5-7
        assert!(is_supported_version(Dialect::SQLite, "7"));
        assert!(is_supported_version(Dialect::SQLite, "6"));
        assert!(is_supported_version(Dialect::SQLite, "5"));
        assert!(!is_supported_version(Dialect::SQLite, "4")); // Too old
        assert!(!is_supported_version(Dialect::SQLite, "8")); // Too new

        // PostgreSQL supports v5-8
        assert!(is_supported_version(Dialect::PostgreSQL, "8"));
        assert!(is_supported_version(Dialect::PostgreSQL, "7"));
        assert!(is_supported_version(Dialect::PostgreSQL, "6"));
        assert!(is_supported_version(Dialect::PostgreSQL, "5"));
        assert!(!is_supported_version(Dialect::PostgreSQL, "4")); // Too old
        assert!(!is_supported_version(Dialect::PostgreSQL, "9")); // Too new

        // MySQL supports legacy v5 input and current v6 snapshots.
        assert!(is_supported_version(Dialect::MySQL, "6"));
        assert!(is_supported_version(Dialect::MySQL, "5"));
        assert!(!is_supported_version(Dialect::MySQL, "4"));
        assert!(!is_supported_version(Dialect::MySQL, "7"));
    }

    #[test]
    fn test_needs_upgrade() {
        // SQLite v4 and below need upgrade
        assert!(needs_upgrade(Dialect::SQLite, "4"));
        assert!(needs_upgrade(Dialect::SQLite, "3"));
        assert!(!needs_upgrade(Dialect::SQLite, "5"));
        assert!(!needs_upgrade(Dialect::SQLite, "6"));
        assert!(!needs_upgrade(Dialect::SQLite, "7"));

        // PostgreSQL v4 and below need upgrade
        assert!(needs_upgrade(Dialect::PostgreSQL, "4"));
        assert!(!needs_upgrade(Dialect::PostgreSQL, "5"));

        // Supported v5 can be consumed by the structural upgrade machinery;
        // only snapshots too old to understand require an older CLI first.
        assert!(needs_upgrade(Dialect::MySQL, "4"));
        assert!(!needs_upgrade(Dialect::MySQL, "5"));
        assert!(!needs_upgrade(Dialect::MySQL, "6"));
    }
}

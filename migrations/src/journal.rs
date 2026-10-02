//! The legacy drizzle-kit migration journal (`meta/_journal.json`).
//!
//! Older drizzle-kit folders list every *generated* migration here, in order.
//! The current folder layout (one subfolder per migration) has no journal;
//! `drizzle up` converts old folders. It does not record which migrations
//! were applied to a database: that lives in the tracking table.

use crate::version::{JOURNAL_VERSION, snapshot_version};
use drizzle_types::Dialect;
use serde::{Deserialize, Serialize};

/// A parsed `_journal.json`: format version, dialect, and entries.
///
/// # Examples
///
/// ```rust
/// use drizzle_migrations::Journal;
/// use drizzle_types::Dialect;
///
/// let mut journal = Journal::new(Dialect::SQLite);
/// journal.add_entry("0000_init".to_string(), true);
///
/// let parsed = Journal::from_json(&journal.to_json()?)?;
/// assert_eq!(parsed.entries[0].idx, 0);
/// assert_eq!(parsed.entries[0].tag, "0000_init");
/// # Ok::<(), drizzle_migrations::serde_json::Error>(())
/// ```
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Journal {
    /// Journal format version
    pub version: String,
    /// Database dialect
    pub dialect: Dialect,
    /// List of migration entries
    pub entries: Vec<JournalEntry>,
}

/// One migration listed in the journal.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct JournalEntry {
    /// Migration index (0-based)
    pub idx: u32,
    /// Schema version used for this migration
    pub version: String,
    /// Unix timestamp in milliseconds when migration was created
    pub when: u64,
    /// Migration tag (folder/file name), e.g. `0000_initial_migration`.
    pub tag: String,
    /// Whether SQL statement breakpoints are enabled
    pub breakpoints: bool,
}

impl Journal {
    /// Creates an empty journal for `dialect`.
    #[must_use]
    pub fn new(dialect: Dialect) -> Self {
        Self {
            version: JOURNAL_VERSION.to_string(),
            dialect,
            entries: Vec::new(),
        }
    }

    /// Returns the index the next entry will get.
    #[must_use]
    pub fn next_idx(&self) -> u32 {
        u32::try_from(self.entries.len()).unwrap_or(u32::MAX)
    }

    /// Appends an entry stamped with the next index and the current time.
    pub fn add_entry(&mut self, tag: String, breakpoints: bool) -> &JournalEntry {
        self.entries.push(JournalEntry {
            idx: self.next_idx(),
            version: snapshot_version(self.dialect).to_string(),
            when: current_timestamp_ms(),
            tag,
            breakpoints,
        });
        let last_idx = self.entries.len() - 1;
        &self.entries[last_idx]
    }

    /// Parses a journal from JSON.
    ///
    /// # Errors
    ///
    /// Returns an error if the string is not valid journal JSON.
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }

    /// Serializes the journal to pretty-printed JSON.
    ///
    /// # Errors
    ///
    /// Returns an error if serialization fails (e.g., non-serializable data).
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Reads a journal from `path`.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read or its contents are not
    /// valid journal JSON.
    pub fn load(path: &std::path::Path) -> std::io::Result<Self> {
        let contents = std::fs::read_to_string(path)?;
        serde_json::from_str(&contents)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
    }

    /// Reads a journal from `path`, or returns an empty one if the file does
    /// not exist.
    ///
    /// # Errors
    ///
    /// Returns an error if the file exists but cannot be read or parsed as
    /// valid journal JSON.
    pub fn load_or_create(path: &std::path::Path, dialect: Dialect) -> std::io::Result<Self> {
        if path.exists() {
            Self::load(path)
        } else {
            Ok(Self::new(dialect))
        }
    }

    /// Writes the journal to `path`, creating parent folders.
    ///
    /// # Errors
    ///
    /// Returns an error if serialization fails, the parent directory cannot
    /// be created, or the file cannot be written.
    pub fn save(&self, path: &std::path::Path) -> std::io::Result<()> {
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // Write-then-rename so a crash mid-write never corrupts the journal.
        let tmp = path.with_extension("json.tmp");
        std::fs::write(&tmp, json)?;
        std::fs::rename(&tmp, path).inspect_err(|_| {
            let _ = std::fs::remove_file(&tmp);
        })
    }
}

/// Get current timestamp in milliseconds
fn current_timestamp_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_sqlite_journal() {
        let journal = Journal::new(Dialect::SQLite);
        assert_eq!(journal.version, "7");
        assert_eq!(journal.dialect, Dialect::SQLite);
        assert!(journal.entries.is_empty());
    }

    #[test]
    fn test_add_entry() {
        let mut journal = Journal::new(Dialect::SQLite);
        journal.add_entry("0000_initial".to_string(), true);

        assert_eq!(journal.entries.len(), 1);
        assert_eq!(journal.entries[0].idx, 0);
        assert_eq!(journal.entries[0].tag, "0000_initial");
        assert!(journal.entries[0].breakpoints);
    }

    #[test]
    fn test_journal_serialization() {
        let mut journal = Journal::new(Dialect::SQLite);
        journal.add_entry("0000_test".to_string(), true);

        let json = journal.to_json().unwrap();
        let parsed = Journal::from_json(&json).unwrap();

        assert_eq!(parsed.version, journal.version);
        assert_eq!(parsed.dialect, journal.dialect);
        assert_eq!(parsed.entries.len(), 1);
    }
}

//! Journal types for tracking migrations
//!
//! The journal (_journal.json) tracks all applied migrations in order.

use crate::version::{JOURNAL_VERSION, snapshot_version};
use drizzle_types::Dialect;
use serde::{Deserialize, Serialize};

/// Migration journal - tracks all migrations
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Journal {
    /// Journal format version
    pub version: String,
    /// Database dialect
    pub dialect: Dialect,
    /// List of migration entries
    pub entries: Vec<JournalEntry>,
}

/// A single migration entry in the journal
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct JournalEntry {
    /// Migration index (0-based)
    pub idx: u32,
    /// Schema version used for this migration
    pub version: String,
    /// Unix timestamp in milliseconds when migration was created
    pub when: u64,
    /// Migration tag/name (e.g., "`0000_initial_migration`")
    pub tag: String,
    /// Whether SQL statement breakpoints are enabled
    pub breakpoints: bool,
}

impl Journal {
    /// Create a new journal for the given dialect
    #[must_use]
    pub fn new(dialect: Dialect) -> Self {
        Self {
            version: JOURNAL_VERSION.to_string(),
            dialect,
            entries: Vec::new(),
        }
    }

    /// Get the next migration index
    #[must_use]
    pub fn next_idx(&self) -> u32 {
        u32::try_from(self.entries.len()).unwrap_or(u32::MAX)
    }

    /// Add a new entry to the journal
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

    /// Load journal from a JSON string
    ///
    /// # Errors
    ///
    /// Returns an error if the string is not valid journal JSON.
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }

    /// Serialize journal to JSON string
    ///
    /// # Errors
    ///
    /// Returns an error if serialization fails (e.g., non-serializable data).
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Load journal from file
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

    /// Load journal from file, or create new if doesn't exist
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

    /// Save journal to file
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

        std::fs::write(path, json)
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

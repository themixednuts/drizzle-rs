//! Check command - verify migration integrity
//!
//! This command validates that:
//! 1. All migrations in the journal have corresponding SQL files
//! 2. All migrations have corresponding snapshots
//! 3. Snapshots are valid JSON with correct schema
//! 4. Snapshot versions are up-to-date
//! 5. No snapshot collisions (multiple snapshots pointing to same parent)

use crate::error::CliError;
use colored::Colorize;
use drizzle_migrations::{is_supported_version, snapshot_version, Dialect, Journal};
use std::collections::HashMap;
use std::path::Path;

pub struct CheckOptions {
    pub out_dir: String,
    pub dialect: Dialect,
}

#[derive(Default)]
struct CheckReport {
    /// Migrations with missing SQL files
    missing_sql: Vec<String>,
    /// Migrations with missing snapshots
    missing_snapshots: Vec<String>,
    /// Snapshots that failed to parse
    malformed: Vec<(String, String)>,
    /// Snapshots with non-latest versions
    non_latest: Vec<(String, String, String)>, // (file, found_version, expected_version)
    /// Snapshots with same prevId (collision)
    collisions: Vec<(String, Vec<String>)>, // (parent_id, [snapshot files])
    /// Empty SQL files
    empty_sql: Vec<String>,
}

impl CheckReport {
    fn has_errors(&self) -> bool {
        !self.missing_sql.is_empty()
            || !self.missing_snapshots.is_empty()
            || !self.malformed.is_empty()
            || !self.collisions.is_empty()
    }

    fn has_warnings(&self) -> bool {
        !self.non_latest.is_empty() || !self.empty_sql.is_empty()
    }
}

pub fn run(opts: CheckOptions) -> anyhow::Result<()> {
    let migrations_dir = Path::new(&opts.out_dir).join("migrations");
    let meta_dir = migrations_dir.join("meta");

    if !migrations_dir.exists() {
        println!(
            "{} No migrations directory found at {}",
            "!".yellow(),
            migrations_dir.display()
        );
        return Ok(());
    }

    // Load journal
    let journal_path = meta_dir.join("_journal.json");
    if !journal_path.exists() {
        return Err(CliError::JournalNotFound(journal_path.display().to_string()).into());
    }

    let journal_content = std::fs::read_to_string(&journal_path)?;
    let journal: Journal = serde_json::from_str(&journal_content)?;

    if journal.entries.is_empty() {
        println!("{} No migrations found", "!".yellow());
        return Ok(());
    }

    println!("Checking {} migrations...\n", journal.entries.len());

    let mut report = CheckReport::default();
    let mut prev_id_map: HashMap<String, Vec<String>> = HashMap::new();

    let expected_version = snapshot_version(opts.dialect);

    for entry in &journal.entries {
        let sql_path = migrations_dir.join(format!("{}.sql", entry.tag));
        let snapshot_path = meta_dir.join(format!("{:04}_snapshot.json", entry.idx));
        let mut entry_valid = true;

        // Check SQL file exists
        if !sql_path.exists() {
            report.missing_sql.push(entry.tag.clone());
            entry_valid = false;
        } else {
            // Check SQL file is not empty
            let sql_content = std::fs::read_to_string(&sql_path)?;
            if sql_content.trim().is_empty() {
                report.empty_sql.push(entry.tag.clone());
            }
        }

        // Check snapshot exists
        if !snapshot_path.exists() {
            report.missing_snapshots.push(entry.tag.clone());
            entry_valid = false;
        } else {
            // Validate snapshot JSON
            let snapshot_content = std::fs::read_to_string(&snapshot_path)?;
            match validate_snapshot(&snapshot_content, opts.dialect) {
                Ok(snapshot_info) => {
                    // Check version
                    if snapshot_info.version != expected_version {
                        report.non_latest.push((
                            entry.tag.clone(),
                            snapshot_info.version.clone(),
                            expected_version.to_string(),
                        ));
                    }

                    // Track prev_id for collision detection
                    prev_id_map
                        .entry(snapshot_info.prev_id)
                        .or_default()
                        .push(entry.tag.clone());
                }
                Err(e) => {
                    report
                        .malformed
                        .push((snapshot_path.display().to_string(), e));
                    entry_valid = false;
                }
            }
        }

        // Print progress
        if entry_valid {
            println!("  {} {}", "✓".green(), entry.tag);
        } else {
            println!("  {} {}", "✗".red(), entry.tag);
        }
    }

    // Check for collisions (multiple snapshots with same prev_id)
    for (prev_id, tags) in prev_id_map {
        if tags.len() > 1 {
            report.collisions.push((prev_id, tags));
        }
    }

    println!();

    // Print warnings
    if report.has_warnings() {
        println!("{}", "Warnings:".yellow().bold());

        for file in &report.empty_sql {
            println!("  {} Empty SQL file: {}", "!".yellow(), file);
        }

        for (file, found, expected) in &report.non_latest {
            println!(
                "  {} {} is version {} (expected {}), run 'drizzle up' to upgrade",
                "!".yellow(),
                file,
                found,
                expected
            );
        }
        println!();
    }

    // Print errors
    if report.has_errors() {
        println!("{}", "Errors:".red().bold());

        for file in &report.missing_sql {
            println!("  {} Missing SQL file: {}.sql", "✗".red(), file);
        }

        for file in &report.missing_snapshots {
            println!("  {} Missing snapshot for: {}", "✗".red(), file);
        }

        for (file, err) in &report.malformed {
            println!("  {} Malformed snapshot {}: {}", "✗".red(), file, err);
        }

        for (parent, snapshots) in &report.collisions {
            println!(
                "  {} Collision: [{}] all point to parent {}",
                "✗".red(),
                snapshots.join(", "),
                parent
            );
        }

        anyhow::bail!(
            "Found {} error(s)",
            report.missing_sql.len()
                + report.missing_snapshots.len()
                + report.malformed.len()
                + report.collisions.len()
        );
    }

    println!("{}", "Everything's fine 🐶🔥".green());

    Ok(())
}

struct SnapshotInfo {
    version: String,
    prev_id: String,
}

fn validate_snapshot(content: &str, dialect: Dialect) -> Result<SnapshotInfo, String> {
    // First parse as generic JSON to extract version and prevId
    let raw: serde_json::Value =
        serde_json::from_str(content).map_err(|e| format!("Invalid JSON: {}", e))?;

    let version = raw
        .get("version")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
        .to_string();

    let prev_id = raw
        .get("prevId")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
        .to_string();

    // Check if version is supported (not newer than what we support)
    if !is_supported_version(dialect, &version) {
        return Err(format!(
            "Unsupported version {} - please update drizzle-cli",
            version
        ));
    }

    // Now validate the full schema
    match dialect {
        Dialect::Sqlite => {
            serde_json::from_str::<drizzle_migrations::sqlite::SQLiteSnapshot>(content)
                .map(|_| SnapshotInfo { version, prev_id })
                .map_err(|e| format!("Schema validation failed: {}", e))
        }
        Dialect::Postgresql => {
            serde_json::from_str::<drizzle_migrations::postgres::PostgresSnapshot>(content)
                .map(|_| SnapshotInfo { version, prev_id })
                .map_err(|e| format!("Schema validation failed: {}", e))
        }
        Dialect::Mysql => {
            // MySQL schema validation not yet implemented
            Ok(SnapshotInfo { version, prev_id })
        }
    }
}

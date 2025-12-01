//! Check command - verify migration integrity
//!
//! This command validates that:
//! 1. All migrations in the journal have corresponding SQL files
//! 2. All migrations have corresponding snapshots
//! 3. Snapshots are valid JSON

use crate::error::CliError;
use colored::Colorize;
use drizzle_migrations::Journal;
use std::path::Path;

pub struct CheckOptions {
    pub out_dir: String,
    pub dialect: String,
}

pub fn run(opts: CheckOptions) -> anyhow::Result<()> {
    let migrations_dir = Path::new(&opts.out_dir).join("migrations");
    let meta_dir = migrations_dir.join("meta");

    if !migrations_dir.exists() {
        println!("{} No migrations directory found at {}", "!".yellow(), migrations_dir.display());
        return Ok(());
    }

    // Load journal
    let journal_path = meta_dir.join("_journal.json");
    if !journal_path.exists() {
        return Err(CliError::JournalNotFound(journal_path.display().to_string()).into());
    }

    let journal_content = std::fs::read_to_string(&journal_path)?;
    let journal: Journal = serde_json::from_str(&journal_content)?;

    println!("Checking {} migrations...\n", journal.entries.len());

    let mut errors = Vec::new();
    let mut warnings = Vec::new();

    for entry in &journal.entries {
        let sql_path = migrations_dir.join(format!("{}.sql", entry.tag));
        let snapshot_path = meta_dir.join(format!("{:04}_snapshot.json", entry.idx));

        // Check SQL file exists
        if !sql_path.exists() {
            errors.push(format!("Missing SQL file: {}", sql_path.display()));
        } else {
            // Check SQL file is not empty
            let sql_content = std::fs::read_to_string(&sql_path)?;
            if sql_content.trim().is_empty() {
                warnings.push(format!("Empty SQL file: {}", sql_path.display()));
            }
        }

        // Check snapshot exists
        if !snapshot_path.exists() {
            errors.push(format!("Missing snapshot: {}", snapshot_path.display()));
        } else {
            // Validate snapshot JSON
            let snapshot_content = std::fs::read_to_string(&snapshot_path)?;
            if let Err(e) = validate_snapshot(&snapshot_content, &opts.dialect) {
                errors.push(format!("Invalid snapshot {}: {}", snapshot_path.display(), e));
            }
        }

        // Print progress
        if sql_path.exists() && snapshot_path.exists() {
            println!("  {} {}", "✓".green(), entry.tag);
        } else {
            println!("  {} {}", "✗".red(), entry.tag);
        }
    }

    println!();

    // Print warnings
    if !warnings.is_empty() {
        println!("{}", "Warnings:".yellow().bold());
        for warning in &warnings {
            println!("  {} {}", "!".yellow(), warning);
        }
        println!();
    }

    // Print errors
    if !errors.is_empty() {
        println!("{}", "Errors:".red().bold());
        for error in &errors {
            println!("  {} {}", "✗".red(), error);
        }
        anyhow::bail!("Found {} error(s)", errors.len());
    }

    println!("{}", "Everything's fine 🐶🔥".green());

    Ok(())
}

fn validate_snapshot(content: &str, dialect: &str) -> Result<(), String> {
    match dialect {
        "sqlite" | "turso" | "libsql" => {
            serde_json::from_str::<drizzle_migrations::sqlite::SQLiteSnapshot>(content)
                .map(|_| ())
                .map_err(|e| e.to_string())
        }
        "postgresql" | "postgres" => {
            serde_json::from_str::<drizzle_migrations::postgres::PostgresSnapshot>(content)
                .map(|_| ())
                .map_err(|e| e.to_string())
        }
        _ => Err(format!("Unknown dialect: {}", dialect)),
    }
}


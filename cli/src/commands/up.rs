//! Up command - upgrade snapshots to latest version
//!
//! This command upgrades all snapshots to the latest schema version.
//! This is needed when the snapshot format changes between drizzle versions.
//!
//! The upgrade process transforms the data structure, not just the version number:
//! - SQLite v5 → v6: JSON defaults become escaped strings, views field added
//! - PostgreSQL v5 → v6: Table keys become schema.name, enum format changes
//! - PostgreSQL v6 → v7: Index format changes, policies/sequences/roles added

use crate::error::CliError;
use colored::Colorize;
use drizzle_migrations::{
    Dialect, Journal, needs_upgrade, snapshot_version, upgrade::upgrade_to_latest,
};
use std::path::Path;

pub struct UpOptions {
    pub out_dir: String,
    pub dialect: Dialect,
}

pub fn run(opts: UpOptions) -> anyhow::Result<()> {
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
        println!("{} No migrations to upgrade", "!".yellow());
        return Ok(());
    }

    let expected_version = snapshot_version(opts.dialect);

    println!("Upgrading snapshots to version {}...\n", expected_version);

    let mut upgraded = 0;
    let mut already_current = 0;
    let mut errors = 0;

    for entry in &journal.entries {
        let snapshot_path = meta_dir.join(format!("{:04}_snapshot.json", entry.idx));

        if !snapshot_path.exists() {
            println!("  {} Missing snapshot for {}", "✗".red(), entry.tag);
            errors += 1;
            continue;
        }

        let content = std::fs::read_to_string(&snapshot_path)?;

        // Parse as generic JSON
        let raw: serde_json::Value = serde_json::from_str(&content)?;

        let current_version = raw
            .get("version")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();

        if current_version == expected_version {
            println!(
                "  {} {} (already v{})",
                "✓".green(),
                entry.tag,
                expected_version
            );
            already_current += 1;
            continue;
        }

        // Check if this version is too old and needs upgrade
        if !needs_upgrade(opts.dialect, &current_version) && current_version != expected_version {
            // Version is supported but not latest - still upgrade
            println!(
                "  {} {} - v{} is supported but upgrading to v{}",
                "↑".cyan(),
                entry.tag,
                current_version,
                expected_version
            );
        }

        // Perform the actual upgrade with data transformation
        let upgraded_json = upgrade_to_latest(raw, opts.dialect);

        // Verify the upgrade worked
        let new_version = upgraded_json
            .get("version")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");

        if new_version != expected_version {
            println!(
                "  {} {} - upgrade failed (got v{}, expected v{})",
                "✗".red(),
                entry.tag,
                new_version,
                expected_version
            );
            errors += 1;
            continue;
        }

        // Write back
        let upgraded_content = serde_json::to_string_pretty(&upgraded_json)?;
        std::fs::write(&snapshot_path, upgraded_content)?;

        println!(
            "  {} {} (v{} → v{})",
            "↑".cyan(),
            entry.tag,
            current_version,
            expected_version
        );
        upgraded += 1;
    }

    println!();

    if errors > 0 {
        anyhow::bail!("{} errors occurred during upgrade", errors);
    }

    if upgraded > 0 {
        println!(
            "{} Upgraded {} snapshot(s), {} already current",
            "✓".green().bold(),
            upgraded,
            already_current
        );
    } else {
        println!(
            "{} All {} snapshot(s) already at latest version",
            "✓".green().bold(),
            already_current
        );
    }

    Ok(())
}

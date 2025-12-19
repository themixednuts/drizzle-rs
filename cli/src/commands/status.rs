//! Status command implementation
//!
//! Shows migration status (applied vs pending).

use colored::Colorize;

use crate::config::DrizzleConfig;
use crate::error::CliError;

/// Run the status command
pub fn run(config: &DrizzleConfig, db_name: Option<&str>) -> Result<(), CliError> {
    use drizzle_migrations::journal::Journal;

    let db = config.database(db_name)?;

    println!("{}", "Migration Status".bright_cyan());
    println!();

    if !config.is_single_database() {
        let name = db_name.unwrap_or("(default)");
        println!("  {}: {}", "Database".bright_blue(), name);
        println!();
    }

    let out_dir = db.migrations_dir();
    let journal_path = db.journal_path();

    // Check if migrations directory exists
    if !out_dir.exists() {
        println!("  {}", "No migrations directory found.".yellow());
        println!("  Run 'drizzle generate' to create your first migration.");
        return Ok(());
    }

    // Load journal
    let journal = if journal_path.exists() {
        Journal::load(&journal_path).map_err(|e| CliError::IoError(e.to_string()))?
    } else {
        println!("  {}", "No migrations journal found.".yellow());
        println!("  Run 'drizzle generate' to create your first migration.");
        return Ok(());
    };

    if journal.entries.is_empty() {
        println!("  {}", "No migrations found.".yellow());
        return Ok(());
    }

    // Display migration entries
    println!("  {} migration(s) in journal:\n", journal.entries.len());

    for (i, entry) in journal.entries.iter().enumerate() {
        // Migration is in {out}/{tag}/migration.sql
        let migration_path = out_dir.join(&entry.tag).join("migration.sql");
        let snapshot_path = out_dir.join(&entry.tag).join("snapshot.json");

        let sql_exists = migration_path.exists();
        let snapshot_exists = snapshot_path.exists();

        let status_icon = if sql_exists && snapshot_exists {
            "✓".green()
        } else if sql_exists {
            "○".yellow()
        } else {
            "✗".red()
        };
        let idx_display = format!("{:3}.", i + 1).bright_black();

        println!("  {} {} {}", idx_display, status_icon, entry.tag);

        if !sql_exists {
            println!("      {}", "Migration file missing!".red());
        }
        if !snapshot_exists && sql_exists {
            println!("      {}", "Snapshot file missing".yellow());
        }
    }

    println!();
    println!(
        "  {}: {}",
        "Migrations directory".bright_black(),
        out_dir.display()
    );
    println!(
        "  {}: {}",
        "Schema files".bright_black(),
        db.schema_display()
    );

    Ok(())
}

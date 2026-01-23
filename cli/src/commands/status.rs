//! Status command implementation
//!
//! Shows migration status (applied vs pending).

use crate::config::DrizzleConfig;
use crate::error::CliError;
use crate::output;

/// Run the status command
pub fn run(config: &DrizzleConfig, db_name: Option<&str>) -> Result<(), CliError> {
    use drizzle_migrations::journal::Journal;

    let db = config.database(db_name)?;

    println!("{}", output::heading("Migration Status"));
    println!();

    if !config.is_single_database() {
        let name = db_name.unwrap_or("(default)");
        println!("  {}: {}", output::label("Database"), name);
        println!();
    }

    let out_dir = db.migrations_dir();
    let journal_path = db.journal_path();

    // Check if migrations directory exists
    if !out_dir.exists() {
        println!("  {}", output::warning("No migrations directory found."));
        println!("  Run 'drizzle generate' to create your first migration.");
        return Ok(());
    }

    // Load journal
    let journal = if journal_path.exists() {
        Journal::load(&journal_path).map_err(|e| CliError::IoError(e.to_string()))?
    } else {
        println!("  {}", output::warning("No migrations journal found."));
        println!("  Run 'drizzle generate' to create your first migration.");
        return Ok(());
    };

    if journal.entries.is_empty() {
        println!("  {}", output::warning("No migrations found."));
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
            output::success("✓")
        } else if sql_exists {
            output::warning("○")
        } else {
            output::error("✗")
        };
        let idx_display = output::muted(&format!("{:3}.", i + 1));

        println!("  {} {} {}", idx_display, status_icon, entry.tag);

        if !sql_exists {
            println!("      {}", output::error("Migration file missing!"));
        }
        if !snapshot_exists && sql_exists {
            println!("      {}", output::warning("Snapshot file missing"));
        }
    }

    println!();
    println!(
        "  {}: {}",
        output::muted("Migrations directory"),
        out_dir.display()
    );
    println!(
        "  {}: {}",
        output::muted("Schema files"),
        db.schema_display()
    );

    Ok(())
}

//! Status command implementation
//!
//! Shows migration status (applied vs pending).

use crate::config::Config;
use crate::error::CliError;
use crate::output;

/// Run the status command
pub fn run(config: &Config, db_name: Option<&str>) -> Result<(), CliError> {
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

    if journal_path.exists() {
        println!(
            "  {}",
            output::warning("Legacy migration journal detected. Run 'drizzle upgrade' first.")
        );
        println!();
    }

    let entries = discover_migration_dirs(out_dir)?;
    if entries.is_empty() {
        println!("  {}", output::warning("No migrations found."));
        return Ok(());
    }

    // Display migration entries
    println!("  {} migration folder(s):\n", entries.len());

    for (i, (tag, migration_path, snapshot_path)) in entries.iter().enumerate() {
        // Migration is in {out}/{tag}/migration.sql
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

        println!("  {} {} {}", idx_display, status_icon, tag);

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

fn discover_migration_dirs(
    out_dir: &std::path::Path,
) -> Result<Vec<(String, std::path::PathBuf, std::path::PathBuf)>, CliError> {
    let mut entries = Vec::new();

    for entry in std::fs::read_dir(out_dir).map_err(|e| CliError::IoError(e.to_string()))? {
        let entry = entry.map_err(|e| CliError::IoError(e.to_string()))?;
        if !entry
            .file_type()
            .map_err(|e| CliError::IoError(e.to_string()))?
            .is_dir()
        {
            continue;
        }

        let tag = entry.file_name().to_string_lossy().to_string();
        if tag == "meta" {
            continue;
        }

        let path = entry.path();
        let migration_path = path.join("migration.sql");
        if !migration_path.exists() {
            continue;
        }

        let snapshot_path = path.join("snapshot.json");
        entries.push((tag, migration_path, snapshot_path));
    }

    entries.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(entries)
}

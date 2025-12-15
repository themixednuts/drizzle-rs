//! Migrate command implementation
//!
//! Runs pending migrations against the database.
//! Note: This command requires database connectivity which depends on
//! driver-specific features being enabled.

use colored::Colorize;

use crate::config::DrizzleConfig;
use crate::error::CliError;

/// Run the migrate command
pub fn run(config: &DrizzleConfig) -> Result<(), CliError> {
    use drizzle_migrations::journal::Journal;

    println!("{}", "🚀 Running migrations...".bright_cyan());
    println!();

    let out_dir = config.migrations_dir();
    let journal_path = config.journal_path();

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
        println!("  {}", "No migrations to run.".yellow());
        return Ok(());
    }

    // Read all pending migration SQL files
    let mut migrations_to_run = Vec::new();
    for entry in &journal.entries {
        let migration_path = out_dir.join(&entry.tag).join("migration.sql");
        if migration_path.exists() {
            let sql = std::fs::read_to_string(&migration_path).map_err(|e| {
                CliError::IoError(format!(
                    "Failed to read {}: {}",
                    migration_path.display(),
                    e
                ))
            })?;
            migrations_to_run.push((entry.tag.clone(), sql));
        } else {
            return Err(CliError::IoError(format!(
                "Migration file not found: {}",
                migration_path.display()
            )));
        }
    }

    println!(
        "  {} {} migration(s) found",
        "Found".bright_blue(),
        migrations_to_run.len()
    );
    println!();

    // Note: Actual database execution requires driver-specific implementations
    // This CLI shows what would be executed but actual migration requires
    // using the programmatic API with a database connection
    println!(
        "{}",
        "⚠️  Database migration requires a connection.".yellow()
    );
    println!();
    println!("  Use the programmatic API to run migrations:");
    println!();
    println!(
        "  {}",
        "let (db, schema) = Drizzle::new(connection, Schema::new());".bright_black()
    );
    println!("  {}", "db.migrate().await?;".bright_black());
    println!();

    println!("  Migrations that would be applied:");
    for (tag, _sql) in &migrations_to_run {
        println!("    {} {}", "→".bright_blue(), tag);
    }

    Ok(())
}

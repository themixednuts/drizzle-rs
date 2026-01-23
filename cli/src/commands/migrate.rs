//! Migrate command implementation
//!
//! Runs pending migrations against the database.

use colored::Colorize;

use crate::config::DrizzleConfig;
use crate::error::CliError;

/// Run the migrate command
pub fn run(config: &DrizzleConfig, db_name: Option<&str>) -> Result<(), CliError> {
    let db = config.database(db_name)?;

    if !config.is_single_database() {
        let name = db_name.unwrap_or("(default)");
        println!("{} {}", "Database:".bright_blue(), name);
    }

    println!("{}", "Running migrations...".bright_cyan());
    println!();

    let out_dir = db.migrations_dir();

    // Check if migrations directory exists
    if !out_dir.exists() {
        println!("  {}", "No migrations directory found.".yellow());
        println!("  Run 'drizzle generate' to create your first migration.");
        return Ok(());
    }

    // Get credentials
    let credentials = db.credentials()?;

    let credentials = match credentials {
        Some(c) => c,
        None => {
            println!("{}", "No database credentials configured.".yellow());
            println!();
            println!("Add credentials to your drizzle.config.toml:");
            println!();
            println!("  {}", "[dbCredentials]".bright_black());
            println!("  {}", "url = \"./dev.db\"".bright_black());
            println!();
            println!("Or use an environment variable:");
            println!();
            println!("  {}", "[dbCredentials]".bright_black());
            println!("  {}", "url = { env = \"DATABASE_URL\" }".bright_black());
            return Ok(());
        }
    };

    // Run migrations
    let result = crate::db::run_migrations(
        &credentials,
        db.dialect,
        out_dir,
        db.migrations_table(),
        db.migrations_schema(),
    )?;

    if result.applied_count == 0 {
        println!("  {}", "No pending migrations.".green());
    } else {
        println!(
            "  {} {} migration(s):",
            "Applied".green(),
            result.applied_count
        );
        for hash in &result.applied_migrations {
            println!("    {} {}", "->".bright_blue(), hash);
        }
    }

    println!();
    println!("{}", "Migrations complete!".bright_green());

    Ok(())
}

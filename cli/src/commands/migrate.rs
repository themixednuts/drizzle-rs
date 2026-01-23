//! Migrate command implementation
//!
//! Runs pending migrations against the database.

use crate::config::DrizzleConfig;
use crate::error::CliError;
use crate::output;

/// Run the migrate command
pub fn run(config: &DrizzleConfig, db_name: Option<&str>) -> Result<(), CliError> {
    let db = config.database(db_name)?;

    if !config.is_single_database() {
        let name = db_name.unwrap_or("(default)");
        println!("{}: {}", output::label("Database"), name);
    }

    println!("{}", output::heading("Running migrations..."));
    println!();

    let out_dir = db.migrations_dir();

    // Check if migrations directory exists
    if !out_dir.exists() {
        println!("  {}", output::warning("No migrations directory found."));
        println!("  Run 'drizzle generate' to create your first migration.");
        return Ok(());
    }

    // Get credentials
    let credentials = db.credentials()?;

    let credentials = match credentials {
        Some(c) => c,
        None => {
            println!("{}", output::warning("No database credentials configured."));
            println!();
            println!("Add credentials to your drizzle.config.toml:");
            println!();
            println!("  {}", output::muted("[dbCredentials]"));
            println!("  {}", output::muted("url = \"./dev.db\""));
            println!();
            println!("Or use an environment variable:");
            println!();
            println!("  {}", output::muted("[dbCredentials]"));
            println!("  {}", output::muted("url = { env = \"DATABASE_URL\" }"));
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
        println!("  {}", output::success("No pending migrations."));
    } else {
        println!(
            "  {} {} migration(s):",
            output::success("Applied"),
            result.applied_count
        );
        for hash in &result.applied_migrations {
            println!("    {} {}", output::label("->"), hash);
        }
    }

    println!();
    println!("{}", output::success("Migrations complete!"));

    Ok(())
}

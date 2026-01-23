//! Introspect command implementation
//!
//! Introspects an existing database and generates a snapshot/schema.

use std::path::Path;

use colored::Colorize;

use crate::config::{DrizzleConfig, IntrospectCasing};
use crate::error::CliError;

/// Run the introspect command
pub fn run(
    config: &DrizzleConfig,
    db_name: Option<&str>,
    init_metadata: bool,
    casing: Option<IntrospectCasing>,
    out_override: Option<&Path>,
    breakpoints_override: Option<bool>,
) -> Result<(), CliError> {
    let db = config.database(db_name)?;

    // CLI flags override config
    let _effective_casing = casing.unwrap_or_else(|| db.effective_introspect_casing());
    let effective_out = out_override.unwrap_or(db.migrations_dir());
    let _effective_breakpoints = breakpoints_override.unwrap_or(db.breakpoints);

    println!("{}", "Introspecting database...".bright_cyan());
    println!();

    if !config.is_single_database() {
        let name = db_name.unwrap_or("(default)");
        println!("  {}: {}", "Database".bright_blue(), name);
    }

    println!("  {}: {}", "Dialect".bright_blue(), db.dialect.as_str());
    if let Some(ref driver) = db.driver {
        println!("  {}: {:?}", "Driver".bright_blue(), driver);
    }
    println!("  {}: {}", "Output".bright_blue(), effective_out.display());

    if init_metadata {
        println!("  {}: enabled", "Init metadata".bright_blue());
    }
    println!();

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
            match db.dialect.to_base() {
                drizzle_types::Dialect::SQLite => {
                    println!("  {}", "url = \"./dev.db\"".bright_black());
                }
                drizzle_types::Dialect::PostgreSQL => {
                    println!(
                        "  {}",
                        "url = \"postgres://user:pass@localhost:5432/db\"".bright_black()
                    );
                }
                drizzle_types::Dialect::MySQL => {
                    // drizzle-cli doesn't currently support MySQL end-to-end, but the base
                    // dialect type includes it, so keep the match exhaustive.
                    println!(
                        "  {}",
                        "url = \"mysql://user:pass@localhost:3306/db\"".bright_black()
                    );
                }
            }
            println!();
            println!("Or use an environment variable:");
            println!();
            println!("  {}", "[dbCredentials]".bright_black());
            println!("  {}", "url = { env = \"DATABASE_URL\" }".bright_black());
            return Ok(());
        }
    };

    // Run introspection
    let result = crate::db::run_introspection(
        &credentials,
        db.dialect,
        effective_out,
        init_metadata,
        db.migrations_table(),
        db.migrations_schema(),
    )?;

    println!();
    println!(
        "  {} {} table(s), {} index(es)",
        "Found".green(),
        result.table_count,
        result.index_count
    );

    if result.view_count > 0 {
        println!("  {} {} view(s)", "Found".green(), result.view_count);
    }

    println!();
    println!(
        "{} Snapshot saved to {}",
        "Done!".bright_green(),
        result.snapshot_path.display()
    );

    if init_metadata {
        println!();
        println!(
            "  {} Migration metadata initialized in database.",
            "Note:".bright_blue()
        );
        println!("  The current database state is now the baseline for future migrations.");
    }

    Ok(())
}

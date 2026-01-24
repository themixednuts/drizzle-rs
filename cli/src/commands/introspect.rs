//! Introspect command implementation
//!
//! Introspects an existing database and generates a snapshot/schema.

use std::path::Path;

use crate::config::{DrizzleConfig, IntrospectCasing};
use crate::error::CliError;
use crate::output;

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

    println!("{}", output::heading("Introspecting database..."));
    println!();

    if !config.is_single_database() {
        let name = db_name.unwrap_or("(default)");
        println!("  {}: {}", output::label("Database"), name);
    }

    println!("  {}: {}", output::label("Dialect"), db.dialect.as_str());
    if let Some(ref driver) = db.driver {
        println!("  {}: {:?}", output::label("Driver"), driver);
    }
    println!("  {}: {}", output::label("Output"), effective_out.display());

    if init_metadata {
        println!("  {}: enabled", output::label("Init metadata"));
    }
    println!();

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
            match db.dialect.to_base() {
                drizzle_types::Dialect::SQLite => {
                    println!("  {}", output::muted("url = \"./dev.db\""));
                }
                drizzle_types::Dialect::PostgreSQL => {
                    println!(
                        "  {}",
                        output::muted("url = \"postgres://user:pass@localhost:5432/db\"")
                    );
                }
                drizzle_types::Dialect::MySQL => {
                    // drizzle-cli doesn't currently support MySQL end-to-end, but the base
                    // dialect type includes it, so keep the match exhaustive.
                    println!(
                        "  {}",
                        output::muted("url = \"mysql://user:pass@localhost:3306/db\"")
                    );
                }
            }
            println!();
            println!("Or use an environment variable:");
            println!();
            println!("  {}", output::muted("[dbCredentials]"));
            println!("  {}", output::muted("url = { env = \"DATABASE_URL\" }"));
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
        output::success("Found"),
        result.table_count,
        result.index_count
    );

    if result.view_count > 0 {
        println!(
            "  {} {} view(s)",
            output::success("Found"),
            result.view_count
        );
    }

    println!();
    println!(
        "{} Snapshot saved to {}",
        output::success("Done!"),
        result.snapshot_path.display()
    );

    if init_metadata {
        println!();
        println!(
            "  {} Migration metadata initialized in database.",
            output::label("Note:")
        );
        println!("  The current database state is now the baseline for future migrations.");
    }

    Ok(())
}

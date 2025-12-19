//! Introspect command implementation
//!
//! Introspects an existing database and generates a snapshot/schema.
//! Note: This command requires database connectivity which depends on
//! driver-specific features being enabled.

use colored::Colorize;

use crate::config::DrizzleConfig;
use crate::error::CliError;

/// Run the introspect command
pub fn run(config: &DrizzleConfig, db_name: Option<&str>, init_metadata: bool) -> Result<(), CliError> {
    let db = config.database(db_name)?;

    println!("{}", "Introspecting database...".bright_cyan());
    println!();

    if !config.is_single_database() {
        let name = db_name.unwrap_or("(default)");
        println!("  {}: {}", "Database".bright_blue(), name);
    }

    let dialect = db.dialect.to_base();

    println!("  {}: {}", "Dialect".bright_blue(), db.dialect.as_str());
    if let Some(ref driver) = db.driver {
        println!("  {}: {:?}", "Driver".bright_blue(), driver);
    }
    println!(
        "  {}: {}",
        "Output".bright_blue(),
        db.out.display()
    );

    if init_metadata {
        println!("  {}: enabled", "Init metadata".bright_blue());
    }
    println!();

    // Note: Introspection requires connecting to the database
    // This requires driver-specific implementations
    println!(
        "{}",
        "Introspection requires a database connection.".yellow()
    );
    println!();
    println!("  Use the programmatic API to introspect:");
    println!();

    match dialect {
        drizzle_types::Dialect::SQLite => {
            println!(
                "  {}",
                "let config = RusqliteConfigBuilder::new(\"./dev.db\")".bright_black()
            );
            println!("  {}", "    .schema::<AppSchema>()".bright_black());
            println!("  {}", "    .out(\"./drizzle\")".bright_black());
            println!("  {}", "    .build();".bright_black());
            println!(
                "  {}",
                "config.run_cli(); // then: cargo run --bin drizzle -- introspect".bright_black()
            );
        }
        drizzle_types::Dialect::PostgreSQL => {
            println!(
                "  {}",
                "let config = TokioPostgresConfigBuilder::new(host, port, user, pass, db)"
                    .bright_black()
            );
            println!("  {}", "    .schema::<AppSchema>()".bright_black());
            println!("  {}", "    .out(\"./drizzle\")".bright_black());
            println!("  {}", "    .build();".bright_black());
            println!(
                "  {}",
                "config.run_cli().await; // then: cargo run --bin drizzle -- introspect"
                    .bright_black()
            );
        }
        drizzle_types::Dialect::MySQL => {
            println!("  MySQL introspection not yet supported.");
        }
    }

    if init_metadata {
        println!();
        println!(
            "  {} The --init flag will create migration metadata after introspection.",
            "Note:".bright_blue()
        );
        println!("  This treats the current database state as the baseline for future migrations.");
    }

    Ok(())
}

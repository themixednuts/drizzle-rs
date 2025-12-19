//! Push command implementation
//!
//! Pushes schema changes directly to the database without creating migration files.
//! Note: This command requires database connectivity which depends on
//! driver-specific features being enabled.

use colored::Colorize;

use crate::config::DrizzleConfig;
use crate::error::CliError;
use crate::snapshot::parse_result_to_snapshot;

/// Run the push command
pub fn run(
    config: &DrizzleConfig,
    db_name: Option<&str>,
    cli_verbose: bool,
    cli_strict: bool,
    _force: bool,
) -> Result<(), CliError> {
    use drizzle_migrations::parser::SchemaParser;

    let db = config.database(db_name)?;

    // CLI flags override config
    let verbose = cli_verbose || db.verbose;
    let strict = cli_strict || db.strict;

    if !config.is_single_database() {
        let name = db_name.unwrap_or("(default)");
        println!("{} {}", "Database:".bright_blue(), name);
    }

    println!("{}", "Pushing schema to database...".bright_cyan());
    println!();

    // Parse schema files
    let schema_files = db.schema_files()?;
    if schema_files.is_empty() {
        return Err(CliError::NoSchemaFiles(db.schema_display()));
    }

    println!(
        "  {} {} schema file(s)",
        "Parsing".bright_blue(),
        schema_files.len()
    );

    let mut combined_code = String::new();
    for path in &schema_files {
        let code = std::fs::read_to_string(path)
            .map_err(|e| CliError::IoError(format!("Failed to read {}: {}", path.display(), e)))?;
        combined_code.push_str(&code);
        combined_code.push('\n');
    }

    let parse_result = SchemaParser::parse(&combined_code);

    if parse_result.tables.is_empty() && parse_result.indexes.is_empty() {
        println!("{}", "No tables or indexes found in schema files.".yellow());
        return Ok(());
    }

    println!(
        "  {} {} table(s), {} index(es)",
        "Found".bright_blue(),
        parse_result.tables.len(),
        parse_result.indexes.len()
    );

    // Build snapshot from parsed schema
    let _code_snapshot = parse_result_to_snapshot(&parse_result);

    // Display verbose output if enabled
    if verbose {
        println!();
        println!("{}", "Verbose mode enabled - SQL statements:".bright_blue());
        // TODO: Generate and display SQL statements
    }

    // Note: Push requires introspecting the database and comparing snapshots
    // This requires driver-specific implementations
    println!();
    println!("{}", "Push requires a database connection.".yellow());
    println!();
    println!("  Use the programmatic API to push schema:");
    println!();
    println!(
        "  {}",
        "let (db, schema) = Drizzle::new(connection, Schema::new());".bright_black()
    );
    println!("  {}", "db.push().await?;".bright_black());
    println!();
    println!("  Tables that would be synced:");
    for table_name in parse_result.tables.keys() {
        println!("    {} {}", "->".bright_blue(), table_name);
    }

    if strict {
        println!();
        println!(
            "  {} Strict mode is enabled. Would require confirmation before execution.",
            "Note:".yellow()
        );
    }

    Ok(())
}

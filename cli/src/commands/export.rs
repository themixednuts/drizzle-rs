//! Export command implementation
//!
//! Exports the schema as SQL statements.

use std::path::PathBuf;

use colored::Colorize;

use crate::config::DrizzleConfig;
use crate::error::CliError;
use crate::snapshot::parse_result_to_snapshot;

/// Run the export command
pub fn run(config: &DrizzleConfig, output_path: Option<PathBuf>) -> Result<(), CliError> {
    use drizzle_migrations::parser::SchemaParser;

    println!("{}", "Exporting schema as SQL...".bright_cyan());
    println!();

    // Parse schema files
    let schema_files = config.schema_files()?;
    if schema_files.is_empty() {
        return Err(CliError::NoSchemaFiles(config.schema_display()));
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
    let snapshot = parse_result_to_snapshot(&parse_result);

    // Generate SQL from snapshot (create statements for all entities)
    let sql_statements = generate_create_sql(&snapshot, config.breakpoints)?;

    if sql_statements.is_empty() {
        println!("{}", "No SQL statements generated.".yellow());
        return Ok(());
    }

    let sql_content = sql_statements.join("\n\n");

    // Output to file or stdout
    match output_path {
        Some(path) => {
            std::fs::write(&path, &sql_content)
                .map_err(|e| CliError::IoError(format!("Failed to write {}: {}", path.display(), e)))?;
            println!();
            println!(
                "{}",
                format!("Exported {} SQL statement(s) to {}", sql_statements.len(), path.display())
                    .bright_green()
            );
        }
        None => {
            println!();
            println!("{}", "-- Generated SQL --".bright_black());
            println!();
            println!("{}", sql_content);
            println!();
            println!("{}", "-- End of SQL --".bright_black());
        }
    }

    Ok(())
}

/// Generate CREATE SQL statements from a snapshot
fn generate_create_sql(
    snapshot: &drizzle_migrations::schema::Snapshot,
    breakpoints: bool,
) -> Result<Vec<String>, CliError> {
    use drizzle_migrations::schema::Snapshot;

    match snapshot {
        Snapshot::Sqlite(snap) => {
            use drizzle_migrations::sqlite::diff_snapshots;
            use drizzle_migrations::sqlite::statements::SqliteGenerator;
            use drizzle_migrations::sqlite::SQLiteSnapshot;

            // Diff against empty snapshot to get all CREATE statements
            let empty = SQLiteSnapshot::new();
            let diff = diff_snapshots(&empty, snap);
            let generator = SqliteGenerator::new().with_breakpoints(breakpoints);
            Ok(generator.generate_migration(&diff))
        }
        Snapshot::Postgres(snap) => {
            use drizzle_migrations::postgres::diff_full_snapshots;
            use drizzle_migrations::postgres::statements::PostgresGenerator;
            use drizzle_migrations::postgres::PostgresSnapshot;

            // Diff against empty snapshot to get all CREATE statements
            let empty = PostgresSnapshot::new();
            let diff = diff_full_snapshots(&empty, snap);
            let generator = PostgresGenerator::new().with_breakpoints(breakpoints);
            Ok(generator.generate(&diff.diffs))
        }
    }
}

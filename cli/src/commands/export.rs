//! Export command implementation
//!
//! Exports the schema as SQL statements.

use std::path::PathBuf;

use crate::commands::overrides;
use crate::config::Config;
use crate::config::Dialect;
use crate::error::CliError;
use crate::output;
use crate::snapshot::parse_result_to_snapshot;

#[derive(Debug, Clone)]
pub struct ExportOptions {
    pub output_path: Option<PathBuf>,
    pub dialect: Option<Dialect>,
    pub schema: Option<Vec<String>>,
}

/// Run the export command
pub fn run(config: &Config, db_name: Option<&str>, opts: ExportOptions) -> Result<(), CliError> {
    use drizzle_migrations::parser::SchemaParser;

    let db = config.database(db_name)?;
    let effective_dialect = overrides::resolve_dialect(db, opts.dialect);

    if !config.is_single_database() {
        let name = db_name.unwrap_or("(default)");
        println!("{}: {}", output::label("Database"), name);
    }

    println!("{}", output::heading("Exporting schema as SQL..."));
    println!();

    println!(
        "  {}: {}",
        output::label("Dialect"),
        effective_dialect.as_str()
    );

    // Parse schema files
    let schema_files = overrides::resolve_schema_files(db, opts.schema.as_deref())?;
    if schema_files.is_empty() {
        return Err(CliError::NoSchemaFiles(overrides::resolve_schema_display(
            db,
            opts.schema.as_deref(),
        )));
    }

    println!(
        "  {} {} schema file(s)",
        output::label("Parsing"),
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
        println!(
            "{}",
            output::warning("No tables or indexes found in schema files.")
        );
        return Ok(());
    }

    println!(
        "  {} {} table(s), {} index(es)",
        output::label("Found"),
        parse_result.tables.len(),
        parse_result.indexes.len()
    );

    // Build snapshot from parsed schema (use config dialect)
    let dialect = effective_dialect.to_base();
    let snapshot = parse_result_to_snapshot(&parse_result, dialect, db.casing);

    // Generate SQL from snapshot (create statements for all entities)
    let sql_statements = generate_create_sql(&snapshot, db.breakpoints)?;

    if sql_statements.is_empty() {
        println!("{}", output::warning("No SQL statements generated."));
        return Ok(());
    }

    let sql_content = sql_statements.join("\n\n");

    // Output to file or stdout
    match opts.output_path {
        Some(path) => {
            std::fs::write(&path, &sql_content).map_err(|e| {
                CliError::IoError(format!("Failed to write {}: {}", path.display(), e))
            })?;
            println!();
            println!(
                "{}",
                output::success(&format!(
                    "Exported {} SQL statement(s) to {}",
                    sql_statements.len(),
                    path.display()
                ))
            );
        }
        None => {
            println!();
            println!("{}", output::muted("-- Generated SQL --"));
            println!();
            println!("{}", sql_content);
            println!();
            println!("{}", output::muted("-- End of SQL --"));
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
            use drizzle_migrations::sqlite::SQLiteSnapshot;
            use drizzle_migrations::sqlite::diff_snapshots;
            use drizzle_migrations::sqlite::statements::SqliteGenerator;

            // Diff against empty snapshot to get all CREATE statements
            let empty = SQLiteSnapshot::new();
            let diff = diff_snapshots(&empty, snap);
            let generator = SqliteGenerator::new().with_breakpoints(breakpoints);
            Ok(generator.generate_migration(&diff))
        }
        Snapshot::Postgres(snap) => {
            use drizzle_migrations::postgres::PostgresSnapshot;
            use drizzle_migrations::postgres::diff_full_snapshots;
            use drizzle_migrations::postgres::statements::PostgresGenerator;

            // Diff against empty snapshot to get all CREATE statements
            let empty = PostgresSnapshot::new();
            let diff = diff_full_snapshots(&empty, snap);
            let generator = PostgresGenerator::new().with_breakpoints(breakpoints);
            Ok(generator.generate(&diff.diffs))
        }
    }
}

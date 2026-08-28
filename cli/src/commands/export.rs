//! Export command implementation
//!
//! Exports the schema as SQL statements.

use std::path::PathBuf;

use crate::commands::overrides;
use crate::config::Config;
use crate::config::Dialect;
use crate::error::CliError;
use crate::output;
use drizzle_migrations::schema::Snapshot;

#[derive(clap::Args, Debug, Clone)]
pub struct ExportOptions {
    /// Output SQL to a file (default: stdout)
    #[arg(long = "sql")]
    pub output_path: Option<PathBuf>,

    /// Override dialect from config
    #[arg(long)]
    pub dialect: Option<Dialect>,

    /// Override schema path(s)
    #[arg(long, value_delimiter = ',')]
    pub schema: Option<Vec<String>>,
}

/// Run the export command.
///
/// # Errors
///
/// Returns [`CliError`] if the requested database cannot be resolved, the
/// schema files cannot be read/parsed, the resolved snapshot cannot be
/// generated, or if writing the output SQL file fails.
pub fn run(config: &Config, db_name: Option<&str>, opts: ExportOptions) -> Result<(), CliError> {
    use drizzle_migrations::parser::SchemaParser;

    let db = config.database(db_name)?;
    let effective_dialect = overrides::resolve_dialect(db, opts.dialect);

    crate::commands::harness::print_db_header(config, db_name);

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
    crate::snapshot::surface_parse_diagnostics(&parse_result)?;

    println!(
        "  {} {} table(s), {} index(es), {} enum(s), {} view(s)",
        output::label("Found"),
        parse_result.tables.len(),
        parse_result.indexes.len(),
        parse_result.enums.len(),
        parse_result.views.len(),
    );

    // Build snapshot from parsed schema (use config dialect)
    let dialect = effective_dialect.to_base();
    let snapshot = Snapshot::from_parse_result(&parse_result, dialect, db.casing);
    if snapshot.is_empty() {
        println!("{}", output::warning("No schema entities found."));
        return Ok(());
    }

    // Use the public planner so export has the same validation, dependency
    // ordering, and SQL rendering call stack as generated migrations.
    let plan = generate_create_plan(&snapshot)?;
    let sql_statements = &plan.statements;

    if sql_statements.is_empty() {
        println!("{}", output::warning("No SQL statements generated."));
        return Ok(());
    }

    let sql_content = if db.breakpoints {
        plan.to_sql()
    } else {
        sql_statements.join("\n\n")
    };

    // Output to file or stdout
    if let Some(path) = opts.output_path {
        std::fs::write(&path, &sql_content)
            .map_err(|e| CliError::IoError(format!("Failed to write {}: {}", path.display(), e)))?;
        println!();
        println!(
            "{}",
            output::success(&format!(
                "Exported {} SQL statement(s) to {}",
                sql_statements.len(),
                path.display()
            ))
        );
    } else {
        println!();
        println!("{}", output::muted("-- Generated SQL --"));
        println!();
        println!("{sql_content}");
        println!();
        println!("{}", output::muted("-- End of SQL --"));
    }

    Ok(())
}

/// Plan CREATE SQL from an empty snapshot through the shared dialect entrypoint.
fn generate_create_plan(
    snapshot: &drizzle_migrations::schema::Snapshot,
) -> Result<drizzle_migrations::Plan, CliError> {
    let empty = Snapshot::empty(snapshot.dialect());
    drizzle_migrations::diff(&empty, snapshot)
        .map_err(|error| CliError::MigrationError(error.to_string()))
}

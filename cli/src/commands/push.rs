//! Push command implementation
//!
//! Pushes schema changes directly to the database without creating migration files.
//! Note: This command requires database connectivity which depends on
//! driver-specific features being enabled.

use crate::config::{Casing, DrizzleConfig};
use crate::error::CliError;
use crate::output;
use crate::snapshot::parse_result_to_snapshot;

#[derive(Debug, Clone)]
pub struct PushOptions {
    pub cli_verbose: bool,
    pub cli_strict: bool,
    pub force: bool,
    pub cli_explain: bool,
    pub casing: Option<Casing>,
    pub extensions_filters: Option<Vec<String>>,
}

/// Run the push command
pub fn run(
    config: &DrizzleConfig,
    db_name: Option<&str>,
    opts: PushOptions,
) -> Result<(), CliError> {
    use drizzle_migrations::parser::SchemaParser;

    let db = config.database(db_name)?;

    // CLI flags override config
    let verbose = opts.cli_verbose || db.verbose;
    let explain = opts.cli_explain;
    let _effective_casing = opts.casing.unwrap_or_else(|| db.effective_casing());
    // Note: extensions_filters would be used when introspecting the database
    // to filter out extension-specific types (e.g., PostGIS geometry types)
    let _extensions_filters = opts.extensions_filters;

    if opts.cli_strict {
        println!(
            "{}",
            output::warning("Deprecated: Do not use '--strict'. Use '--explain' instead.")
        );
        return Err(CliError::Other("strict flag is deprecated".into()));
    }

    if !config.is_single_database() {
        let name = db_name.unwrap_or("(default)");
        println!("{}: {}", output::label("Database"), name);
    }

    println!("{}", output::heading("Pushing schema to database..."));
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

    // Parse schema files
    let schema_files = db.schema_files()?;
    if schema_files.is_empty() {
        return Err(CliError::NoSchemaFiles(db.schema_display()));
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
    let dialect = db.dialect.to_base();
    let desired_snapshot = parse_result_to_snapshot(&parse_result, dialect);

    // Compute push plan (DB snapshot -> desired snapshot)
    let plan = crate::db::plan_push(&credentials, db.dialect, &desired_snapshot, db.breakpoints)?;

    if !plan.warnings.is_empty() {
        println!("{}", output::warning("Warnings:"));
        for w in &plan.warnings {
            println!("  {} {}", output::warning("-"), w);
        }
        println!();
    }

    // Print SQL plan for explain/verbose
    if explain || verbose {
        if plan.sql_statements.is_empty() {
            println!("{}", output::success("No schema changes detected."));
            return Ok(());
        }

        println!("{}", output::muted("--- Planned SQL ---"));
        println!();
        for stmt in &plan.sql_statements {
            println!("{stmt}\n");
        }
        println!("{}", output::muted("--- End SQL ---"));
        println!();
    }

    // Provide explain/dry-run output when requested
    if explain {
        return Ok(());
    }

    if plan.sql_statements.is_empty() {
        println!("{}", output::success("No schema changes detected."));
        return Ok(());
    }

    // Apply plan
    crate::db::apply_push(&credentials, db.dialect, &plan, opts.force)?;

    println!("{}", output::success("Push complete!"));

    Ok(())
}

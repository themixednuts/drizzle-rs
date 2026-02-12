//! Push command implementation
//!
//! Pushes schema changes directly to the database without creating migration files.
//! Note: This command requires database connectivity which depends on
//! driver-specific features being enabled.

use crate::commands::overrides::{self, ConnectionOverrides};
use crate::config::{Casing, Config, Dialect, Extension};
use crate::error::CliError;
use crate::output;
use crate::snapshot::parse_result_to_snapshot;

#[derive(Debug, Clone)]
pub struct PushOptions {
    pub cli_verbose: bool,
    pub force: bool,
    pub cli_explain: bool,
    pub casing: Option<Casing>,
    pub dialect: Option<Dialect>,
    pub schema: Option<Vec<String>>,
    pub tables_filters: Option<Vec<String>>,
    pub schema_filters: Option<Vec<String>>,
    pub extensions_filters: Option<Vec<Extension>>,
    pub connection: ConnectionOverrides,
}

/// Run the push command
pub fn run(config: &Config, db_name: Option<&str>, opts: PushOptions) -> Result<(), CliError> {
    use drizzle_migrations::parser::SchemaParser;

    let db = config.database(db_name)?;

    // CLI flags override config
    let verbose = opts.cli_verbose || db.verbose;
    let explain = opts.cli_explain;
    let effective_casing = opts.casing.or(db.casing);
    let effective_dialect = overrides::resolve_dialect(db, opts.dialect);

    if effective_dialect != Dialect::Postgresql {
        if opts.schema_filters.as_ref().is_some_and(|v| !v.is_empty()) {
            println!(
                "{}",
                output::warning("Ignoring --schemaFilters: only supported for postgresql")
            );
        }
        if opts
            .extensions_filters
            .as_ref()
            .is_some_and(|v| !v.is_empty())
        {
            println!(
                "{}",
                output::warning("Ignoring --extensionsFilters: only supported for postgresql")
            );
        }
    }

    if !config.is_single_database() {
        let name = db_name.unwrap_or("(default)");
        println!("{}: {}", output::label("Database"), name);
    }

    println!("{}", output::heading("Pushing schema to database..."));
    println!();

    println!(
        "  {}: {}",
        output::label("Dialect"),
        effective_dialect.as_str()
    );

    // Get credentials
    let credentials = overrides::resolve_credentials(db, effective_dialect, &opts.connection)?;
    let credentials = match credentials {
        Some(c) => c,
        None => {
            println!("{}", output::warning("No database credentials configured."));
            println!();
            println!("Add credentials to your drizzle.config.toml:");
            println!();
            println!("  {}", output::muted("[dbCredentials]"));
            match effective_dialect.to_base() {
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
    let mut desired_snapshot = parse_result_to_snapshot(&parse_result, dialect, effective_casing);

    let filters = crate::db::SnapshotFilters {
        tables: overrides::resolve_filter_list(
            opts.tables_filters.as_deref(),
            db.tables_filter.as_ref(),
        ),
        schemas: overrides::resolve_schema_filters(
            effective_dialect,
            opts.schema_filters.as_deref(),
            db.schema_filter.as_ref(),
        ),
        extensions: overrides::resolve_extensions_filter(
            opts.extensions_filters.as_deref(),
            db.extensions_filters.as_deref(),
        ),
    };
    crate::db::apply_snapshot_filters(&mut desired_snapshot, effective_dialect, &filters)?;

    // Compute push plan (DB snapshot -> desired snapshot)
    let plan = crate::db::plan_push(
        &credentials,
        effective_dialect,
        &desired_snapshot,
        db.breakpoints,
        &filters,
    )?;

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
    crate::db::apply_push(&credentials, effective_dialect, &plan, opts.force)?;

    println!("{}", output::success("Push complete!"));

    Ok(())
}

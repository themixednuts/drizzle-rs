//! Introspect command implementation
//!
//! Introspects an existing database and generates a snapshot/schema.

use crate::commands::overrides::{self, ConnectionOverrides};
use crate::config::{Config, Dialect, Extension, IntrospectCasing};
use crate::error::CliError;
use crate::output;

#[derive(Debug, Clone)]
pub struct IntrospectOptions {
    pub init_metadata: bool,
    pub casing: Option<IntrospectCasing>,
    pub out: Option<std::path::PathBuf>,
    pub breakpoints: Option<bool>,
    pub dialect: Option<Dialect>,
    pub tables_filters: Option<Vec<String>>,
    pub schema_filters: Option<Vec<String>>,
    pub extensions_filters: Option<Vec<Extension>>,
    pub connection: ConnectionOverrides,
}

/// Run the introspect command
pub fn run(
    config: &Config,
    db_name: Option<&str>,
    opts: IntrospectOptions,
) -> Result<(), CliError> {
    let db = config.database(db_name)?;

    // CLI flags override config
    let effective_casing = opts
        .casing
        .unwrap_or_else(|| db.effective_introspect_casing());
    let effective_dialect = overrides::resolve_dialect(db, opts.dialect);
    let effective_out = opts.out.as_deref().unwrap_or(db.migrations_dir());
    let effective_breakpoints = opts.breakpoints.unwrap_or(db.breakpoints);

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

    println!("{}", output::heading("Introspecting database..."));
    println!();

    if !config.is_single_database() {
        let name = db_name.unwrap_or("(default)");
        println!("  {}: {}", output::label("Database"), name);
    }

    println!(
        "  {}: {}",
        output::label("Dialect"),
        effective_dialect.as_str()
    );
    if let Some(ref driver) = db.driver {
        println!("  {}: {:?}", output::label("Driver"), driver);
    }
    println!("  {}: {}", output::label("Output"), effective_out.display());

    if opts.init_metadata {
        println!("  {}: enabled", output::label("Init metadata"));
    }
    println!();

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

    // Run introspection
    let result = crate::db::run_introspection(
        &credentials,
        effective_dialect,
        effective_out,
        opts.init_metadata,
        effective_breakpoints,
        Some(effective_casing),
        &filters,
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

    if opts.init_metadata {
        println!();
        println!(
            "  {} Migration metadata initialized in database.",
            output::label("Note:")
        );
        println!("  The current database state is now the baseline for future migrations.");
    }

    Ok(())
}

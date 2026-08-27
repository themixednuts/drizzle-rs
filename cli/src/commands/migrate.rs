//! Migrate command implementation
//!
//! Runs pending migrations against the database.

use crate::commands::overrides::{self, ConnectionOverrides};
use crate::config::{Config, Dialect, Driver};
use crate::error::CliError;
use crate::output;

#[derive(clap::Args, Debug, Clone, Copy, Default)]
pub struct MigrateOptions {
    /// Check migration integrity without applying changes; exits non-zero on
    /// any finding
    ///
    /// Beyond the pending summary, this compares the tracking table against
    /// the local migration files: hash drift (an applied migration whose file
    /// was edited afterwards), applied rows with no local migration,
    /// duplicate tracking rows, and interrupted (dirty) migrations.
    #[arg(long)]
    pub verify: bool,

    /// Show what `migrate` would do without applying changes
    ///
    /// Prints the applied/pending summary and pending tags. Integrity
    /// findings are shown as warnings but do not fail the command — use
    /// `--verify` for a hard integrity gate.
    #[arg(long, visible_alias = "dry-run")]
    pub plan: bool,

    /// Run the `--verify` integrity checks first, then apply only if they pass
    #[arg(long)]
    pub safe: bool,

    /// Reconcile a migration that was interrupted mid-apply, then continue
    ///
    /// Statements of the interrupted migration are classified against the live
    /// schema: `CREATE TABLE` / `CREATE [UNIQUE] INDEX` / `CREATE VIEW` /
    /// CREATE TYPE ... AS ENUM statements whose object already exists with a
    /// matching definition are skipped, the rest are executed. Anything that
    /// cannot be proven either way aborts with a manual-resolution list.
    /// MySQL does not support automatic repair because DDL may have committed
    /// partially; inspect and reconcile its schema manually instead.
    #[arg(long)]
    pub repair: bool,
}

/// Run the migrate command.
///
/// # Errors
///
/// Returns [`CliError`] if mutually exclusive flags are combined, the database
/// or credentials cannot be resolved, connecting to the database fails, or
/// applying migrations fails.
pub fn run(config: &Config, db_name: Option<&str>, opts: MigrateOptions) -> Result<(), CliError> {
    validate_mutex_opts(opts)?;

    let db = config.database(db_name)?;

    crate::commands::harness::print_db_header(config, db_name);

    println!("{}", output::heading(migrate_heading(opts)));
    println!();

    let out_dir = db.migrations_dir();

    // Check if migrations directory exists
    if !out_dir.exists() {
        println!("  {}", output::warning("No migrations directory found."));
        println!("  Run 'drizzle generate' to create your first migration.");
        return Ok(());
    }

    // Codegen-only drivers (e.g. durable-sqlite) have no remote endpoint for the
    // CLI to reach — migrations execute inside the DO runtime. Short-circuit
    // with a pointed message instead of the generic "no credentials" fallback.
    if matches!(db.driver, Some(Driver::DurableSqlite)) {
        print_durable_sqlite_notice(out_dir);
        return Ok(());
    }

    let connection =
        overrides::resolve_connection(db, db.dialect, &ConnectionOverrides::default())?;
    let Some(connection) = connection else {
        print_missing_credentials_help(db.dialect);
        return Ok(());
    };

    let plan = if opts.verify || opts.plan || opts.safe {
        Some(crate::db::plan_migrations(
            &connection,
            out_dir,
            db.migrations_table(),
            db.migrations_schema(),
        )?)
    } else {
        None
    };

    if let Some(plan) = &plan
        && handle_plan_short_circuit(plan, opts)?
    {
        return Ok(());
    }

    // Run migrations
    let result = crate::db::run_migrations(
        &connection,
        out_dir,
        db.migrations_table(),
        db.migrations_schema(),
        opts.repair,
    )?;

    print_migration_result(&result, opts.safe);
    Ok(())
}

fn validate_mutex_opts(opts: MigrateOptions) -> Result<(), CliError> {
    if opts.safe && opts.verify {
        return Err(CliError::Other(
            "--safe can't be combined with --verify".to_string(),
        ));
    }
    if opts.safe && opts.plan {
        return Err(CliError::Other(
            "--safe can't be combined with --plan".to_string(),
        ));
    }
    // --repair applies statements; the read-only modes would silently ignore it.
    if opts.repair && opts.verify {
        return Err(CliError::Other(
            "--repair can't be combined with --verify".to_string(),
        ));
    }
    if opts.repair && opts.plan {
        return Err(CliError::Other(
            "--repair can't be combined with --plan".to_string(),
        ));
    }
    Ok(())
}

const fn migrate_heading(opts: MigrateOptions) -> &'static str {
    if opts.verify {
        "Verifying migrations..."
    } else if opts.plan {
        "Planning migrations..."
    } else if opts.safe {
        "Running safe migration flow..."
    } else if opts.repair {
        "Repairing and running migrations..."
    } else {
        "Running migrations..."
    }
}

fn print_durable_sqlite_notice(out_dir: &std::path::Path) {
    println!(
        "{}",
        output::warning("Durable Objects SQLite runs inside the Workers runtime.")
    );
    println!();
    println!("  The CLI can't apply migrations to a DO from outside.");
    println!(
        "  Apply them at `DurableObject` init time by importing `{}/migrations.js`",
        out_dir.display()
    );
    println!("  and running each statement against `state.storage().sql()`.");
    println!();
    println!(
        "  (This command only generates the SQL + JS bundle — run `drizzle generate` for that.)"
    );
}

fn print_missing_credentials_help(dialect: Dialect) {
    println!("{}", output::warning("No database credentials configured."));
    println!();
    println!("Add credentials to your drizzle.config.toml:");
    println!();
    println!("  {}", output::muted("[dbCredentials]"));
    let example = match dialect {
        Dialect::Sqlite => "url = \"./dev.db\"",
        Dialect::Turso => "url = \"libsql://your-db.turso.io\"",
        Dialect::Postgresql => "url = \"postgres://user:password@localhost:5432/mydb\"",
        Dialect::Mysql => "url = \"mysql://user:password@localhost:3306/mydb\"",
    };
    println!("  {}", output::muted(example));
    println!();
    println!("Or use an environment variable:");
    println!();
    println!("  {}", output::muted("[dbCredentials]"));
    println!("  {}", output::muted("url = { env = \"DATABASE_URL\" }"));
}

/// Print plan summary and return `Ok(true)` if the caller should return early.
///
/// Integrity findings are warnings under `--plan`, and failures under
/// `--verify` and `--safe`.
fn handle_plan_short_circuit(
    plan: &crate::db::MigrationPlan,
    opts: MigrateOptions,
) -> Result<bool, CliError> {
    println!(
        "  {} {}",
        output::label("Applied migrations:"),
        plan.applied_count
    );
    println!(
        "  {} {} ({} statement(s))",
        output::label("Pending migrations:"),
        plan.pending_count,
        plan.pending_statements
    );

    if !plan.pending_migrations.is_empty() {
        println!("  {}", output::label("Pending tags:"));
        for tag in &plan.pending_migrations {
            println!("    {} {}", output::label("->"), tag);
        }
    }

    if !plan.findings.is_empty() {
        println!();
        println!("  {}", output::label("Integrity findings:"));
        for finding in &plan.findings {
            println!("    {} {}", output::warning("!"), finding);
        }
    }
    println!();

    if opts.verify || opts.safe {
        if !plan.findings.is_empty() {
            let mode = if opts.verify {
                "verification"
            } else {
                "--safe"
            };
            return Err(CliError::MigrationError(format!(
                "{} failed with {} integrity finding(s):\n  - {}",
                mode,
                plan.findings.len(),
                plan.findings.join("\n  - ")
            )));
        }
        if opts.verify {
            println!(
                "{}",
                output::success(&format!(
                    "Migration verification passed: {} applied migration(s) match their \
                     local files; no drift, no interrupted rows.",
                    plan.applied_count
                ))
            );
            return Ok(true);
        }
    }

    if opts.plan {
        println!("{}", output::success("Migration plan complete."));
        return Ok(true);
    }

    if opts.safe && plan.pending_count == 0 {
        println!("  {}", output::success("No pending migrations."));
        println!();
        println!("{}", output::success("Safe migration complete!"));
        return Ok(true);
    }

    Ok(false)
}

fn print_migration_result(result: &crate::db::MigrationResult, safe: bool) {
    if !result.repaired_migrations.is_empty() {
        println!(
            "  {} {} interrupted migration(s):",
            output::success("Repaired"),
            result.repaired_migrations.len()
        );
        for tag in &result.repaired_migrations {
            println!("    {} {}", output::label("->"), tag);
        }
        println!();
    }

    if result.applied_count == 0 {
        println!("  {}", output::success("No pending migrations."));
    } else {
        println!(
            "  {} {} migration(s):",
            output::success("Applied"),
            result.applied_count
        );
        for hash in &result.applied_migrations {
            println!("    {} {}", output::label("->"), hash);
        }
    }

    println!();
    if safe {
        println!("{}", output::success("Safe migration complete!"));
    } else {
        println!("{}", output::success("Migrations complete!"));
    }
}

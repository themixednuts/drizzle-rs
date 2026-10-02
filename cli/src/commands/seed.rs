//! `drizzle seed`: fills a live database with deterministic test data.
//!
//! The tables are read from the database itself, so no Rust schema is
//! needed. Needs a database driver feature enabled at build time.

use crate::commands::overrides::{self, ConnectionOverrides, FilterArgs};
use crate::config::{Config, Dialect, Driver};
use crate::error::CliError;
use crate::output;
use std::path::PathBuf;

#[derive(clap::Args, Debug, Clone)]
pub struct SeedOptions {
    /// RNG seed: the same seed and drizzle version give the same rows
    #[arg(long, default_value_t = 0)]
    pub seed: u64,

    /// Rows per table: `--count 50` for every table, `--count users=100`
    /// for one (repeatable). A table without a count that references a
    /// seeded table gets one row per parent row, or `--relation` rows.
    #[arg(long, value_name = "N|TABLE=N")]
    pub count: Vec<String>,

    /// Rows of CHILD per row of PARENT: `--relation users:posts=3`
    /// (repeatable). CHILD needs a foreign key to PARENT.
    #[arg(long, value_name = "PARENT:CHILD=N")]
    pub relation: Vec<String>,

    /// Leave a table out (repeatable)
    #[arg(long, value_name = "TABLE")]
    pub skip: Vec<String>,

    /// Empty the seeded tables first (children before parents)
    #[arg(long)]
    pub reset: bool,

    /// Write the SQL to FILE instead of running it (`-` for stdout)
    #[arg(long, value_name = "FILE")]
    pub out: Option<PathBuf>,

    /// Run `--reset` without asking for confirmation
    #[arg(long)]
    pub force: bool,

    /// Override dialect from config
    #[arg(long)]
    pub dialect: Option<Dialect>,

    #[command(flatten)]
    pub filters: FilterArgs,

    #[command(flatten)]
    pub connection: ConnectionOverrides,
}

/// The parsed `--count`, `--relation` and `--skip` arguments.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct SeedPlan {
    /// `--seed`.
    pub seed: u64,
    /// A bare `--count N`.
    pub default_count: Option<usize>,
    /// `--count TABLE=N`.
    pub counts: Vec<(String, usize)>,
    /// `--relation PARENT:CHILD=N`.
    pub relations: Vec<(String, String, usize)>,
    /// `--skip TABLE`.
    pub skips: Vec<String>,
    /// `--reset`.
    pub reset: bool,
}

impl SeedPlan {
    /// Parses the seed options.
    ///
    /// # Errors
    ///
    /// Returns [`CliError::Other`] for a malformed `--count` or `--relation`
    /// value, or two bare `--count`s.
    pub fn from_options(opts: &SeedOptions) -> Result<Self, CliError> {
        let number = |text: &str, flag: &str, value: &str| {
            text.trim().parse::<usize>().map_err(|_| {
                CliError::Other(format!(
                    "--{flag} {value}: `{}` is not a row count",
                    text.trim()
                ))
            })
        };
        let mut plan = Self {
            seed: opts.seed,
            reset: opts.reset,
            skips: opts.skip.clone(),
            ..Self::default()
        };
        for value in &opts.count {
            match value.rsplit_once('=') {
                Some((table, count)) if !table.trim().is_empty() => {
                    plan.counts
                        .push((table.trim().to_owned(), number(count, "count", value)?));
                }
                Some(_) => {
                    return Err(CliError::Other(format!(
                        "--count {value}: expected N or TABLE=N"
                    )));
                }
                None if plan.default_count.is_some() => {
                    return Err(CliError::Other(
                        "--count without a table was given twice".to_owned(),
                    ));
                }
                None => plan.default_count = Some(number(value, "count", value)?),
            }
        }
        for value in &opts.relation {
            let parsed = value.rsplit_once('=').and_then(|(tables, count)| {
                let (parent, child) = tables.split_once(':')?;
                (!parent.trim().is_empty() && !child.trim().is_empty())
                    .then(|| (parent.trim().to_owned(), child.trim().to_owned(), count))
            });
            let Some((parent, child, count)) = parsed else {
                return Err(CliError::Other(format!(
                    "--relation {value}: expected PARENT:CHILD=N"
                )));
            };
            plan.relations
                .push((parent, child, number(count, "relation", value)?));
        }
        Ok(plan)
    }
}

/// The SQL a seed runs, with every value written inline.
#[derive(Debug, Default)]
pub struct SeedScript {
    /// `DELETE`s (and resets) that empty the tables, children first.
    pub reset: Vec<String>,
    /// The INSERTs (and `PostgreSQL` sequence updates), parents first.
    pub inserts: Vec<String>,
    /// Rows inserted per table, in insert order.
    pub rows: Vec<(String, usize)>,
}

impl SeedScript {
    #[allow(dead_code)] // unused in a build without drivers
    fn add_rows(&mut self, table: &str, rows: usize) {
        match self.rows.last_mut() {
            Some((last, count)) if last == table => *count += rows,
            _ if rows > 0 => self.rows.push((table.to_owned(), rows)),
            _ => {}
        }
    }

    /// The whole script: one statement per line, each ending in `;`.
    #[must_use]
    pub fn to_sql(&self) -> String {
        let mut sql = String::new();
        for statement in self.reset.iter().chain(&self.inserts) {
            sql.push_str(statement);
            sql.push_str(";\n");
        }
        sql
    }
}

fn seed_error(error: drizzle_seed::SeedError) -> CliError {
    CliError::Other(format!("Seeding failed: {error}"))
}

/// Builds the script for `schema` with the dialect's `SeedConfig`
/// constructor.
#[allow(unused_macros)] // unused in a build without drivers
macro_rules! build_script {
    ($constructor:ident, $schema:expr, $plan:expr, $max_params:expr) => {{
        let plan: &SeedPlan = $plan;
        let mut config = drizzle_seed::SeedConfig::$constructor($schema).seed(plan.seed);
        if let Some(count) = plan.default_count {
            config = config.default_count(count);
        }
        if let Some(limit) = $max_params {
            config = config.max_params(limit);
        }
        for (table, count) in &plan.counts {
            config = config.count_by_name(table, *count);
        }
        for (parent, child, count) in &plan.relations {
            config = config.relation_by_name(parent, child, *count);
        }
        for table in &plan.skips {
            config = config.skip_by_name(table);
        }
        let mut script = SeedScript::default();
        if plan.reset {
            for statement in config.reset_plan().map_err(seed_error)? {
                script
                    .reset
                    .push(statement.inline_sql().map_err(seed_error)?);
            }
        }
        for statement in config.try_generate().map_err(seed_error)? {
            script.add_rows(statement.table(), statement.rows());
            script
                .inserts
                .push(statement.inline_sql().map_err(seed_error)?);
        }
        Ok(script)
    }};
}

/// Builds the seed script for `schema` in `dialect`.
///
/// `max_params` caps the values per INSERT (D1's HTTP API limits statement
/// length, so its driver passes a lower cap).
///
/// # Errors
///
/// Returns [`CliError`] when this build has no driver for `dialect`, a
/// table or column name in `plan` is unknown, or seeding fails (for example
/// a foreign-key cycle).
#[allow(unused_variables)] // every dialect arm can be compiled out
pub fn build_script(
    dialect: drizzle_types::Dialect,
    schema: &drizzle_seed::schema::Schema,
    plan: &SeedPlan,
    max_params: Option<usize>,
) -> Result<SeedScript, CliError> {
    match dialect {
        #[cfg(any(
            feature = "rusqlite",
            feature = "libsql",
            feature = "turso",
            feature = "d1-http"
        ))]
        drizzle_types::Dialect::SQLite => build_script!(sqlite, schema, plan, max_params),
        #[cfg(any(feature = "postgres-sync", feature = "tokio-postgres"))]
        drizzle_types::Dialect::PostgreSQL => build_script!(postgres, schema, plan, max_params),
        #[cfg(any(feature = "mysql-sync", feature = "mysql-async"))]
        drizzle_types::Dialect::MySQL => build_script!(mysql, schema, plan, max_params),
        #[allow(unreachable_patterns)]
        other => Err(CliError::Other(format!(
            "this drizzle build has no {other:?} driver; build it with a driver feature to seed"
        ))),
    }
}

/// Runs `drizzle seed`: reads the tables from the live database (after the
/// configured table and schema filters, without the migrations tracking
/// table), generates rows for them, and inserts them in one transaction
/// where the driver supports it. With `--out`, writes the SQL instead.
///
/// # Errors
///
/// Returns [`CliError`] if `db_name` does not match the config, an option is
/// malformed, there are no credentials or no driver for them, reading the
/// database fails, seeding fails, writing `--out` fails, a statement fails,
/// or the user declines `--reset` ([`CliError::Aborted`]).
pub fn run(config: &Config, db_name: Option<&str>, opts: &SeedOptions) -> Result<(), CliError> {
    let db = config.database(db_name)?;
    let plan = SeedPlan::from_options(opts)?;
    let effective_dialect = overrides::resolve_dialect(db, opts.dialect);
    let to_stdout = opts.out.as_deref() == Some(std::path::Path::new("-"));
    // With `--out -`, stdout carries only the SQL.
    let say = |line: String| {
        if to_stdout {
            eprintln!("{line}");
        } else {
            println!("{line}");
        }
    };

    if !to_stdout {
        crate::commands::harness::print_db_header(config, db_name);
    }
    say(output::heading("Seeding database..."));
    say(format!(
        "  {}: {}",
        output::label("Dialect"),
        effective_dialect.as_str()
    ));

    let connection = overrides::resolve_connection(db, effective_dialect, &opts.connection)?;
    let Some(connection) = connection else {
        return Err(CliError::MissingCredentials("seed"));
    };
    say(format!(
        "  {}: {}",
        output::label("Driver"),
        connection.driver
    ));

    let filters = crate::db::SnapshotFilters {
        tables: overrides::resolve_filter_list(
            opts.filters.tables_filter.as_deref(),
            db.tables_filter.as_ref(),
        ),
        schemas: overrides::resolve_schema_filters(
            effective_dialect,
            opts.filters.schema_filters.as_deref(),
            db.schema_filter.as_ref(),
        ),
        extensions: overrides::resolve_extensions_filter(
            effective_dialect,
            opts.filters.extensions_filters.as_deref(),
            db.extensions_filters.as_deref(),
        ),
        roles: None,
    };
    let snapshot = crate::db::introspect_for_seed(&connection, &filters, db.migrations_table())?;
    let schema = drizzle_seed::schema::Schema::from_snapshot(&snapshot).map_err(seed_error)?;
    if schema.tables().is_empty() {
        say(output::warning("No tables found to seed."));
        return Ok(());
    }

    let max_params = (connection.driver == Driver::D1Http).then_some(250);
    let script = build_script(effective_dialect.to_base(), &schema, &plan, max_params)?;

    if let Some(out) = &opts.out {
        let sql = script.to_sql();
        if to_stdout {
            print!("{sql}");
        } else {
            std::fs::write(out, sql).map_err(|error| {
                CliError::IoError(format!("Failed to write {}: {error}", out.display()))
            })?;
            say(format!(
                "{} Wrote {} statement(s) to {}",
                output::success("Done!"),
                script.reset.len() + script.inserts.len(),
                out.display()
            ));
        }
        return Ok(());
    }

    if plan.reset && !opts.force && !confirm_reset(&script)? {
        return Err(CliError::Aborted(
            "Seed aborted: --reset was not confirmed, so nothing was changed".into(),
        ));
    }

    let statements: Vec<String> = script
        .reset
        .iter()
        .chain(&script.inserts)
        .cloned()
        .collect();
    crate::db::execute_seed(&connection, &statements)?;

    println!();
    for (table, rows) in &script.rows {
        println!("  {} {table}: {rows} row(s)", output::success("+"));
    }
    println!();
    println!("{}", output::success("Seed complete!"));
    Ok(())
}

/// Asks before `--reset` deletes rows; refuses in a non-interactive shell.
fn confirm_reset(script: &SeedScript) -> Result<bool, CliError> {
    use std::io::{self, IsTerminal, Write};

    if !io::stdin().is_terminal() {
        return Err(CliError::Other(
            "Refusing to delete rows for --reset in non-interactive mode. Use --force.".into(),
        ));
    }
    println!(
        "{}",
        output::warning(&format!(
            "--reset deletes every row of the {} seeded table(s) first.",
            script.rows.len()
        ))
    );
    print!("Continue? [y/N]: ");
    io::stdout()
        .flush()
        .map_err(|error| CliError::IoError(error.to_string()))?;
    let mut answer = String::new();
    io::stdin()
        .read_line(&mut answer)
        .map_err(|error| CliError::IoError(error.to_string()))?;
    Ok(matches!(answer.trim(), "y" | "Y" | "yes" | "YES" | "Yes"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[derive(Parser)]
    struct Cli {
        #[command(flatten)]
        seed: SeedOptions,
    }

    fn plan(args: &[&str]) -> Result<SeedPlan, CliError> {
        let mut argv = vec!["drizzle"];
        argv.extend_from_slice(args);
        SeedPlan::from_options(&Cli::parse_from(argv).seed)
    }

    #[test]
    fn parses_counts_relations_and_skips() {
        let plan = plan(&[
            "--seed",
            "7",
            "--count",
            "20",
            "--count",
            "users=100",
            "--count",
            "auth.sessions = 5",
            "--relation",
            "users:posts=3",
            "--skip",
            "audit_log",
            "--reset",
        ])
        .unwrap();
        assert_eq!(
            plan,
            SeedPlan {
                seed: 7,
                default_count: Some(20),
                counts: vec![("users".into(), 100), ("auth.sessions".into(), 5)],
                relations: vec![("users".into(), "posts".into(), 3)],
                skips: vec!["audit_log".into()],
                reset: true,
            }
        );
    }

    #[test]
    fn rejects_malformed_values() {
        for args in [
            &["--count", "lots"][..],
            &["--count", "users=lots"],
            &["--count", "=5"],
            &["--count", "1", "--count", "2"],
            &["--relation", "users=3"],
            &["--relation", "users:=3"],
            &["--relation", "users:posts=x"],
        ] {
            assert!(plan(args).is_err(), "{args:?}");
        }
    }
}

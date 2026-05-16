//! Drizzle CLI - Main entry point
//!
//! This is the main binary for the drizzle-cli tool.
//! CLI interface matches drizzle-kit for TypeScript compatibility.

use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::process::ExitCode;

use drizzle_cli::commands::{
    check::CheckOptions, export::ExportOptions, generate::GenerateOptions,
    introspect::IntrospectOptions, migrate::MigrateOptions, new::NewOptions, push::PushOptions,
    upgrade::UpgradeOptions,
};
use drizzle_cli::config::Config;
use drizzle_cli::error::CliError;
use drizzle_cli::output;

/// Default configuration file name
const DEFAULT_CONFIG_FILE: &str = "drizzle.config.toml";

/// JSON schema URL for TOML validation
const SCHEMA_URL: &str =
    "https://raw.githubusercontent.com/themixednuts/drizzle-rs/main/cli/schema.json";

/// Drizzle - Database migration CLI for drizzle-rs
#[derive(Parser, Debug)]
#[command(name = "drizzle")]
#[command(author, version, about = "Database migration CLI for drizzle-rs", long_about = None)]
struct Cli {
    /// Path to config file (default: drizzle.config.toml)
    #[arg(short, long, global = true, value_name = "PATH")]
    config: Option<PathBuf>,

    /// Database name (for multi-database configs)
    #[arg(long, global = true, value_name = "NAME")]
    db: Option<String>,

    #[command(subcommand)]
    command: Command,
}

/// CLI subcommands.
///
/// Each variant carries the `Options` struct that lives next to the command
/// implementation in `drizzle_cli::commands::*`. The struct itself derives
/// `clap::Args`, so adding/renaming flags happens in exactly one place — the
/// dispatcher below is just `Command::X(opts) => commands::x::run(...)`.
///
/// These commands match drizzle-kit for TypeScript compatibility.
#[derive(Subcommand, Debug)]
enum Command {
    /// Generate a new migration from schema changes
    Generate(GenerateOptions),

    /// Run pending migrations
    Migrate(MigrateOptions),

    /// Upgrade migration snapshots to the latest version
    Up(UpgradeOptions),

    /// Push schema changes directly to database (without migration files)
    Push(PushOptions),

    /// Introspect database and generate schema
    Introspect(IntrospectOptions),

    /// Introspect database and generate schema (alias for introspect)
    Pull(IntrospectOptions),

    /// Show migration status
    Status,

    /// Validate configuration file
    Check(CheckOptions),

    /// Export schema as SQL statements
    Export(ExportOptions),

    /// Interactively build a new schema file
    New(NewOptions),

    /// Initialize a new drizzle.config.toml configuration file
    Init {
        /// Database dialect (sqlite, postgresql, turso)
        #[arg(short, long, default_value = "sqlite", value_parser = ["sqlite", "postgresql", "postgres", "turso"])]
        dialect: String,

        /// Database driver (optional; Rust drivers only)
        ///
        /// - sqlite: rusqlite
        /// - turso: libsql, turso
        /// - postgresql: postgres-sync, tokio-postgres
        #[arg(short = 'r', long, value_parser = ["rusqlite", "libsql", "turso", "postgres-sync", "tokio-postgres"])]
        driver: Option<String>,
    },
}

fn main() -> ExitCode {
    // Load .env file if present (silently ignore if not found)
    let _ = dotenvy::dotenv();

    let cli = Cli::parse();

    let result = run(cli);

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            let msg = e.to_string();
            eprintln!("{}", output::err_line(&msg));
            ExitCode::FAILURE
        }
    }
}

fn run(cli: Cli) -> Result<(), CliError> {
    use drizzle_cli::commands;
    let db_name = cli.db.as_deref();
    let config_path = cli.config.as_deref();

    match cli.command {
        // `new` runs with an optional config (the wizard can scaffold from
        // scratch); `init` doesn't read a config at all.
        Command::New(opts) => commands::new::run(load_config(config_path).ok().as_ref(), &opts),
        Command::Init { dialect, driver } => run_init(&dialect, driver.as_deref()),

        // Everything else requires a loaded config.
        Command::Generate(opts) => {
            commands::generate::run(&load_config(config_path)?, db_name, opts)
        }
        Command::Migrate(opts) => commands::migrate::run(&load_config(config_path)?, db_name, opts),
        Command::Up(opts) => commands::upgrade::run(&load_config(config_path)?, db_name, &opts),
        Command::Push(opts) => commands::push::run(&load_config(config_path)?, db_name, &opts),
        Command::Introspect(opts) | Command::Pull(opts) => {
            commands::introspect::run(&load_config(config_path)?, db_name, &opts)
        }
        Command::Status => commands::status::run(&load_config(config_path)?, db_name),
        Command::Check(opts) => commands::check::run(&load_config(config_path)?, db_name, &opts),
        Command::Export(opts) => commands::export::run(&load_config(config_path)?, db_name, opts),
    }
}

/// Load configuration with fallback to default path
fn load_config(custom_path: Option<&std::path::Path>) -> Result<Config, CliError> {
    custom_path.map_or_else(
        || Config::load().map_err(Into::into),
        |path| {
            // If the user points at a config in a different directory, also try to load a `.env`
            // next to that config before resolving `{ env = "NAME" }` values.
            if let Some(dir) = path.parent() {
                let _ = dotenvy::from_path(dir.join(".env"));
            }
            Config::load_from(path).map_err(Into::into)
        },
    )
}

/// Initialize a new drizzle.config.toml file
fn run_init(dialect: &str, driver: Option<&str>) -> Result<(), CliError> {
    let config_path = PathBuf::from(DEFAULT_CONFIG_FILE);

    if config_path.exists() {
        return Err(CliError::Other(format!(
            "{DEFAULT_CONFIG_FILE} already exists. Delete it first to reinitialize."
        )));
    }

    let config_content = generate_init_config(dialect, driver)?;

    std::fs::write(&config_path, config_content).map_err(|e| CliError::IoError(e.to_string()))?;

    println!(
        "{}",
        output::success(&format!("Created {DEFAULT_CONFIG_FILE}"))
    );
    println!();
    println!("Next steps:");
    println!("  1. Edit {DEFAULT_CONFIG_FILE} with your database credentials");
    println!(
        "  2. Create your schema file at {}",
        output::heading("src/schema.rs")
    );
    println!(
        "  3. Run {} to generate your first migration",
        output::heading("drizzle generate")
    );

    Ok(())
}

/// Generate the init configuration content based on dialect and driver
fn generate_init_config(dialect: &str, driver: Option<&str>) -> Result<String, CliError> {
    let dialect = dialect.to_lowercase();
    let driver = driver.map(str::to_lowercase);

    // Rust-only: keep init output aligned with what `cli/src/config.rs` can actually parse.
    match dialect.as_str() {
        "sqlite" => {
            if let Some(ref d) = driver
                && d != "rusqlite"
            {
                return Err(CliError::Other(format!(
                    "Invalid driver for sqlite: {d}. Supported: rusqlite"
                )));
            }
            Ok(format!(
                r#"#:schema {SCHEMA_URL}

# Drizzle Configuration (drizzle-rs)
#
# This file is parsed by `drizzle-cli` and should stay aligned with its config schema:
# - dialect: sqlite | turso | postgresql
# - drivers: Rust drivers only (optional)

dialect = "sqlite"
# driver = "rusqlite"
schema = "src/schema.rs"
out = "./drizzle"
# breakpoints = true

[dbCredentials]
url = "./dev.db"
"#
            ))
        }
        "turso" => {
            if let Some(ref d) = driver
                && d != "libsql"
                && d != "turso"
            {
                return Err(CliError::Other(format!(
                    "Invalid driver for turso: {d}. Supported: libsql, turso"
                )));
            }
            Ok(format!(
                r#"#:schema {SCHEMA_URL}

# Drizzle Configuration (drizzle-rs)

dialect = "turso"
# driver = "libsql"   # local libsql (embedded)
# driver = "turso"    # remote Turso
schema = "src/schema.rs"
out = "./drizzle"
# breakpoints = true

[dbCredentials]
url = "libsql://your-db.turso.io"
authToken = "your-auth-token"
"#
            ))
        }
        "postgresql" | "postgres" => {
            if let Some(ref d) = driver
                && d != "postgres-sync"
                && d != "tokio-postgres"
            {
                return Err(CliError::Other(format!(
                    "Invalid driver for postgresql: {d}. Supported: postgres-sync, tokio-postgres"
                )));
            }
            Ok(format!(
                r#"#:schema {SCHEMA_URL}

# Drizzle Configuration (drizzle-rs)

dialect = "postgresql"
# driver = "postgres-sync"
# driver = "tokio-postgres"
schema = "src/schema.rs"
out = "./drizzle"
# breakpoints = true

[dbCredentials]
url = "postgres://user:password@localhost:5432/mydb"

# Or use individual connection fields:
# [dbCredentials]
# host = "localhost"
# port = 5432
# user = "postgres"
# password = "password"
# database = "mydb"
# ssl = true
"#
            ))
        }
        _ => Err(CliError::Other(format!(
            "Unknown dialect: {dialect}. Supported: sqlite, turso, postgresql"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use drizzle_cli::config::{Dialect, Driver, Extension, IntrospectCasing};

    #[test]
    fn parse_generate_parity_flags() {
        let cli = Cli::parse_from([
            "drizzle",
            "--db",
            "app",
            "generate",
            "--dialect",
            "postgres",
            "--driver",
            "postgres-sync",
            "--schema",
            "src/a.rs,src/b.rs",
            "--out",
            "drizzle_out",
            "--breakpoints",
            "false",
        ]);

        assert_eq!(cli.db.as_deref(), Some("app"));
        match cli.command {
            Command::Generate(opts) => {
                assert_eq!(opts.dialect, Some(Dialect::Postgresql));
                assert_eq!(opts.driver, Some(Driver::PostgresSync));
                assert_eq!(
                    opts.schema,
                    Some(vec!["src/a.rs".to_string(), "src/b.rs".to_string()])
                );
                assert_eq!(opts.breakpoints, Some(false));
            }
            _ => panic!("expected generate command"),
        }
    }

    #[test]
    fn parse_push_filters_and_connection_flags() {
        let cli = Cli::parse_from([
            "drizzle",
            "push",
            "--dialect",
            "postgresql",
            "--tablesFilter",
            "users_*,!users_tmp",
            "--schemaFilter",
            "public,!internal",
            "--extensionsFilters",
            "postgis",
            "--host",
            "localhost",
            "--database",
            "appdb",
            "--user",
            "postgres",
            "--password",
            "secret",
            "--ssl",
            "true",
        ]);

        match cli.command {
            Command::Push(opts) => {
                assert_eq!(opts.dialect, Some(Dialect::Postgresql));
                assert_eq!(
                    opts.filters.tables_filter,
                    Some(vec!["users_*".to_string(), "!users_tmp".to_string()])
                );
                assert_eq!(
                    opts.filters.schema_filters,
                    Some(vec!["public".to_string(), "!internal".to_string()])
                );
                assert_eq!(
                    opts.filters.extensions_filters,
                    Some(vec![Extension::Postgis])
                );
                assert_eq!(opts.connection.host.as_deref(), Some("localhost"));
                assert_eq!(opts.connection.database.as_deref(), Some("appdb"));
                assert_eq!(opts.connection.user.as_deref(), Some("postgres"));
                assert_eq!(opts.connection.password.as_deref(), Some("secret"));
                assert_eq!(opts.connection.ssl.as_deref(), Some("true"));
            }
            _ => panic!("expected push command"),
        }
    }

    #[test]
    fn parse_pull_alias_and_turso_flags() {
        let cli = Cli::parse_from([
            "drizzle",
            "pull",
            "--dialect",
            "turso",
            "--casing",
            "preserve",
            "--breakpoints",
            "true",
            "--url",
            "libsql://example.turso.io",
            "--authToken",
            "token",
        ]);

        match cli.command {
            Command::Pull(opts) => {
                assert_eq!(opts.dialect, Some(Dialect::Turso));
                assert_eq!(opts.casing, Some(IntrospectCasing::Preserve));
                assert_eq!(opts.breakpoints, Some(true));
                assert_eq!(
                    opts.connection.url.as_deref(),
                    Some("libsql://example.turso.io")
                );
                assert_eq!(opts.connection.auth_token.as_deref(), Some("token"));
            }
            _ => panic!("expected pull command"),
        }
    }

    #[test]
    fn parse_check_and_up_dialect_overrides() {
        let check_cli = Cli::parse_from(["drizzle", "check", "--dialect", "postgres"]);
        match check_cli.command {
            Command::Check(opts) => {
                assert_eq!(opts.dialect, Some(Dialect::Postgresql));
            }
            _ => panic!("expected check command"),
        }

        let up_cli = Cli::parse_from(["drizzle", "up", "--dialect", "sqlite"]);
        match up_cli.command {
            Command::Up(opts) => {
                assert_eq!(opts.dialect, Some(Dialect::Sqlite));
            }
            _ => panic!("expected up command"),
        }
    }

    #[test]
    fn push_strict_flag_is_rejected() {
        let err = Cli::try_parse_from(["drizzle", "push", "--strict"])
            .expect_err("--strict should be removed");
        let msg = err.to_string();
        assert!(msg.contains("--strict"));
        assert!(msg.contains("unexpected argument"));
    }

    #[test]
    fn parse_push_ssl_mode_string() {
        let cli = Cli::parse_from([
            "drizzle",
            "push",
            "--dialect",
            "postgresql",
            "--host",
            "localhost",
            "--database",
            "db",
            "--ssl",
            "require",
        ]);

        match cli.command {
            Command::Push(opts) => {
                assert_eq!(opts.connection.ssl.as_deref(), Some("require"));
            }
            _ => panic!("expected push command"),
        }
    }

    #[test]
    fn parse_migrate_modes() {
        let verify_cli = Cli::parse_from(["drizzle", "migrate", "--verify"]);
        match verify_cli.command {
            Command::Migrate(opts) => {
                assert!(opts.verify);
                assert!(!opts.plan);
                assert!(!opts.safe);
            }
            _ => panic!("expected migrate command"),
        }

        let safe_cli = Cli::parse_from(["drizzle", "migrate", "--safe"]);
        match safe_cli.command {
            Command::Migrate(opts) => {
                assert!(!opts.verify);
                assert!(!opts.plan);
                assert!(opts.safe);
            }
            _ => panic!("expected migrate command"),
        }
    }
}

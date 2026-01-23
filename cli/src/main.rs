//! Drizzle CLI - Main entry point
//!
//! This is the main binary for the drizzle-cli tool.
//! CLI interface matches drizzle-kit for TypeScript compatibility.

use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::process::ExitCode;

use drizzle_cli::config::{Casing, DrizzleConfig, IntrospectCasing};
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

/// CLI subcommands
///
/// These commands match drizzle-kit for TypeScript compatibility.
#[derive(Subcommand, Debug)]
enum Command {
    /// Generate a new migration from schema changes
    Generate {
        /// Migration name (optional, auto-generated if not provided)
        #[arg(short, long)]
        name: Option<String>,

        /// Create a custom (empty) migration file for manual SQL
        #[arg(long)]
        custom: bool,

        /// Casing for generated identifiers (camelCase or snake_case)
        #[arg(long, value_parser = parse_casing)]
        casing: Option<Casing>,
    },

    /// Run pending migrations
    Migrate,

    /// Upgrade migration snapshots to the latest version
    Up {
        /// Override dialect from config
        #[arg(long)]
        dialect: Option<String>,

        /// Override output directory
        #[arg(long)]
        out: Option<PathBuf>,
    },

    /// Push schema changes directly to database (without migration files)
    Push {
        /// Show all SQL statements that would be executed
        #[arg(long)]
        verbose: bool,

        /// Deprecated: use --explain instead
        #[arg(long, hide = true)]
        strict: bool,

        /// Force execution without warnings (auto-approve data-loss statements)
        #[arg(long)]
        force: bool,

        /// Print planned SQL changes without executing them (dry run)
        #[arg(long)]
        explain: bool,

        /// Casing for identifiers (camelCase or snake_case)
        #[arg(long, value_parser = parse_casing)]
        casing: Option<Casing>,

        /// Extensions filter (e.g., postgis)
        #[arg(long = "extensionsFilters", value_delimiter = ',')]
        extensions_filters: Option<Vec<String>>,
    },

    /// Introspect database and generate schema
    Introspect {
        /// Initialize migration metadata after introspecting
        #[arg(long, name = "init")]
        init_metadata: bool,

        /// Casing for introspected identifiers (camel or preserve)
        #[arg(long, value_parser = parse_introspect_casing)]
        casing: Option<IntrospectCasing>,

        /// Override output directory
        #[arg(long)]
        out: Option<PathBuf>,

        /// Override breakpoints setting
        #[arg(long)]
        breakpoints: Option<bool>,
    },

    /// Introspect database and generate schema (alias for introspect)
    Pull {
        /// Initialize migration metadata after introspecting
        #[arg(long, name = "init")]
        init_metadata: bool,

        /// Casing for introspected identifiers (camel or preserve)
        #[arg(long, value_parser = parse_introspect_casing)]
        casing: Option<IntrospectCasing>,

        /// Override output directory
        #[arg(long)]
        out: Option<PathBuf>,

        /// Override breakpoints setting
        #[arg(long)]
        breakpoints: Option<bool>,
    },

    /// Show migration status
    Status,

    /// Validate configuration file
    Check {
        /// Override dialect from config
        #[arg(long)]
        dialect: Option<String>,

        /// Override output directory
        #[arg(long)]
        out: Option<PathBuf>,
    },

    /// Export schema as SQL statements
    Export {
        /// Output SQL to a file (default: stdout)
        #[arg(long)]
        sql: Option<PathBuf>,
    },

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

/// Parse casing argument
fn parse_casing(s: &str) -> Result<Casing, String> {
    s.parse()
}

/// Parse introspect casing argument
fn parse_introspect_casing(s: &str) -> Result<IntrospectCasing, String> {
    s.parse()
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
    let db_name = cli.db.as_deref();

    match cli.command {
        Command::Init { dialect, driver } => run_init(&dialect, driver.as_deref()),
        Command::Generate {
            name,
            custom,
            casing,
        } => {
            let config = load_config(cli.config.as_deref())?;
            drizzle_cli::commands::generate::run(&config, db_name, name, custom, casing)
        }
        Command::Migrate => {
            let config = load_config(cli.config.as_deref())?;
            drizzle_cli::commands::migrate::run(&config, db_name)
        }
        Command::Up { dialect, out } => {
            let config = load_config(cli.config.as_deref())?;
            drizzle_cli::commands::upgrade::run(
                &config,
                db_name,
                dialect.as_deref(),
                out.as_deref(),
            )
        }
        Command::Push {
            verbose,
            strict,
            force,
            explain,
            casing,
            extensions_filters,
        } => {
            let config = load_config(cli.config.as_deref())?;
            drizzle_cli::commands::push::run(
                &config,
                db_name,
                drizzle_cli::commands::push::PushOptions {
                    cli_verbose: verbose,
                    cli_strict: strict,
                    force,
                    cli_explain: explain,
                    casing,
                    extensions_filters,
                },
            )
        }
        Command::Introspect {
            init_metadata,
            casing,
            out,
            breakpoints,
        }
        | Command::Pull {
            init_metadata,
            casing,
            out,
            breakpoints,
        } => {
            let config = load_config(cli.config.as_deref())?;
            drizzle_cli::commands::introspect::run(
                &config,
                db_name,
                init_metadata,
                casing,
                out.as_deref(),
                breakpoints,
            )
        }
        Command::Status => {
            let config = load_config(cli.config.as_deref())?;
            drizzle_cli::commands::status::run(&config, db_name)
        }
        Command::Check { dialect, out } => {
            let config = load_config(cli.config.as_deref())?;
            drizzle_cli::commands::check::run(&config, db_name, dialect.as_deref(), out.as_deref())
        }
        Command::Export { sql } => {
            let config = load_config(cli.config.as_deref())?;
            drizzle_cli::commands::export::run(&config, db_name, sql)
        }
    }
}

/// Load configuration with fallback to default path
fn load_config(custom_path: Option<&std::path::Path>) -> Result<DrizzleConfig, CliError> {
    match custom_path {
        Some(path) => {
            // If the user points at a config in a different directory, also try to load a `.env`
            // next to that config before resolving `{ env = "NAME" }` values.
            if let Some(dir) = path.parent() {
                let _ = dotenvy::from_path(dir.join(".env"));
            }
            DrizzleConfig::load_from(path).map_err(Into::into)
        }
        None => DrizzleConfig::load().map_err(Into::into),
    }
}

/// Initialize a new drizzle.config.toml file
fn run_init(dialect: &str, driver: Option<&str>) -> Result<(), CliError> {
    let config_path = PathBuf::from(DEFAULT_CONFIG_FILE);

    if config_path.exists() {
        return Err(CliError::Other(format!(
            "{} already exists. Delete it first to reinitialize.",
            DEFAULT_CONFIG_FILE
        )));
    }

    let config_content = generate_init_config(dialect, driver)?;

    std::fs::write(&config_path, config_content).map_err(|e| CliError::IoError(e.to_string()))?;

    println!(
        "{}",
        output::success(&format!("Created {}", DEFAULT_CONFIG_FILE))
    );
    println!();
    println!("Next steps:");
    println!(
        "  1. Edit {} with your database credentials",
        DEFAULT_CONFIG_FILE
    );
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
    let driver = driver.map(|d| d.to_lowercase());

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
                r#"#:schema {}

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
"#,
                SCHEMA_URL
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
                r#"#:schema {}

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
"#,
                SCHEMA_URL
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
                r#"#:schema {}

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
"#,
                SCHEMA_URL
            ))
        }
        _ => Err(CliError::Other(format!(
            "Unknown dialect: {dialect}. Supported: sqlite, turso, postgresql"
        ))),
    }
}

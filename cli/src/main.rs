//! Drizzle CLI - Main entry point
//!
//! This is the main binary for the drizzle-cli tool.
//! CLI interface matches drizzle-kit for TypeScript compatibility.

use clap::{Args, Parser, Subcommand};
use std::path::PathBuf;
use std::process::ExitCode;

use drizzle_cli::config::{Casing, Config, Dialect, Driver, Extension, IntrospectCasing};
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

#[derive(Args, Debug, Clone, Default)]
struct ConnectionArgs {
    /// Database connection URL
    #[arg(long)]
    url: Option<String>,

    /// Database host
    #[arg(long)]
    host: Option<String>,

    /// Database port
    #[arg(long)]
    port: Option<u16>,

    /// Database user
    #[arg(long)]
    user: Option<String>,

    /// Database password
    #[arg(long)]
    password: Option<String>,

    /// Database name
    #[arg(long)]
    database: Option<String>,

    /// SSL mode (true/false or require/prefer/verify-full/disable)
    #[arg(long)]
    ssl: Option<String>,

    /// Turso auth token
    #[arg(long = "authToken", alias = "auth-token")]
    auth_token: Option<String>,
}

#[derive(Args, Debug, Clone, Default)]
struct FilterArgs {
    /// Table name filters
    #[arg(long = "tablesFilter", value_delimiter = ',')]
    tables_filter: Option<Vec<String>>,

    /// Schema name filters
    #[arg(long = "schemaFilters", alias = "schemaFilter", value_delimiter = ',')]
    schema_filters: Option<Vec<String>>,

    /// Extension filters (e.g. postgis)
    #[arg(long = "extensionsFilters", value_delimiter = ',', value_parser = parse_extension)]
    extensions_filters: Option<Vec<Extension>>,
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

        /// Override dialect from config
        #[arg(long, value_parser = parse_dialect)]
        dialect: Option<Dialect>,

        /// Override driver from config
        #[arg(long, value_parser = parse_driver)]
        driver: Option<Driver>,

        /// Override schema path(s)
        #[arg(long, value_delimiter = ',')]
        schema: Option<Vec<String>>,

        /// Override output directory
        #[arg(long)]
        out: Option<PathBuf>,

        /// Override breakpoints setting
        #[arg(long)]
        breakpoints: Option<bool>,
    },

    /// Run pending migrations
    Migrate {
        /// Verify migration consistency without applying changes
        #[arg(long)]
        verify: bool,

        /// Print pending migration plan without applying changes
        #[arg(long)]
        plan: bool,

        /// Verify first, then apply if checks pass
        #[arg(long)]
        safe: bool,
    },

    /// Upgrade migration snapshots to the latest version
    Up {
        /// Override dialect from config
        #[arg(long, value_parser = parse_dialect)]
        dialect: Option<Dialect>,

        /// Override output directory
        #[arg(long)]
        out: Option<PathBuf>,
    },

    /// Push schema changes directly to database (without migration files)
    Push {
        /// Show all SQL statements that would be executed
        #[arg(long)]
        verbose: bool,

        /// Force execution without warnings (auto-approve data-loss statements)
        #[arg(long)]
        force: bool,

        /// Print planned SQL changes without executing them (dry run)
        #[arg(long)]
        explain: bool,

        /// Casing for identifiers (camelCase or snake_case)
        #[arg(long, value_parser = parse_casing)]
        casing: Option<Casing>,

        /// Override dialect from config
        #[arg(long, value_parser = parse_dialect)]
        dialect: Option<Dialect>,

        /// Override schema path(s)
        #[arg(long, value_delimiter = ',')]
        schema: Option<Vec<String>>,

        #[command(flatten)]
        filters: FilterArgs,

        #[command(flatten)]
        connection: ConnectionArgs,
    },

    /// Introspect database and generate schema
    Introspect {
        /// Initialize migration metadata after introspecting
        #[arg(long = "init")]
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

        /// Override dialect from config
        #[arg(long, value_parser = parse_dialect)]
        dialect: Option<Dialect>,

        #[command(flatten)]
        filters: FilterArgs,

        #[command(flatten)]
        connection: ConnectionArgs,
    },

    /// Introspect database and generate schema (alias for introspect)
    Pull {
        /// Initialize migration metadata after introspecting
        #[arg(long = "init")]
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

        /// Override dialect from config
        #[arg(long, value_parser = parse_dialect)]
        dialect: Option<Dialect>,

        #[command(flatten)]
        filters: FilterArgs,

        #[command(flatten)]
        connection: ConnectionArgs,
    },

    /// Show migration status
    Status,

    /// Validate configuration file
    Check {
        /// Override dialect from config
        #[arg(long, value_parser = parse_dialect)]
        dialect: Option<Dialect>,

        /// Override output directory
        #[arg(long)]
        out: Option<PathBuf>,
    },

    /// Export schema as SQL statements
    Export {
        /// Output SQL to a file (default: stdout)
        #[arg(long)]
        sql: Option<PathBuf>,

        /// Override dialect from config
        #[arg(long, value_parser = parse_dialect)]
        dialect: Option<Dialect>,

        /// Override schema path(s)
        #[arg(long, value_delimiter = ',')]
        schema: Option<Vec<String>>,
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

/// Parse dialect argument
fn parse_dialect(s: &str) -> Result<Dialect, String> {
    s.parse()
}

/// Parse driver argument
fn parse_driver(s: &str) -> Result<Driver, String> {
    s.parse()
}

/// Parse extension filter argument
fn parse_extension(s: &str) -> Result<Extension, String> {
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
            dialect,
            driver,
            schema,
            out,
            breakpoints,
        } => {
            let config = load_config(cli.config.as_deref())?;
            drizzle_cli::commands::generate::run(
                &config,
                db_name,
                drizzle_cli::commands::generate::GenerateOptions {
                    name,
                    custom,
                    casing,
                    dialect,
                    driver,
                    schema,
                    out,
                    breakpoints,
                },
            )
        }
        Command::Migrate { verify, plan, safe } => {
            let config = load_config(cli.config.as_deref())?;
            drizzle_cli::commands::migrate::run(
                &config,
                db_name,
                drizzle_cli::commands::migrate::MigrateOptions { verify, plan, safe },
            )
        }
        Command::Up { dialect, out } => {
            let config = load_config(cli.config.as_deref())?;
            drizzle_cli::commands::upgrade::run(&config, db_name, dialect, out.as_deref())
        }
        Command::Push {
            verbose,
            force,
            explain,
            casing,
            dialect,
            schema,
            filters,
            connection,
        } => {
            let config = load_config(cli.config.as_deref())?;
            drizzle_cli::commands::push::run(
                &config,
                db_name,
                drizzle_cli::commands::push::PushOptions {
                    cli_verbose: verbose,
                    force,
                    cli_explain: explain,
                    casing,
                    dialect,
                    schema,
                    tables_filters: filters.tables_filter,
                    schema_filters: filters.schema_filters,
                    extensions_filters: filters.extensions_filters,
                    connection: drizzle_cli::commands::overrides::ConnectionOverrides {
                        url: connection.url,
                        host: connection.host,
                        port: connection.port,
                        user: connection.user,
                        password: connection.password,
                        database: connection.database,
                        ssl: connection.ssl,
                        auth_token: connection.auth_token,
                    },
                },
            )
        }
        Command::Introspect {
            init_metadata,
            casing,
            out,
            breakpoints,
            dialect,
            filters,
            connection,
        }
        | Command::Pull {
            init_metadata,
            casing,
            out,
            breakpoints,
            dialect,
            filters,
            connection,
        } => {
            let config = load_config(cli.config.as_deref())?;
            drizzle_cli::commands::introspect::run(
                &config,
                db_name,
                drizzle_cli::commands::introspect::IntrospectOptions {
                    init_metadata,
                    casing,
                    out,
                    breakpoints,
                    dialect,
                    tables_filters: filters.tables_filter,
                    schema_filters: filters.schema_filters,
                    extensions_filters: filters.extensions_filters,
                    connection: drizzle_cli::commands::overrides::ConnectionOverrides {
                        url: connection.url,
                        host: connection.host,
                        port: connection.port,
                        user: connection.user,
                        password: connection.password,
                        database: connection.database,
                        ssl: connection.ssl,
                        auth_token: connection.auth_token,
                    },
                },
            )
        }
        Command::Status => {
            let config = load_config(cli.config.as_deref())?;
            drizzle_cli::commands::status::run(&config, db_name)
        }
        Command::Check { dialect, out } => {
            let config = load_config(cli.config.as_deref())?;
            drizzle_cli::commands::check::run(&config, db_name, dialect, out.as_deref())
        }
        Command::Export {
            sql,
            dialect,
            schema,
        } => {
            let config = load_config(cli.config.as_deref())?;
            drizzle_cli::commands::export::run(
                &config,
                db_name,
                drizzle_cli::commands::export::ExportOptions {
                    output_path: sql,
                    dialect,
                    schema,
                },
            )
        }
    }
}

/// Load configuration with fallback to default path
fn load_config(custom_path: Option<&std::path::Path>) -> Result<Config, CliError> {
    match custom_path {
        Some(path) => {
            // If the user points at a config in a different directory, also try to load a `.env`
            // next to that config before resolving `{ env = "NAME" }` values.
            if let Some(dir) = path.parent() {
                let _ = dotenvy::from_path(dir.join(".env"));
            }
            Config::load_from(path).map_err(Into::into)
        }
        None => Config::load().map_err(Into::into),
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

#[cfg(test)]
mod tests {
    use super::*;

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
            Command::Generate {
                dialect,
                driver,
                schema,
                breakpoints,
                ..
            } => {
                assert_eq!(dialect, Some(Dialect::Postgresql));
                assert_eq!(driver, Some(Driver::PostgresSync));
                assert_eq!(
                    schema,
                    Some(vec!["src/a.rs".to_string(), "src/b.rs".to_string()])
                );
                assert_eq!(breakpoints, Some(false));
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
            Command::Push {
                dialect,
                filters,
                connection,
                ..
            } => {
                assert_eq!(dialect, Some(Dialect::Postgresql));
                assert_eq!(
                    filters.tables_filter,
                    Some(vec!["users_*".to_string(), "!users_tmp".to_string()])
                );
                assert_eq!(
                    filters.schema_filters,
                    Some(vec!["public".to_string(), "!internal".to_string()])
                );
                assert_eq!(filters.extensions_filters, Some(vec![Extension::Postgis]));
                assert_eq!(connection.host.as_deref(), Some("localhost"));
                assert_eq!(connection.database.as_deref(), Some("appdb"));
                assert_eq!(connection.user.as_deref(), Some("postgres"));
                assert_eq!(connection.password.as_deref(), Some("secret"));
                assert_eq!(connection.ssl.as_deref(), Some("true"));
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
            Command::Pull {
                dialect,
                casing,
                breakpoints,
                connection,
                ..
            } => {
                assert_eq!(dialect, Some(Dialect::Turso));
                assert_eq!(casing, Some(IntrospectCasing::Preserve));
                assert_eq!(breakpoints, Some(true));
                assert_eq!(connection.url.as_deref(), Some("libsql://example.turso.io"));
                assert_eq!(connection.auth_token.as_deref(), Some("token"));
            }
            _ => panic!("expected pull command"),
        }
    }

    #[test]
    fn parse_check_and_up_dialect_overrides() {
        let check_cli = Cli::parse_from(["drizzle", "check", "--dialect", "postgres"]);
        match check_cli.command {
            Command::Check { dialect, .. } => {
                assert_eq!(dialect, Some(Dialect::Postgresql));
            }
            _ => panic!("expected check command"),
        }

        let up_cli = Cli::parse_from(["drizzle", "up", "--dialect", "sqlite"]);
        match up_cli.command {
            Command::Up { dialect, .. } => {
                assert_eq!(dialect, Some(Dialect::Sqlite));
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
            Command::Push { connection, .. } => {
                assert_eq!(connection.ssl.as_deref(), Some("require"));
            }
            _ => panic!("expected push command"),
        }
    }

    #[test]
    fn parse_migrate_modes() {
        let verify_cli = Cli::parse_from(["drizzle", "migrate", "--verify"]);
        match verify_cli.command {
            Command::Migrate { verify, plan, safe } => {
                assert!(verify);
                assert!(!plan);
                assert!(!safe);
            }
            _ => panic!("expected migrate command"),
        }

        let safe_cli = Cli::parse_from(["drizzle", "migrate", "--safe"]);
        match safe_cli.command {
            Command::Migrate { verify, plan, safe } => {
                assert!(!verify);
                assert!(!plan);
                assert!(safe);
            }
            _ => panic!("expected migrate command"),
        }
    }
}

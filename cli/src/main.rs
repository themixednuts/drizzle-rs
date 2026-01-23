//! Drizzle CLI - Main entry point
//!
//! This is the main binary for the drizzle-cli tool.
//! CLI interface matches drizzle-kit for TypeScript compatibility.

use clap::{Parser, Subcommand};
use colored::Colorize;
use std::path::PathBuf;
use std::process::ExitCode;

use drizzle_cli::config::{Casing, DrizzleConfig, IntrospectCasing};
use drizzle_cli::error::CliError;

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
        /// Database dialect (sqlite, postgresql, mysql, turso, singlestore)
        #[arg(short, long, default_value = "sqlite")]
        dialect: String,

        /// Database driver (d1-http, expo, aws-data-api, pglite, sqlite-cloud, durable-sqlite)
        #[arg(short = 'r', long)]
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
            eprintln!("{} {}", "Error:".red().bold(), e);
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
                verbose,
                strict,
                force,
                explain,
                casing,
                extensions_filters,
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
        Some(path) => DrizzleConfig::load_from(path).map_err(Into::into),
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
        format!("✅ Created {}", DEFAULT_CONFIG_FILE).bright_green()
    );
    println!();
    println!("Next steps:");
    println!(
        "  1. Edit {} with your database credentials",
        DEFAULT_CONFIG_FILE
    );
    println!(
        "  2. Create your schema file at {}",
        "src/schema.rs".bright_cyan()
    );
    println!(
        "  3. Run {} to generate your first migration",
        "drizzle generate".bright_cyan()
    );

    Ok(())
}

/// Generate the init configuration content based on dialect and driver
fn generate_init_config(dialect: &str, driver: Option<&str>) -> Result<String, CliError> {
    // Handle driver-specific configurations
    if let Some(drv) = driver {
        return match drv.to_lowercase().as_str() {
            "d1-http" => Ok(format!(
                r#"#:schema {}

# Drizzle Configuration - Cloudflare D1
# See: https://orm.drizzle.team/kit-docs/config-reference

dialect = "sqlite"
driver = "d1-http"
schema = "src/schema.rs"
out = "./drizzle"

[dbCredentials]
accountId = "your-cloudflare-account-id"
databaseId = "your-d1-database-id"
token = "your-cloudflare-api-token"
"#,
                SCHEMA_URL
            )),
            "aws-data-api" => Ok(format!(
                r#"#:schema {}

# Drizzle Configuration - AWS RDS Data API
# See: https://orm.drizzle.team/kit-docs/config-reference

dialect = "postgresql"
driver = "aws-data-api"
schema = "src/schema.rs"
out = "./drizzle"

[dbCredentials]
database = "your-database-name"
secretArn = "arn:aws:secretsmanager:region:account:secret:name"
resourceArn = "arn:aws:rds:region:account:cluster:name"
"#,
                SCHEMA_URL
            )),
            "pglite" => Ok(format!(
                r#"#:schema {}

# Drizzle Configuration - PGlite
# See: https://orm.drizzle.team/kit-docs/config-reference

dialect = "postgresql"
driver = "pglite"
schema = "src/schema.rs"
out = "./drizzle"

[dbCredentials]
url = "./dev.db"
"#,
                SCHEMA_URL
            )),
            "sqlite-cloud" => Ok(format!(
                r#"#:schema {}

# Drizzle Configuration - SQLite Cloud
# See: https://orm.drizzle.team/kit-docs/config-reference

dialect = "sqlite"
driver = "sqlite-cloud"
schema = "src/schema.rs"
out = "./drizzle"

[dbCredentials]
url = "sqlitecloud://your-host.sqlite.cloud:8860/your-database?apikey=your-api-key"
"#,
                SCHEMA_URL
            )),
            "expo" => Ok(format!(
                r#"#:schema {}

# Drizzle Configuration - Expo SQLite
# See: https://orm.drizzle.team/kit-docs/config-reference
# Note: Expo driver is for React Native and not supported in drizzle-kit CLI

dialect = "sqlite"
driver = "expo"
schema = "src/schema.rs"
out = "./drizzle"

[dbCredentials]
url = "./dev.db"
"#,
                SCHEMA_URL
            )),
            "durable-sqlite" => Ok(format!(
                r#"#:schema {}

# Drizzle Configuration - Cloudflare Durable Objects SQLite
# See: https://orm.drizzle.team/kit-docs/config-reference
# Note: Durable SQLite driver is not supported in drizzle-kit CLI

dialect = "sqlite"
driver = "durable-sqlite"
schema = "src/schema.rs"
out = "./drizzle"

[dbCredentials]
url = "./dev.db"
"#,
                SCHEMA_URL
            )),
            _ => Err(CliError::Other(format!(
                "Unknown driver: {}. Valid drivers: d1-http, aws-data-api, pglite, sqlite-cloud, expo, durable-sqlite",
                drv
            ))),
        };
    }

    // Handle dialect-only configurations
    match dialect.to_lowercase().as_str() {
        "sqlite" => Ok(format!(
            r#"#:schema {}

# Drizzle Configuration
# See: https://orm.drizzle.team/kit-docs/config-reference

dialect = "sqlite"
schema = "src/schema.rs"
out = "./drizzle"
# breakpoints = true

[dbCredentials]
url = "./dev.db"
"#,
            SCHEMA_URL
        )),
        "turso" => Ok(format!(
            r#"#:schema {}

# Drizzle Configuration
# See: https://orm.drizzle.team/kit-docs/config-reference

dialect = "turso"
schema = "src/schema.rs"
out = "./drizzle"
# breakpoints = true

[dbCredentials]
url = "libsql://your-db.turso.io"
authToken = "your-auth-token"
"#,
            SCHEMA_URL
        )),
        "postgresql" | "postgres" => Ok(format!(
            r#"#:schema {}

# Drizzle Configuration
# See: https://orm.drizzle.team/kit-docs/config-reference

dialect = "postgresql"
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
        )),
        "mysql" => Ok(format!(
            r#"#:schema {}

# Drizzle Configuration
# See: https://orm.drizzle.team/kit-docs/config-reference

dialect = "mysql"
schema = "src/schema.rs"
out = "./drizzle"
# breakpoints = true

[dbCredentials]
url = "mysql://user:password@localhost:3306/mydb"

# Or use individual connection fields:
# [dbCredentials]
# host = "localhost"
# port = 3306
# user = "root"
# password = "password"
# database = "mydb"
"#,
            SCHEMA_URL
        )),
        "singlestore" => Ok(format!(
            r#"#:schema {}

# Drizzle Configuration - SingleStore
# See: https://orm.drizzle.team/kit-docs/config-reference

dialect = "singlestore"
schema = "src/schema.rs"
out = "./drizzle"
# breakpoints = true

[dbCredentials]
url = "mysql://user:password@localhost:3306/mydb"

# Or use individual connection fields:
# [dbCredentials]
# host = "localhost"
# port = 3306
# user = "root"
# password = "password"
# database = "mydb"
"#,
            SCHEMA_URL
        )),
        _ => Err(CliError::Other(format!(
            "Unknown dialect: {}. Valid dialects: sqlite, turso, postgresql, mysql, singlestore",
            dialect
        ))),
    }
}

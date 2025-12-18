//! Drizzle CLI - Main entry point
//!
//! This is the main binary for the drizzle-cli tool.

use clap::{Parser, Subcommand};
use colored::Colorize;
use std::path::PathBuf;
use std::process::ExitCode;

use drizzle_cli::config::DrizzleConfig;
use drizzle_cli::error::CliError;

/// Default configuration file name
const DEFAULT_CONFIG_FILE: &str = "drizzle.config.toml";

/// JSON schema URL for TOML validation
const SCHEMA_URL: &str =
    "https://raw.githubusercontent.com/themixednuts/drizzle-rs/master/cli/schema.json";

/// Drizzle - Database migration CLI for drizzle-rs
#[derive(Parser, Debug)]
#[command(name = "drizzle")]
#[command(author, version, about = "Database migration CLI for drizzle-rs", long_about = None)]
struct Cli {
    /// Path to config file (default: drizzle.config.toml)
    #[arg(short, long, global = true, value_name = "PATH")]
    config: Option<PathBuf>,

    #[command(subcommand)]
    command: Command,
}

/// CLI subcommands
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
    },

    /// Run pending migrations
    Migrate,

    /// Push schema changes directly to database (without migration files)
    Push,

    /// Introspect database and generate snapshot
    Introspect,

    /// Show migration status
    Status,

    /// Initialize a new drizzle.toml configuration file
    Init {
        /// Database dialect (sqlite or postgresql)
        #[arg(short, long, default_value = "sqlite")]
        dialect: String,

        /// Database driver
        #[arg(short = 'r', long)]
        driver: Option<String>,
    },
}

fn main() -> ExitCode {
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
    match cli.command {
        Command::Init { dialect, driver } => run_init(&dialect, driver.as_deref()),
        Command::Generate { name, custom } => {
            let config = load_config(cli.config.as_deref())?;
            drizzle_cli::commands::generate::run(&config, name, custom)
        }
        Command::Migrate => {
            let config = load_config(cli.config.as_deref())?;
            drizzle_cli::commands::migrate::run(&config)
        }
        Command::Push => {
            let config = load_config(cli.config.as_deref())?;
            drizzle_cli::commands::push::run(&config)
        }
        Command::Introspect => {
            let config = load_config(cli.config.as_deref())?;
            drizzle_cli::commands::introspect::run(&config)
        }
        Command::Status => {
            let config = load_config(cli.config.as_deref())?;
            drizzle_cli::commands::status::run(&config)
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
fn run_init(dialect: &str, _driver: Option<&str>) -> Result<(), CliError> {
    let config_path = PathBuf::from(DEFAULT_CONFIG_FILE);

    if config_path.exists() {
        return Err(CliError::Other(format!(
            "{} already exists. Delete it first to reinitialize.",
            DEFAULT_CONFIG_FILE
        )));
    }

    let config_content = match dialect.to_lowercase().as_str() {
        "sqlite" => format!(
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
        ),
        "turso" => format!(
            r#"#:schema {}

# Drizzle Configuration
# See: https://orm.drizzle.team/kit-docs/config-reference

dialect = "turso"
schema = "src/schema.rs"
out = "./drizzle"
# breakpoints = true

[dbCredentials]
url = "libsql://your-db.turso.io"
authToken = "your-token"
"#,
            SCHEMA_URL
        ),
        "postgresql" | "postgres" => format!(
            r#"#:schema {}

# Drizzle Configuration
# See: https://orm.drizzle.team/kit-docs/config-reference

dialect = "postgresql"
schema = "src/schema.rs"
out = "./drizzle"
# breakpoints = true

[dbCredentials]
url = "postgres://user:password@localhost:5432/mydb"
# Or use individual fields:
# host = "localhost"
# port = 5432
# user = "user"
# password = "password"
# database = "mydb"
"#,
            SCHEMA_URL
        ),
        "mysql" => format!(
            r#"#:schema {}

# Drizzle Configuration
# See: https://orm.drizzle.team/kit-docs/config-reference

dialect = "mysql"
schema = "src/schema.rs"
out = "./drizzle"
# breakpoints = true

[dbCredentials]
url = "mysql://user:password@localhost:3306/mydb"
"#,
            SCHEMA_URL
        ),
        _ => return Err(CliError::Other(format!("Unknown dialect: {}", dialect))),
    };

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

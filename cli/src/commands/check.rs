//! Check command - validates configuration

use colored::Colorize;

use crate::config::{Config, Credentials, PostgresCreds, MysqlCreds};
use crate::error::CliError;

pub fn run(config: &Config) -> Result<(), CliError> {
    println!("{}", "Checking configuration...".bright_cyan());
    println!();

    let mut warnings = Vec::new();
    let mut has_errors = false;

    // Basic info
    println!("  {}: {}", "Dialect".bright_blue(), config.dialect);
    if let Some(driver) = config.driver {
        println!("  {}: {}", "Driver".bright_blue(), driver);
    }
    println!("  {}: {}", "Schema".bright_blue(), config.schema_display());
    println!("  {}: {}", "Output".bright_blue(), config.out.display());

    // Schema files
    println!();
    print!("  {} Schema files... ", "Checking".bright_blue());
    match config.schema_files() {
        Ok(files) => {
            println!("{}", "OK".green());
            for f in &files {
                println!("    {}", f.display());
            }
        }
        Err(e) => {
            println!("{}", "ERROR".red());
            println!("    {e}");
            has_errors = true;
        }
    }

    // Migrations dir
    println!();
    print!("  {} Migrations... ", "Checking".bright_blue());
    let dir = config.migrations_dir();
    if dir.exists() {
        println!("{}", "OK".green());
        if config.journal_path().exists() {
            println!("    Journal: {}", "found".green());
        } else {
            println!("    Journal: {} (run generate first)", "missing".yellow());
            warnings.push("No migration journal");
        }
    } else {
        println!("{}", "NOT CREATED".yellow());
        warnings.push("Migrations directory doesn't exist yet");
    }

    // Credentials
    println!();
    print!("  {} Credentials... ", "Checking".bright_blue());
    match config.credentials() {
        Some(creds) => {
            println!("{}", "OK".green());
            print_credentials(&creds);
        }
        None => {
            println!("{}", "NOT SET".yellow());
            warnings.push("No credentials (needed for push/pull/migrate)");
        }
    }

    // Summary
    println!();
    if has_errors {
        println!("{}", "Configuration has errors.".red().bold());
        Err(CliError::Other("config check failed".into()))
    } else if warnings.is_empty() {
        println!("{}", "Configuration OK.".green().bold());
        Ok(())
    } else {
        println!("{}", format!("{} warning(s):", warnings.len()).yellow());
        for w in warnings {
            println!("  - {w}");
        }
        Ok(())
    }
}

fn print_credentials(creds: &Credentials) {
    match creds {
        Credentials::Sqlite { path } => {
            println!("    SQLite: {path}");
        }
        Credentials::Turso { url, auth_token } => {
            println!("    Turso: {}", mask_url(url));
            if auth_token.is_some() {
                println!("    Token: ****");
            }
        }
        Credentials::Postgres(pg) => match pg {
            PostgresCreds::Url(url) => println!("    PostgreSQL: {}", mask_url(url)),
            PostgresCreds::Host { host, port, database, user, .. } => {
                println!("    PostgreSQL: {host}:{port}/{database}");
                if let Some(u) = user {
                    println!("    User: {u}");
                }
            }
        },
        Credentials::Mysql(my) => match my {
            MysqlCreds::Url(url) => println!("    MySQL: {}", mask_url(url)),
            MysqlCreds::Host { host, port, database, user, .. } => {
                println!("    MySQL: {host}:{port}/{database}");
                if let Some(u) = user {
                    println!("    User: {u}");
                }
            }
        },
        Credentials::D1 { account_id, database_id, .. } => {
            println!("    D1: {}.../{}", &account_id[..8.min(account_id.len())], database_id);
        }
        Credentials::AwsDataApi { database, .. } => {
            println!("    AWS Data API: {database}");
        }
        Credentials::Pglite { path } => {
            println!("    PGlite: {path}");
        }
        Credentials::SqliteCloud { url } => {
            println!("    SQLite Cloud: {}", mask_url(url));
        }
    }
}

fn mask_url(url: &str) -> String {
    if let Some(at) = url.find('@') {
        if let Some(colon) = url[..at].rfind(':') {
            let scheme_end = url.find("://").map(|p| p + 3).unwrap_or(0);
            if colon > scheme_end {
                return format!("{}****{}", &url[..colon + 1], &url[at..]);
            }
        }
    }
    url.to_string()
}

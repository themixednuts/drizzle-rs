//! Check command - validates configuration

use std::path::Path;

use crate::config::{Config, Credentials, Dialect, PostgresCreds};
use crate::error::CliError;
use crate::output;

pub fn run(
    config: &Config,
    db_name: Option<&str>,
    dialect_override: Option<Dialect>,
    out_override: Option<&Path>,
) -> Result<(), CliError> {
    let db = config.database(db_name)?;

    // CLI flags override config
    let effective_dialect = dialect_override.unwrap_or(db.dialect);
    let effective_out = out_override
        .map(Path::to_path_buf)
        .unwrap_or(db.out.clone());

    println!("{}", output::heading("Checking configuration..."));
    println!();

    if !config.is_single_database() {
        let name = db_name.unwrap_or("(default)");
        println!("  {}: {}", output::label("Database"), name);
    }

    let mut warnings = Vec::new();
    let mut has_errors = false;

    // Basic info
    println!("  {}: {}", output::label("Dialect"), effective_dialect);
    if let Some(driver) = db.driver {
        println!("  {}: {}", output::label("Driver"), driver);
    }
    println!("  {}: {}", output::label("Schema"), db.schema_display());
    println!("  {}: {}", output::label("Output"), effective_out.display());

    // Schema files
    println!();
    print!("  {} Schema files... ", output::label("Checking"));
    match db.schema_files() {
        Ok(files) => {
            println!("{}", output::status_ok());
            for f in &files {
                println!("    {}", f.display());
            }
        }
        Err(e) => {
            println!("{}", output::status_error());
            println!("    {e}");
            has_errors = true;
        }
    }

    // Migrations dir
    println!();
    print!("  {} Migrations... ", output::label("Checking"));
    let dir = &effective_out;
    let journal_path = effective_out.join("meta").join("_journal.json");
    if dir.exists() {
        println!("{}", output::status_ok());
        let migration_count = count_migration_dirs(dir)?;
        if migration_count > 0 {
            println!(
                "    Folders: {}",
                output::success(&migration_count.to_string())
            );
        } else {
            println!(
                "    Folders: {} (run generate first)",
                output::warning("missing")
            );
            warnings.push("No migration folders");
        }

        if journal_path.exists() {
            println!(
                "    Legacy journal: {} (run upgrade)",
                output::warning("found")
            );
            warnings.push("Legacy migration journal detected");
        }
    } else {
        println!("{}", output::status_warning("NOT CREATED"));
        warnings.push("Migrations directory doesn't exist yet");
    }

    // Credentials
    println!();
    print!("  {} Credentials... ", output::label("Checking"));
    match db.credentials() {
        Ok(Some(creds)) => {
            println!("{}", output::status_ok());
            print_credentials(&creds);
        }
        Ok(None) => {
            println!("{}", output::status_warning("NOT SET"));
            warnings.push("No credentials (needed for push/pull/migrate)");
        }
        Err(e) => {
            println!("{}", output::status_error());
            println!("    {}", e);
            has_errors = true;
        }
    }

    // Summary
    println!();
    if has_errors {
        println!("{}", output::error("Configuration has errors."));
        Err(CliError::Other("config check failed".into()))
    } else if warnings.is_empty() {
        println!("{}", output::success("Configuration OK."));
        Ok(())
    } else {
        println!(
            "{}",
            output::warning(&format!("{} warning(s):", warnings.len()))
        );
        for w in warnings {
            println!("  - {w}");
        }
        Ok(())
    }
}

fn count_migration_dirs(dir: &Path) -> Result<usize, CliError> {
    let mut count = 0usize;
    for entry in std::fs::read_dir(dir).map_err(|e| CliError::IoError(e.to_string()))? {
        let entry = entry.map_err(|e| CliError::IoError(e.to_string()))?;
        if !entry
            .file_type()
            .map_err(|e| CliError::IoError(e.to_string()))?
            .is_dir()
        {
            continue;
        }

        let tag = entry.file_name().to_string_lossy().to_string();
        if tag == "meta" {
            continue;
        }

        if entry.path().join("migration.sql").exists() {
            count += 1;
        }
    }

    Ok(count)
}

fn print_credentials(creds: &Credentials) {
    match creds {
        Credentials::Sqlite { path } => {
            println!("    {}: {path}", output::label("SQLite"));
        }
        Credentials::Turso { url, auth_token } => {
            println!("    {}: {}", output::label("Turso"), mask_url(url));
            if auth_token.is_some() {
                println!("    Token: ****");
            }
        }
        Credentials::Postgres(pg) => match pg {
            PostgresCreds::Url(url) => {
                println!("    {}: {}", output::label("PostgreSQL"), mask_url(url))
            }
            PostgresCreds::Host {
                host,
                port,
                database,
                user,
                ..
            } => {
                println!(
                    "    {}: {host}:{port}/{database}",
                    output::label("PostgreSQL")
                );
                if let Some(u) = user {
                    println!("    User: {u}");
                }
            }
        },
        Credentials::D1 {
            account_id,
            database_id,
            ..
        } => {
            // Mask the token (never print it) and show just the IDs.
            println!("    {}: {account_id}/{database_id}", output::label("D1"));
            println!("    Token: ****");
        }
        Credentials::AwsDataApi { database, .. } => {
            // Mask both ARNs — drizzle-kit flags both secretArn and resourceArn
            // as secrets. The database name is fine to show.
            println!("    {}: {database}", output::label("AWS Data API"));
            println!("    SecretArn:   ****");
            println!("    ResourceArn: ****");
        }
    }
}

fn mask_url(url: &str) -> String {
    if let Some(at) = url.find('@')
        && let Some(colon) = url[..at].rfind(':')
    {
        let scheme_end = url.find("://").map(|p| p + 3).unwrap_or(0);
        if colon > scheme_end {
            return format!("{}****{}", &url[..colon + 1], &url[at..]);
        }
    }
    url.to_string()
}

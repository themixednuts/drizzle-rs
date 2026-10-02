//! `drizzle init`: writes a starter `drizzle.config.toml`.

use crate::config::CONFIG_FILE;
use crate::error::CliError;
use crate::output;
use std::path::Path;

/// JSON schema URL for TOML validation
const SCHEMA_URL: &str =
    "https://raw.githubusercontent.com/themixednuts/drizzle-rs/main/cli/schema.json";

/// Default `schema` value of a new config.
pub const DEFAULT_SCHEMA: &str = "src/schema.rs";

/// Default `out` value of a new config.
pub const DEFAULT_OUT: &str = "./drizzle";

/// Runs `drizzle init`: writes [`CONFIG_FILE`] in the current directory.
///
/// # Errors
///
/// Returns [`CliError`] if the file already exists, the dialect or driver is
/// unknown, or the file cannot be written.
pub fn run(dialect: &str, driver: Option<&str>) -> Result<(), CliError> {
    let config_path = Path::new(CONFIG_FILE);

    if config_path.exists() {
        return Err(CliError::Other(format!(
            "{CONFIG_FILE} already exists. Delete it first to reinitialize."
        )));
    }

    let config_content = config_contents(dialect, driver, DEFAULT_SCHEMA, DEFAULT_OUT)?;

    std::fs::write(config_path, config_content).map_err(|e| CliError::IoError(e.to_string()))?;

    println!("{}", output::success(&format!("Created {CONFIG_FILE}")));
    println!();
    println!("Next steps:");
    println!("  1. Edit {CONFIG_FILE} with your database credentials");
    println!(
        "  2. Create your schema file at {}",
        output::heading(DEFAULT_SCHEMA)
    );
    println!(
        "  3. Run {} to generate your first migration",
        output::heading("drizzle generate")
    );

    Ok(())
}

/// Returns the text of a starter config for `dialect` (and optionally
/// `driver`), with `schema` and `out` set to the given paths.
///
/// Accepts the dialects `sqlite`, `turso`, `postgresql` (or `postgres`) and
/// `mysql`, and only the Rust drivers of that dialect.
///
/// # Errors
///
/// Returns [`CliError::Other`] for an unknown dialect or a driver that does
/// not belong to it.
pub fn config_contents(
    dialect: &str,
    driver: Option<&str>,
    schema: &str,
    out: &str,
) -> Result<String, CliError> {
    let dialect = dialect.to_lowercase();
    let driver = driver.map(str::to_lowercase);
    let schema = toml_string(schema);
    let out = toml_string(out);

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
# - dialect: sqlite | turso | postgresql | mysql
# - drivers: Rust drivers only (optional)

dialect = "sqlite"
# driver = "rusqlite"
schema = {schema}
out = {out}
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
schema = {schema}
out = {out}
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
schema = {schema}
out = {out}
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
        "mysql" => {
            if let Some(ref driver) = driver
                && driver != "mysql-sync"
                && driver != "mysql-async"
            {
                return Err(CliError::Other(format!(
                    "Invalid driver for mysql: {driver}. Supported: mysql-sync, mysql-async"
                )));
            }
            let driver_config = driver.as_deref().map_or_else(
                || "# driver = \"mysql-sync\"\n# driver = \"mysql-async\"".to_string(),
                |driver| format!("driver = \"{driver}\""),
            );
            Ok(format!(
                r#"#:schema {SCHEMA_URL}

# Drizzle Configuration (drizzle-rs)

dialect = "mysql"
{driver_config}
schema = {schema}
out = {out}
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
# ssl = "required"
"#
            ))
        }
        _ => Err(CliError::Other(format!(
            "Unknown dialect: {dialect}. Supported: sqlite, turso, postgresql, mysql"
        ))),
    }
}

/// Quotes `value` as a TOML string (backslashes in Windows paths included).
fn toml_string(value: &str) -> String {
    toml::Value::String(value.to_string()).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mysql_init_template_uses_mysql_port_and_drivers() {
        let config = config_contents("mysql", Some("mysql-sync"), DEFAULT_SCHEMA, DEFAULT_OUT)
            .expect("mysql config");
        assert!(config.contains("dialect = \"mysql\""), "{config}");
        assert!(config.contains("mysql://user:password@localhost:3306/mydb"));
        assert!(config.contains("driver = \"mysql-sync\""), "{config}");
        assert!(!config.contains("ssl = \"preferred\""), "{config}");

        let async_config =
            config_contents("mysql", Some("mysql-async"), DEFAULT_SCHEMA, DEFAULT_OUT)
                .expect("async mysql config");
        assert!(
            async_config.contains("driver = \"mysql-async\""),
            "{async_config}"
        );
        assert!(
            config_contents("mysql", Some("postgres-sync"), DEFAULT_SCHEMA, DEFAULT_OUT).is_err()
        );
    }

    #[test]
    fn template_paths_are_quoted_and_parse_back() {
        for dialect in ["sqlite", "turso", "postgresql", "mysql"] {
            let text = config_contents(dialect, None, "src/db/schema.rs", "C:\\app\\drizzle")
                .expect("config");
            let value: toml::Table = toml::from_str(&text).expect("valid TOML");
            assert_eq!(value["schema"].as_str(), Some("src/db/schema.rs"));
            assert_eq!(value["out"].as_str(), Some("C:\\app\\drizzle"));
        }
    }
}

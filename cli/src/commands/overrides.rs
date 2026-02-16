use std::path::PathBuf;

use crate::config::{
    Credentials, DatabaseConfig, Dialect, Driver, Extension, Filter, PostgresCreds,
};
use crate::error::CliError;

#[derive(Debug, Clone, Default)]
pub struct ConnectionOverrides {
    pub url: Option<String>,
    pub host: Option<String>,
    pub port: Option<u16>,
    pub user: Option<String>,
    pub password: Option<String>,
    pub database: Option<String>,
    pub ssl: Option<String>,
    pub auth_token: Option<String>,
}

impl ConnectionOverrides {
    pub fn has_any(&self) -> bool {
        self.url.is_some()
            || self.host.is_some()
            || self.port.is_some()
            || self.user.is_some()
            || self.password.is_some()
            || self.database.is_some()
            || self.ssl.is_some()
            || self.auth_token.is_some()
    }
}

pub fn resolve_dialect(db: &DatabaseConfig, override_dialect: Option<Dialect>) -> Dialect {
    override_dialect.unwrap_or(db.dialect)
}

pub fn resolve_driver(
    db: &DatabaseConfig,
    dialect: Dialect,
    driver_override: Option<Driver>,
) -> Result<Option<Driver>, CliError> {
    let driver = driver_override.or(db.driver);
    if let Some(driver) = driver
        && !driver.is_valid_for(dialect)
    {
        return Err(CliError::Other(format!(
            "driver '{}' invalid for {} dialect",
            driver, dialect
        )));
    }
    Ok(driver)
}

pub fn resolve_credentials(
    db: &DatabaseConfig,
    dialect: Dialect,
    overrides: &ConnectionOverrides,
) -> Result<Option<Credentials>, CliError> {
    if !overrides.has_any() {
        if dialect != db.dialect {
            return Err(CliError::Other(format!(
                "--dialect={} requires matching credential flags (--url/--host/--database/etc)",
                dialect
            )));
        }
        return db.credentials().map_err(Into::into);
    }

    let creds = match dialect {
        Dialect::Sqlite => {
            if overrides.host.is_some()
                || overrides.port.is_some()
                || overrides.user.is_some()
                || overrides.password.is_some()
                || overrides.database.is_some()
                || overrides.ssl.is_some()
                || overrides.auth_token.is_some()
            {
                return Err(CliError::Other(
                    "sqlite credentials only support --url for local database path".into(),
                ));
            }

            let path = overrides
                .url
                .clone()
                .ok_or_else(|| CliError::Other("sqlite requires --url".into()))?;

            Credentials::Sqlite {
                path: path.into_boxed_str(),
            }
        }
        Dialect::Turso => {
            if overrides.host.is_some()
                || overrides.port.is_some()
                || overrides.user.is_some()
                || overrides.password.is_some()
                || overrides.database.is_some()
                || overrides.ssl.is_some()
            {
                return Err(CliError::Other(
                    "turso credentials support --url and optional --authToken".into(),
                ));
            }

            let url = overrides
                .url
                .clone()
                .ok_or_else(|| CliError::Other("turso requires --url".into()))?;

            Credentials::Turso {
                url: url.into_boxed_str(),
                auth_token: overrides.auth_token.clone().map(String::into_boxed_str),
            }
        }
        Dialect::Postgresql => {
            if overrides.auth_token.is_some() {
                return Err(CliError::Other(
                    "postgresql does not support --authToken (use --password or --url)".into(),
                ));
            }

            if let Some(url) = overrides.url.clone() {
                if overrides.host.is_some()
                    || overrides.port.is_some()
                    || overrides.user.is_some()
                    || overrides.password.is_some()
                    || overrides.database.is_some()
                    || overrides.ssl.is_some()
                {
                    return Err(CliError::Other(
                        "postgresql credentials: use either --url OR --host/--database[/--port/...], not both"
                            .into(),
                    ));
                }

                Credentials::Postgres(PostgresCreds::Url(url.into_boxed_str()))
            } else {
                let host = overrides.host.clone().ok_or_else(|| {
                    CliError::Other("postgresql host credentials require --host".into())
                })?;
                let database = overrides.database.clone().ok_or_else(|| {
                    CliError::Other("postgresql host credentials require --database".into())
                })?;

                Credentials::Postgres(PostgresCreds::Host {
                    host: host.into_boxed_str(),
                    port: overrides.port.unwrap_or(5432),
                    user: overrides.user.clone().map(String::into_boxed_str),
                    password: overrides.password.clone().map(String::into_boxed_str),
                    database: database.into_boxed_str(),
                    ssl: parse_ssl_override(overrides.ssl.as_deref())?.unwrap_or(false),
                })
            }
        }
    };

    Ok(Some(creds))
}

fn parse_ssl_override(ssl: Option<&str>) -> Result<Option<bool>, CliError> {
    let Some(raw) = ssl else {
        return Ok(None);
    };

    let value = raw.trim().to_ascii_lowercase();
    let enabled = match value.as_str() {
        "true" | "1" | "yes" | "on" | "require" | "allow" | "prefer" | "verify-full"
        | "verify-ca" => true,
        "false" | "0" | "no" | "off" | "disable" => false,
        _ => {
            return Err(CliError::Other(format!(
                "invalid --ssl value '{}'; expected one of: true,false,require,allow,prefer,verify-full,verify-ca,disable",
                raw
            )));
        }
    };

    Ok(Some(enabled))
}

pub fn resolve_filter_list(cli: Option<&[String]>, config: Option<&Filter>) -> Option<Vec<String>> {
    if let Some(values) = cli {
        if values.is_empty() {
            return None;
        }
        return Some(values.to_vec());
    }

    config.map(|f| f.iter().map(ToOwned::to_owned).collect())
}

pub fn resolve_schema_filters(
    dialect: Dialect,
    cli: Option<&[String]>,
    config: Option<&Filter>,
) -> Option<Vec<String>> {
    let resolved = resolve_filter_list(cli, config);
    if resolved.is_some() {
        return resolved;
    }

    if matches!(dialect, Dialect::Postgresql) {
        Some(vec!["public".to_string()])
    } else {
        None
    }
}

pub fn resolve_extensions_filter(
    cli: Option<&[Extension]>,
    config: Option<&[Extension]>,
) -> Option<Vec<Extension>> {
    if let Some(values) = cli {
        if values.is_empty() {
            return None;
        }
        return Some(values.to_vec());
    }

    config.map(|v| v.to_vec())
}

pub fn resolve_schema_display(db: &DatabaseConfig, schema_override: Option<&[String]>) -> String {
    match schema_override {
        Some(v) if !v.is_empty() => v.join(", "),
        _ => db.schema_display(),
    }
}

pub fn resolve_schema_files(
    db: &DatabaseConfig,
    schema_override: Option<&[String]>,
) -> Result<Vec<PathBuf>, CliError> {
    let Some(schema_patterns) = schema_override else {
        return db.schema_files().map_err(Into::into);
    };

    if schema_patterns.is_empty() {
        return Err(CliError::NoSchemaFiles("(empty schema override)".into()));
    }

    let mut files = Vec::new();

    for pattern in schema_patterns {
        let pat = pattern.trim();
        let is_glob = pat.contains('*') || pat.contains('?') || pat.contains('[');

        if !is_glob {
            let p = PathBuf::from(pat);
            if p.exists() {
                files.push(p);
                continue;
            }
        }

        let pat_norm = pat.replace('\\', "/");
        let paths = glob::glob(&pat_norm)
            .map_err(|e| CliError::Other(format!("invalid glob '{}': {}", pat, e)))?;
        let matched: Vec<_> = paths.filter_map(Result::ok).collect();

        if matched.is_empty() && !is_glob {
            let p = PathBuf::from(&pat_norm);
            if p.exists() {
                files.push(p);
            }
        } else {
            files.extend(matched);
        }
    }

    files.retain(|p| p.is_file());
    files.sort();
    files.dedup();

    if files.is_empty() {
        return Err(CliError::NoSchemaFiles(
            schema_patterns
                .iter()
                .map(|s| s.as_str())
                .collect::<Vec<_>>()
                .join(", "),
        ));
    }

    Ok(files)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use std::path::PathBuf;
    use tempfile::TempDir;

    fn load_db(config_toml: &str) -> (TempDir, DatabaseConfig) {
        let dir = TempDir::new().expect("temp dir");
        let path = dir.path().join("drizzle.config.toml");
        std::fs::write(&path, config_toml).expect("write config");
        let config = Config::load_from(&path).expect("load config");
        let db = config.default_database().expect("default db").clone();
        (dir, db)
    }

    #[test]
    fn resolve_filter_list_prefers_cli_values() {
        let config = Filter::Many(vec!["from_config".to_string()]);
        let cli = vec!["from_cli".to_string()];

        let resolved = resolve_filter_list(Some(&cli), Some(&config));
        assert_eq!(resolved, Some(vec!["from_cli".to_string()]));
    }

    #[test]
    fn resolve_filter_list_uses_config_when_cli_missing() {
        let config = Filter::Many(vec!["public".to_string(), "dev".to_string()]);
        let resolved = resolve_filter_list(None, Some(&config));
        assert_eq!(
            resolved,
            Some(vec!["public".to_string(), "dev".to_string()])
        );
    }

    #[test]
    fn resolve_schema_filters_defaults_to_public_for_postgres() {
        let resolved = resolve_schema_filters(Dialect::Postgresql, None, None);
        assert_eq!(resolved, Some(vec!["public".to_string()]));
    }

    #[test]
    fn resolve_schema_filters_does_not_default_for_sqlite() {
        let resolved = resolve_schema_filters(Dialect::Sqlite, None, None);
        assert_eq!(resolved, None);
    }

    #[test]
    fn resolve_extensions_filter_prefers_cli_values() {
        let cli = vec![Extension::Postgis];
        let config = vec![];

        let resolved = resolve_extensions_filter(Some(&cli), Some(&config));
        assert_eq!(resolved, Some(vec![Extension::Postgis]));
    }

    #[test]
    fn resolve_driver_rejects_invalid_override() {
        let (_dir, db) = load_db(
            r#"
dialect = "sqlite"
schema = "src/schema.rs"
"#,
        );

        let err = resolve_driver(&db, Dialect::Sqlite, Some(Driver::TokioPostgres))
            .expect_err("driver should be rejected");
        assert_eq!(
            err.to_string(),
            "driver 'tokio-postgres' invalid for sqlite dialect"
        );
    }

    #[test]
    fn resolve_credentials_requires_overrides_for_dialect_switch() {
        let (_dir, db) = load_db(
            r#"
dialect = "sqlite"
[dbCredentials]
url = "./dev.db"
"#,
        );

        let err = resolve_credentials(&db, Dialect::Postgresql, &ConnectionOverrides::default())
            .expect_err("dialect switch should require explicit credentials");
        assert_eq!(
            err.to_string(),
            "--dialect=postgresql requires matching credential flags (--url/--host/--database/etc)"
        );
    }

    #[test]
    fn resolve_credentials_sqlite_rejects_host_fields() {
        let (_dir, db) = load_db(
            r#"
dialect = "sqlite"
"#,
        );

        let overrides = ConnectionOverrides {
            host: Some("localhost".to_string()),
            ..Default::default()
        };

        let err = resolve_credentials(&db, Dialect::Sqlite, &overrides)
            .expect_err("sqlite should reject host-style credentials");
        assert_eq!(
            err.to_string(),
            "sqlite credentials only support --url for local database path"
        );
    }

    #[test]
    fn resolve_credentials_postgres_rejects_mixed_url_and_host_fields() {
        let (_dir, db) = load_db(
            r#"
dialect = "postgresql"
"#,
        );

        let overrides = ConnectionOverrides {
            url: Some("postgres://u:p@localhost:5432/db".to_string()),
            host: Some("localhost".to_string()),
            database: Some("db".to_string()),
            ..Default::default()
        };

        let err = resolve_credentials(&db, Dialect::Postgresql, &overrides)
            .expect_err("postgres should reject mixed credentials");
        assert_eq!(
            err.to_string(),
            "postgresql credentials: use either --url OR --host/--database[/--port/...], not both"
        );
    }

    #[test]
    fn resolve_credentials_postgres_requires_database_for_host_mode() {
        let (_dir, db) = load_db(
            r#"
dialect = "postgresql"
"#,
        );

        let overrides = ConnectionOverrides {
            host: Some("localhost".to_string()),
            ..Default::default()
        };

        let err = resolve_credentials(&db, Dialect::Postgresql, &overrides)
            .expect_err("postgres host credentials require database");
        assert_eq!(
            err.to_string(),
            "postgresql host credentials require --database"
        );
    }

    #[test]
    fn resolve_credentials_turso_accepts_url_with_optional_token() {
        let (_dir, db) = load_db(
            r#"
dialect = "turso"
"#,
        );

        let overrides = ConnectionOverrides {
            url: Some("libsql://example.turso.io".to_string()),
            auth_token: Some("secret".to_string()),
            ..Default::default()
        };

        let creds = resolve_credentials(&db, Dialect::Turso, &overrides)
            .expect("resolve creds")
            .expect("some creds");

        match creds {
            Credentials::Turso { url, auth_token } => {
                assert_eq!(url.as_ref(), "libsql://example.turso.io");
                assert_eq!(auth_token.as_deref(), Some("secret"));
            }
            _ => panic!("expected turso credentials"),
        }
    }

    #[test]
    fn resolve_credentials_postgres_host_mode_accepts_ssl_modes() {
        let (_dir, db) = load_db(
            r#"
dialect = "postgresql"
"#,
        );

        let require_ssl = ConnectionOverrides {
            host: Some("localhost".to_string()),
            database: Some("db".to_string()),
            ssl: Some("require".to_string()),
            ..Default::default()
        };
        let creds = resolve_credentials(&db, Dialect::Postgresql, &require_ssl)
            .expect("resolve")
            .expect("creds");
        match creds {
            Credentials::Postgres(PostgresCreds::Host { ssl, .. }) => assert!(ssl),
            _ => panic!("expected postgres host creds"),
        }

        let disable_ssl = ConnectionOverrides {
            host: Some("localhost".to_string()),
            database: Some("db".to_string()),
            ssl: Some("disable".to_string()),
            ..Default::default()
        };
        let creds = resolve_credentials(&db, Dialect::Postgresql, &disable_ssl)
            .expect("resolve")
            .expect("creds");
        match creds {
            Credentials::Postgres(PostgresCreds::Host { ssl, .. }) => assert!(!ssl),
            _ => panic!("expected postgres host creds"),
        }
    }

    #[test]
    fn resolve_credentials_postgres_host_mode_rejects_invalid_ssl_value() {
        let (_dir, db) = load_db(
            r#"
dialect = "postgresql"
"#,
        );

        let overrides = ConnectionOverrides {
            host: Some("localhost".to_string()),
            database: Some("db".to_string()),
            ssl: Some("maybe".to_string()),
            ..Default::default()
        };

        let err = resolve_credentials(&db, Dialect::Postgresql, &overrides)
            .expect_err("invalid ssl should fail");
        assert_eq!(
            err.to_string(),
            "invalid --ssl value 'maybe'; expected one of: true,false,require,allow,prefer,verify-full,verify-ca,disable"
        );
    }

    #[test]
    fn resolve_schema_filters_defaults_to_public_in_multi_db_postgres() {
        let dir = TempDir::new().expect("temp dir");
        let path = dir.path().join("drizzle.config.toml");
        std::fs::write(
            &path,
            r#"
[databases.pg]
dialect = "postgresql"

[databases.pg.dbCredentials]
url = "postgres://localhost/db"

[databases.sqlite]
dialect = "sqlite"

[databases.sqlite.dbCredentials]
url = "./dev.db"
"#,
        )
        .expect("write config");

        let config = Config::load_from(&path).expect("load config");
        let db = config.database(Some("pg")).expect("pg db");

        let resolved = resolve_schema_filters(Dialect::Postgresql, None, db.schema_filter.as_ref());
        assert_eq!(resolved, Some(vec!["public".to_string()]));
    }

    #[test]
    fn resolve_schema_files_uses_override_glob() {
        let (dir, db) = load_db(
            r#"
dialect = "sqlite"
schema = "src/schema.rs"
"#,
        );

        let a = dir.path().join("a.schema.rs");
        let b = dir.path().join("b.schema.rs");
        std::fs::write(&a, "pub struct A;").expect("write a");
        std::fs::write(&b, "pub struct B;").expect("write b");

        let pattern = format!("{}/*.schema.rs", dir.path().display()).replace('\\', "/");
        let override_patterns = vec![pattern];
        let files = resolve_schema_files(&db, Some(&override_patterns)).expect("resolve files");

        let paths: Vec<PathBuf> = files;
        assert_eq!(paths.len(), 2);
        assert!(paths.iter().any(|p| p.ends_with("a.schema.rs")));
        assert!(paths.iter().any(|p| p.ends_with("b.schema.rs")));
    }
}

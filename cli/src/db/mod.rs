//! Database connection and migration execution for CLI commands
//!
//! This module provides database connectivity for running migrations and other
//! database operations from the CLI.

use std::path::Path;

#[cfg(any(feature = "postgres-sync", feature = "tokio-postgres"))]
use crate::config::PostgresCreds;
use crate::config::{Credentials, Dialect};
use crate::error::CliError;
use drizzle_migrations::MigrationSet;

/// Result of a migration run
#[derive(Debug)]
pub struct MigrationResult {
    /// Number of migrations applied
    pub applied_count: usize,
    /// Tags of applied migrations
    pub applied_migrations: Vec<String>,
}

/// Execute migrations against the database
///
/// This is the main entry point that dispatches to the appropriate driver
/// based on the credentials type.
pub fn run_migrations(
    credentials: &Credentials,
    dialect: Dialect,
    migrations_dir: &Path,
) -> Result<MigrationResult, CliError> {
    // Load migrations from filesystem
    let set = MigrationSet::from_dir(migrations_dir, dialect.to_base())
        .map_err(|e| CliError::Other(format!("Failed to load migrations: {}", e)))?;

    if set.all().is_empty() {
        return Ok(MigrationResult {
            applied_count: 0,
            applied_migrations: vec![],
        });
    }

    match credentials {
        #[cfg(feature = "rusqlite")]
        Credentials::Sqlite { path } => run_sqlite_migrations(&set, path),

        #[cfg(not(feature = "rusqlite"))]
        Credentials::Sqlite { .. } => Err(CliError::MissingDriver {
            dialect: "SQLite",
            feature: "rusqlite",
        }),

        #[cfg(feature = "libsql")]
        Credentials::Turso { url, auth_token } if is_local_libsql(url) => {
            run_libsql_local_migrations(&set, url)
        }

        #[cfg(feature = "turso")]
        Credentials::Turso { url, auth_token } => {
            run_turso_migrations(&set, url, auth_token.as_deref())
        }

        #[cfg(all(not(feature = "turso"), not(feature = "libsql")))]
        Credentials::Turso { .. } => Err(CliError::MissingDriver {
            dialect: "Turso",
            feature: "turso or libsql",
        }),

        #[cfg(all(not(feature = "turso"), feature = "libsql"))]
        Credentials::Turso { url, .. } if !is_local_libsql(url) => Err(CliError::MissingDriver {
            dialect: "Turso (remote)",
            feature: "turso",
        }),

        // PostgreSQL - prefer sync driver if available, fall back to async
        #[cfg(feature = "postgres-sync")]
        Credentials::Postgres(creds) => run_postgres_sync_migrations(&set, creds),

        #[cfg(all(not(feature = "postgres-sync"), feature = "tokio-postgres"))]
        Credentials::Postgres(creds) => run_postgres_async_migrations(&set, creds),

        #[cfg(all(not(feature = "postgres-sync"), not(feature = "tokio-postgres")))]
        Credentials::Postgres(_) => Err(CliError::MissingDriver {
            dialect: "PostgreSQL",
            feature: "postgres-sync or tokio-postgres",
        }),

        // Other credential types not yet supported for direct migration
        _ => Err(CliError::Other(
            "This credential type is not yet supported for CLI migrations. \
             Use the programmatic API instead."
                .into(),
        )),
    }
}

/// Check if a Turso URL is a local libsql database
#[allow(dead_code)]
fn is_local_libsql(url: &str) -> bool {
    url.starts_with("file:")
        || url.starts_with("./")
        || url.starts_with("/")
        || !url.contains("://")
}

// ============================================================================
// SQLite (rusqlite)
// ============================================================================

#[cfg(feature = "rusqlite")]
fn run_sqlite_migrations(set: &MigrationSet, path: &str) -> Result<MigrationResult, CliError> {
    let conn = rusqlite::Connection::open(path).map_err(|e| {
        CliError::ConnectionError(format!("Failed to open SQLite database '{}': {}", path, e))
    })?;

    // Create migrations table
    conn.execute(&set.create_table_sql(), []).map_err(|e| {
        CliError::MigrationError(format!("Failed to create migrations table: {}", e))
    })?;

    // Query applied hashes
    let applied_hashes = query_applied_hashes_sqlite(&conn, set)?;

    // Get pending migrations
    let pending: Vec<_> = set.pending(&applied_hashes).collect();
    if pending.is_empty() {
        return Ok(MigrationResult {
            applied_count: 0,
            applied_migrations: vec![],
        });
    }

    // Execute in transaction
    conn.execute("BEGIN", [])
        .map_err(|e| CliError::MigrationError(e.to_string()))?;

    let mut applied = Vec::new();
    for migration in &pending {
        for stmt in migration.statements() {
            if !stmt.trim().is_empty()
                && let Err(e) = conn.execute(stmt, [])
            {
                let _ = conn.execute("ROLLBACK", []);
                return Err(CliError::MigrationError(format!(
                    "Migration '{}' failed: {}",
                    migration.hash(),
                    e
                )));
            }
        }
        if let Err(e) = conn.execute(
            &set.record_migration_sql(migration.hash(), migration.created_at()),
            [],
        ) {
            let _ = conn.execute("ROLLBACK", []);
            return Err(CliError::MigrationError(e.to_string()));
        }
        applied.push(migration.hash().to_string());
    }

    conn.execute("COMMIT", [])
        .map_err(|e| CliError::MigrationError(e.to_string()))?;

    Ok(MigrationResult {
        applied_count: applied.len(),
        applied_migrations: applied,
    })
}

#[cfg(feature = "rusqlite")]
fn query_applied_hashes_sqlite(
    conn: &rusqlite::Connection,
    set: &MigrationSet,
) -> Result<Vec<String>, CliError> {
    let mut stmt = match conn.prepare(&set.query_all_hashes_sql()) {
        Ok(s) => s,
        Err(_) => return Ok(vec![]), // Table might not exist yet
    };

    let hashes = stmt
        .query_map([], |row| row.get(0))
        .map_err(|e| CliError::MigrationError(e.to_string()))?
        .filter_map(Result::ok)
        .collect();

    Ok(hashes)
}

// ============================================================================
// PostgreSQL (postgres - sync)
// ============================================================================

#[cfg(feature = "postgres-sync")]
fn run_postgres_sync_migrations(
    set: &MigrationSet,
    creds: &PostgresCreds,
) -> Result<MigrationResult, CliError> {
    let url = creds.connection_url();
    let mut client = postgres::Client::connect(&url, postgres::NoTls).map_err(|e| {
        CliError::ConnectionError(format!("Failed to connect to PostgreSQL: {}", e))
    })?;

    // Create schema if needed
    if let Some(schema_sql) = set.create_schema_sql() {
        client
            .execute(&schema_sql, &[])
            .map_err(|e| CliError::MigrationError(e.to_string()))?;
    }

    // Create migrations table
    client
        .execute(&set.create_table_sql(), &[])
        .map_err(|e| CliError::MigrationError(e.to_string()))?;

    // Query applied hashes
    let rows = client
        .query(&set.query_all_hashes_sql(), &[])
        .unwrap_or_default();
    let applied_hashes: Vec<String> = rows.iter().map(|r| r.get(0)).collect();

    // Get pending migrations
    let pending: Vec<_> = set.pending(&applied_hashes).collect();
    if pending.is_empty() {
        return Ok(MigrationResult {
            applied_count: 0,
            applied_migrations: vec![],
        });
    }

    // Execute in transaction
    let mut tx = client
        .transaction()
        .map_err(|e| CliError::MigrationError(e.to_string()))?;

    let mut applied = Vec::new();
    for migration in &pending {
        for stmt in migration.statements() {
            if !stmt.trim().is_empty() {
                tx.execute(stmt, &[]).map_err(|e| {
                    CliError::MigrationError(format!(
                        "Migration '{}' failed: {}",
                        migration.hash(),
                        e
                    ))
                })?;
            }
        }
        tx.execute(
            &set.record_migration_sql(migration.hash(), migration.created_at()),
            &[],
        )
        .map_err(|e| CliError::MigrationError(e.to_string()))?;
        applied.push(migration.hash().to_string());
    }

    tx.commit()
        .map_err(|e| CliError::MigrationError(e.to_string()))?;

    Ok(MigrationResult {
        applied_count: applied.len(),
        applied_migrations: applied,
    })
}

// ============================================================================
// PostgreSQL (tokio-postgres - async)
// ============================================================================

#[cfg(feature = "tokio-postgres")]
#[allow(dead_code)] // Used when postgres-sync is not enabled
fn run_postgres_async_migrations(
    set: &MigrationSet,
    creds: &PostgresCreds,
) -> Result<MigrationResult, CliError> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| CliError::Other(format!("Failed to create async runtime: {}", e)))?;

    rt.block_on(run_postgres_async_inner(set, creds))
}

#[cfg(feature = "tokio-postgres")]
#[allow(dead_code)]
async fn run_postgres_async_inner(
    set: &MigrationSet,
    creds: &PostgresCreds,
) -> Result<MigrationResult, CliError> {
    let url = creds.connection_url();
    let (mut client, connection) = tokio_postgres::connect(&url, tokio_postgres::NoTls)
        .await
        .map_err(|e| {
            CliError::ConnectionError(format!("Failed to connect to PostgreSQL: {}", e))
        })?;

    // Spawn connection handler
    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("PostgreSQL connection error: {}", e);
        }
    });

    // Create schema if needed
    if let Some(schema_sql) = set.create_schema_sql() {
        client
            .execute(&schema_sql, &[])
            .await
            .map_err(|e| CliError::MigrationError(e.to_string()))?;
    }

    // Create migrations table
    client
        .execute(&set.create_table_sql(), &[])
        .await
        .map_err(|e| CliError::MigrationError(e.to_string()))?;

    // Query applied hashes
    let rows = client
        .query(&set.query_all_hashes_sql(), &[])
        .await
        .unwrap_or_default();
    let applied_hashes: Vec<String> = rows.iter().map(|r| r.get(0)).collect();

    // Get pending migrations
    let pending: Vec<_> = set.pending(&applied_hashes).collect();
    if pending.is_empty() {
        return Ok(MigrationResult {
            applied_count: 0,
            applied_migrations: vec![],
        });
    }

    // Execute in transaction
    let tx = client
        .transaction()
        .await
        .map_err(|e| CliError::MigrationError(e.to_string()))?;

    let mut applied = Vec::new();
    for migration in &pending {
        for stmt in migration.statements() {
            if !stmt.trim().is_empty() {
                tx.execute(stmt, &[]).await.map_err(|e| {
                    CliError::MigrationError(format!(
                        "Migration '{}' failed: {}",
                        migration.hash(),
                        e
                    ))
                })?;
            }
        }
        tx.execute(
            &set.record_migration_sql(migration.hash(), migration.created_at()),
            &[],
        )
        .await
        .map_err(|e| CliError::MigrationError(e.to_string()))?;
        applied.push(migration.hash().to_string());
    }

    tx.commit()
        .await
        .map_err(|e| CliError::MigrationError(e.to_string()))?;

    Ok(MigrationResult {
        applied_count: applied.len(),
        applied_migrations: applied,
    })
}

// ============================================================================
// LibSQL (local)
// ============================================================================

#[cfg(feature = "libsql")]
fn run_libsql_local_migrations(
    set: &MigrationSet,
    path: &str,
) -> Result<MigrationResult, CliError> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| CliError::Other(format!("Failed to create async runtime: {}", e)))?;

    rt.block_on(run_libsql_local_inner(set, path))
}

#[cfg(feature = "libsql")]
async fn run_libsql_local_inner(
    set: &MigrationSet,
    path: &str,
) -> Result<MigrationResult, CliError> {
    let db = libsql::Builder::new_local(path)
        .build()
        .await
        .map_err(|e| {
            CliError::ConnectionError(format!("Failed to open LibSQL database '{}': {}", path, e))
        })?;

    let conn = db
        .connect()
        .map_err(|e| CliError::ConnectionError(e.to_string()))?;

    // Create migrations table
    conn.execute(&set.create_table_sql(), ())
        .await
        .map_err(|e| {
            CliError::MigrationError(format!("Failed to create migrations table: {}", e))
        })?;

    // Query applied hashes
    let applied_hashes = query_applied_hashes_libsql(&conn, set).await?;

    // Get pending migrations
    let pending: Vec<_> = set.pending(&applied_hashes).collect();
    if pending.is_empty() {
        return Ok(MigrationResult {
            applied_count: 0,
            applied_migrations: vec![],
        });
    }

    // Execute in transaction
    let tx = conn
        .transaction()
        .await
        .map_err(|e| CliError::MigrationError(e.to_string()))?;

    let mut applied = Vec::new();
    for migration in &pending {
        for stmt in migration.statements() {
            if !stmt.trim().is_empty()
                && let Err(e) = tx.execute(stmt, ()).await
            {
                tx.rollback().await.ok();
                return Err(CliError::MigrationError(format!(
                    "Migration '{}' failed: {}",
                    migration.hash(),
                    e
                )));
            }
        }
        if let Err(e) = tx
            .execute(
                &set.record_migration_sql(migration.hash(), migration.created_at()),
                (),
            )
            .await
        {
            tx.rollback().await.ok();
            return Err(CliError::MigrationError(e.to_string()));
        }
        applied.push(migration.hash().to_string());
    }

    tx.commit()
        .await
        .map_err(|e| CliError::MigrationError(e.to_string()))?;

    Ok(MigrationResult {
        applied_count: applied.len(),
        applied_migrations: applied,
    })
}

#[cfg(feature = "libsql")]
async fn query_applied_hashes_libsql(
    conn: &libsql::Connection,
    set: &MigrationSet,
) -> Result<Vec<String>, CliError> {
    let mut rows = match conn.query(&set.query_all_hashes_sql(), ()).await {
        Ok(r) => r,
        Err(_) => return Ok(vec![]), // Table might not exist yet
    };

    let mut hashes = Vec::new();
    while let Ok(Some(row)) = rows.next().await {
        if let Ok(hash) = row.get::<String>(0) {
            hashes.push(hash);
        }
    }

    Ok(hashes)
}

// ============================================================================
// Turso (remote)
// ============================================================================

#[cfg(feature = "turso")]
fn run_turso_migrations(
    set: &MigrationSet,
    url: &str,
    auth_token: Option<&str>,
) -> Result<MigrationResult, CliError> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| CliError::Other(format!("Failed to create async runtime: {}", e)))?;

    rt.block_on(run_turso_inner(set, url, auth_token))
}

#[cfg(feature = "turso")]
async fn run_turso_inner(
    set: &MigrationSet,
    url: &str,
    auth_token: Option<&str>,
) -> Result<MigrationResult, CliError> {
    let builder =
        libsql::Builder::new_remote(url.to_string(), auth_token.unwrap_or("").to_string());

    let db = builder.build().await.map_err(|e| {
        CliError::ConnectionError(format!("Failed to connect to Turso '{}': {}", url, e))
    })?;

    let conn = db
        .connect()
        .map_err(|e| CliError::ConnectionError(e.to_string()))?;

    // Create migrations table
    conn.execute(&set.create_table_sql(), ())
        .await
        .map_err(|e| {
            CliError::MigrationError(format!("Failed to create migrations table: {}", e))
        })?;

    // Query applied hashes
    let applied_hashes = query_applied_hashes_turso(&conn, set).await?;

    // Get pending migrations
    let pending: Vec<_> = set.pending(&applied_hashes).collect();
    if pending.is_empty() {
        return Ok(MigrationResult {
            applied_count: 0,
            applied_migrations: vec![],
        });
    }

    // Execute in transaction
    let tx = conn
        .transaction()
        .await
        .map_err(|e| CliError::MigrationError(e.to_string()))?;

    let mut applied = Vec::new();
    for migration in &pending {
        for stmt in migration.statements() {
            if !stmt.trim().is_empty()
                && let Err(e) = tx.execute(stmt, ()).await
            {
                tx.rollback().await.ok();
                return Err(CliError::MigrationError(format!(
                    "Migration '{}' failed: {}",
                    migration.hash(),
                    e
                )));
            }
        }
        if let Err(e) = tx
            .execute(
                &set.record_migration_sql(migration.hash(), migration.created_at()),
                (),
            )
            .await
        {
            tx.rollback().await.ok();
            return Err(CliError::MigrationError(e.to_string()));
        }
        applied.push(migration.hash().to_string());
    }

    tx.commit()
        .await
        .map_err(|e| CliError::MigrationError(e.to_string()))?;

    Ok(MigrationResult {
        applied_count: applied.len(),
        applied_migrations: applied,
    })
}

#[cfg(feature = "turso")]
async fn query_applied_hashes_turso(
    conn: &libsql::Connection,
    set: &MigrationSet,
) -> Result<Vec<String>, CliError> {
    let mut rows = match conn.query(&set.query_all_hashes_sql(), ()).await {
        Ok(r) => r,
        Err(_) => return Ok(vec![]), // Table might not exist yet
    };

    let mut hashes = Vec::new();
    while let Ok(Some(row)) = rows.next().await {
        if let Ok(hash) = row.get::<String>(0) {
            hashes.push(hash);
        }
    }

    Ok(hashes)
}

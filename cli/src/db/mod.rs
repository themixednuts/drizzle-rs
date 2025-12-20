//! Database connection and migration execution for CLI commands
//!
//! This module provides database connectivity for running migrations and other
//! database operations from the CLI.

use std::path::Path;

#[cfg(any(feature = "postgres-sync", feature = "tokio-postgres"))]
use crate::config::PostgresCreds;
use crate::config::{Credentials, Dialect};
use crate::error::CliError;
use drizzle_migrations::schema::Snapshot;
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

// ============================================================================
// Database Introspection
// ============================================================================

/// Result of database introspection
#[derive(Debug)]
pub struct IntrospectResult {
    /// Generated Rust schema code
    pub schema_code: String,
    /// Number of tables found
    pub table_count: usize,
    /// Number of indexes found
    pub index_count: usize,
    /// Number of views found
    pub view_count: usize,
    /// Any warnings during introspection
    pub warnings: Vec<String>,
    /// The schema snapshot for migration tracking
    pub snapshot: Snapshot,
    /// Path to the generated snapshot file
    pub snapshot_path: std::path::PathBuf,
}

/// Introspect a database and write schema/snapshot files
///
/// This is the main entry point for CLI introspection.
pub fn run_introspection(
    credentials: &Credentials,
    dialect: Dialect,
    out_dir: &Path,
) -> Result<IntrospectResult, CliError> {
    use drizzle_migrations::journal::Journal;
    use drizzle_migrations::words::generate_migration_tag;

    // Perform introspection
    let mut result = introspect_database(credentials, dialect)?;

    // Write schema file
    let schema_path = out_dir.join("schema.rs");
    if let Some(parent) = schema_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| {
            CliError::Other(format!(
                "Failed to create output directory '{}': {}",
                parent.display(),
                e
            ))
        })?;
    }
    std::fs::write(&schema_path, &result.schema_code).map_err(|e| {
        CliError::Other(format!(
            "Failed to write schema file '{}': {}",
            schema_path.display(),
            e
        ))
    })?;

    // Create meta directory for journal
    let meta_dir = out_dir.join("meta");
    std::fs::create_dir_all(&meta_dir).map_err(|e| {
        CliError::Other(format!(
            "Failed to create meta directory '{}': {}",
            meta_dir.display(),
            e
        ))
    })?;

    // Load or create journal
    let journal_path = meta_dir.join("_journal.json");
    let mut journal = Journal::load_or_create(&journal_path, dialect.to_base())
        .map_err(|e| CliError::Other(format!("Failed to load journal: {}", e)))?;

    // Generate migration tag (V3 format: timestamp-based)
    let tag = generate_migration_tag(None);

    // Create migration directory: {out}/{tag}/
    let migration_dir = out_dir.join(&tag);
    std::fs::create_dir_all(&migration_dir).map_err(|e| {
        CliError::Other(format!(
            "Failed to create migration directory '{}': {}",
            migration_dir.display(),
            e
        ))
    })?;

    // Save snapshot JSON: {out}/{tag}/snapshot.json
    let snapshot_path = migration_dir.join("snapshot.json");
    result.snapshot.save(&snapshot_path).map_err(|e| {
        CliError::Other(format!(
            "Failed to write snapshot file '{}': {}",
            snapshot_path.display(),
            e
        ))
    })?;

    // Generate initial migration SQL by diffing against empty snapshot
    let base_dialect = dialect.to_base();
    let empty_snapshot = Snapshot::empty(base_dialect);
    let sql_statements = generate_introspect_migration(&empty_snapshot, &result.snapshot, true)?;

    // Write migration.sql: {out}/{tag}/migration.sql
    let migration_sql_path = migration_dir.join("migration.sql");
    let sql_content = if sql_statements.is_empty() {
        "-- No tables to create (empty database)\n".to_string()
    } else {
        sql_statements.join("\n--> statement-breakpoint\n")
    };
    std::fs::write(&migration_sql_path, &sql_content).map_err(|e| {
        CliError::Other(format!(
            "Failed to write migration file '{}': {}",
            migration_sql_path.display(),
            e
        ))
    })?;

    // Update result with path
    result.snapshot_path = snapshot_path;

    // Update journal
    journal.add_entry(tag.clone(), true); // Default to breakpoints=true for now
    journal.save(&journal_path).map_err(|e| {
        CliError::Other(format!("Failed to save journal: {}", e))
    })?;

    Ok(result)
}

/// Generate migration SQL from snapshot diff (for introspection)
fn generate_introspect_migration(
    prev: &Snapshot,
    current: &Snapshot,
    _breakpoints: bool,
) -> Result<Vec<String>, CliError> {
    match (prev, current) {
        (Snapshot::Sqlite(prev_snap), Snapshot::Sqlite(curr_snap)) => {
            use drizzle_migrations::sqlite::diff_snapshots;
            use drizzle_migrations::sqlite::statements::SqliteGenerator;

            let diff = diff_snapshots(prev_snap, curr_snap);
            let generator = SqliteGenerator::new().with_breakpoints(true);
            Ok(generator.generate_migration(&diff))
        }
        (Snapshot::Postgres(prev_snap), Snapshot::Postgres(curr_snap)) => {
            use drizzle_migrations::postgres::diff_full_snapshots;
            use drizzle_migrations::postgres::statements::PostgresGenerator;

            let diff = diff_full_snapshots(prev_snap, curr_snap);
            let generator = PostgresGenerator::new().with_breakpoints(true);
            Ok(generator.generate(&diff.diffs))
        }
        _ => Err(CliError::DialectMismatch),
    }
}

/// Introspect a database and generate schema code
fn introspect_database(
    credentials: &Credentials,
    dialect: Dialect,
) -> Result<IntrospectResult, CliError> {
    match dialect {
        Dialect::Sqlite | Dialect::Turso => introspect_sqlite_dialect(credentials),
        Dialect::Postgresql => introspect_postgres_dialect(credentials),
        _ => Err(CliError::Other(format!(
            "Introspection not yet supported for {:?}",
            dialect
        ))),
    }
}

/// Introspect SQLite-family databases
fn introspect_sqlite_dialect(credentials: &Credentials) -> Result<IntrospectResult, CliError> {
    match credentials {
        #[cfg(feature = "rusqlite")]
        Credentials::Sqlite { path } => introspect_rusqlite(path),

        #[cfg(not(feature = "rusqlite"))]
        Credentials::Sqlite { .. } => Err(CliError::MissingDriver {
            dialect: "SQLite",
            feature: "rusqlite",
        }),

        #[cfg(feature = "libsql")]
        Credentials::Turso { url, auth_token } if is_local_libsql(url) => {
            introspect_libsql_local(url)
        }

        #[cfg(feature = "turso")]
        Credentials::Turso { url, auth_token } => {
            introspect_turso(url, auth_token.as_deref())
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

        _ => Err(CliError::Other(
            "SQLite introspection requires sqlite path or turso credentials".into(),
        )),
    }
}

/// Introspect PostgreSQL databases
fn introspect_postgres_dialect(credentials: &Credentials) -> Result<IntrospectResult, CliError> {
    match credentials {
        #[cfg(feature = "postgres-sync")]
        Credentials::Postgres(creds) => introspect_postgres_sync(creds),

        #[cfg(all(not(feature = "postgres-sync"), feature = "tokio-postgres"))]
        Credentials::Postgres(creds) => introspect_postgres_async(creds),

        #[cfg(all(not(feature = "postgres-sync"), not(feature = "tokio-postgres")))]
        Credentials::Postgres(_) => Err(CliError::MissingDriver {
            dialect: "PostgreSQL",
            feature: "postgres-sync or tokio-postgres",
        }),

        _ => Err(CliError::Other(
            "PostgreSQL introspection requires postgres credentials".into(),
        )),
    }
}

// ============================================================================
// SQLite Introspection (rusqlite)
// ============================================================================

#[cfg(feature = "rusqlite")]
fn introspect_rusqlite(path: &str) -> Result<IntrospectResult, CliError> {
    use drizzle_migrations::sqlite::{
        codegen::{generate_rust_schema, CodegenOptions},
        introspect::{
            process_columns, process_foreign_keys, process_indexes, queries, RawColumnInfo,
            RawForeignKey, RawIndexColumn, RawIndexInfo,
        },
        SQLiteDDL, Table,
    };
    use std::collections::{HashMap, HashSet};

    let conn = rusqlite::Connection::open(path).map_err(|e| {
        CliError::ConnectionError(format!("Failed to open SQLite database '{}': {}", path, e))
    })?;

    // Query tables
    let mut tables_stmt = conn
        .prepare(queries::TABLES_QUERY)
        .map_err(|e| CliError::Other(format!("Failed to prepare tables query: {}", e)))?;

    let tables: Vec<(String, Option<String>)> = tables_stmt
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
        .map_err(|e| CliError::Other(e.to_string()))?
        .filter_map(Result::ok)
        .collect();

    let table_sql_map: HashMap<String, String> = tables
        .iter()
        .filter_map(|(name, sql)| sql.as_ref().map(|s| (name.clone(), s.clone())))
        .collect();

    // Query columns
    let mut columns_stmt = conn
        .prepare(queries::COLUMNS_QUERY)
        .map_err(|e| CliError::Other(format!("Failed to prepare columns query: {}", e)))?;

    let raw_columns: Vec<RawColumnInfo> = columns_stmt
        .query_map([], |row| {
            Ok(RawColumnInfo {
                table: row.get(0)?,
                name: row.get(1)?,
                column_type: row.get(2)?,
                not_null: row.get(3)?,
                default_value: row.get(4)?,
                pk: row.get(5)?,
                hidden: row.get(6)?,
                sql: row.get(7)?,
            })
        })
        .map_err(|e| CliError::Other(e.to_string()))?
        .filter_map(Result::ok)
        .collect();

    // Query indexes and foreign keys for each table
    let mut all_indexes: Vec<RawIndexInfo> = Vec::new();
    let mut all_index_columns: Vec<RawIndexColumn> = Vec::new();
    let mut all_fks: Vec<RawForeignKey> = Vec::new();

    for (table_name, _) in &tables {
        // Indexes
        if let Ok(mut idx_stmt) = conn.prepare(&queries::indexes_query(table_name)) {
            let indexes: Vec<RawIndexInfo> = idx_stmt
                .query_map([], |row| {
                    Ok(RawIndexInfo {
                        table: table_name.clone(),
                        name: row.get(1)?,
                        unique: row.get::<_, i32>(2)? != 0,
                        origin: row.get(3)?,
                        partial: row.get::<_, i32>(4)? != 0,
                    })
                })
                .map_err(|e| CliError::Other(e.to_string()))?
                .filter_map(Result::ok)
                .collect();

            // Index columns
            for idx in &indexes {
                if let Ok(mut col_stmt) = conn.prepare(&queries::index_info_query(&idx.name)) {
                    if let Ok(col_iter) = col_stmt.query_map([], |row| {
                        Ok(RawIndexColumn {
                            index_name: idx.name.clone(),
                            seqno: row.get(0)?,
                            cid: row.get(1)?,
                            name: row.get(2)?,
                            desc: row.get::<_, i32>(3)? != 0,
                            coll: row.get(4)?,
                            key: row.get::<_, i32>(5)? != 0,
                        })
                    }) {
                        all_index_columns.extend(col_iter.filter_map(Result::ok));
                    }
                }
            }
            all_indexes.extend(indexes);
        }

        // Foreign keys
        if let Ok(mut fk_stmt) = conn.prepare(&queries::foreign_keys_query(table_name)) {
            if let Ok(fk_iter) = fk_stmt.query_map([], |row| {
                Ok(RawForeignKey {
                    table: table_name.clone(),
                    id: row.get(0)?,
                    seq: row.get(1)?,
                    to_table: row.get(2)?,
                    from_column: row.get(3)?,
                    to_column: row.get(4)?,
                    on_update: row.get(5)?,
                    on_delete: row.get(6)?,
                    r#match: row.get(7)?,
                })
            }) {
                all_fks.extend(fk_iter.filter_map(Result::ok));
            }
        }
    }

    // Process raw data into DDL entities
    let generated_columns = HashMap::new(); // TODO: parse generated columns from SQL
    let pk_columns: HashSet<(String, String)> = raw_columns
        .iter()
        .filter(|c| c.pk > 0)
        .map(|c| (c.table.clone(), c.name.clone()))
        .collect();

    let (columns, primary_keys) = process_columns(&raw_columns, &generated_columns, &pk_columns);
    let indexes = process_indexes(&all_indexes, &all_index_columns, &table_sql_map);
    let foreign_keys = process_foreign_keys(&all_fks);

    // Build DDL collection
    let mut ddl = SQLiteDDL::new();

    for (table_name, table_sql) in &tables {
        let mut table = Table::new(table_name.clone());
        // Parse table options from SQL if available
        if let Some(sql) = table_sql {
            let sql_upper = sql.to_uppercase();
            table.strict = sql_upper.contains(" STRICT");
            table.without_rowid = sql_upper.contains("WITHOUT ROWID");
        }
        ddl.tables.push(table);
    }

    for col in columns {
        ddl.columns.push(col);
    }

    for idx in indexes {
        ddl.indexes.push(idx);
    }

    for fk in foreign_keys {
        ddl.fks.push(fk);
    }

    for pk in primary_keys {
        ddl.pks.push(pk);
    }

    // TODO: Parse unique constraints from table SQL

    // Generate Rust code
    let options = CodegenOptions {
        module_doc: Some(format!("Schema introspected from {}", path)),
        include_schema: true,
        schema_name: "Schema".to_string(),
        use_pub: true,
    };

    let generated = generate_rust_schema(&ddl, &options);

    // Create snapshot from DDL
    let mut sqlite_snapshot = drizzle_migrations::sqlite::SQLiteSnapshot::new();
    for entity in ddl.to_entities() {
        sqlite_snapshot.add_entity(entity);
    }
    let snapshot = Snapshot::Sqlite(sqlite_snapshot);

    Ok(IntrospectResult {
        schema_code: generated.code,
        table_count: generated.tables.len(),
        index_count: generated.indexes.len(),
        view_count: ddl.views.len(),
        warnings: generated.warnings,
        snapshot,
        snapshot_path: std::path::PathBuf::new(),
    })
}

// ============================================================================
// LibSQL Introspection (local)
// ============================================================================

#[cfg(feature = "libsql")]
fn introspect_libsql_local(path: &str) -> Result<IntrospectResult, CliError> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| CliError::Other(format!("Failed to create async runtime: {}", e)))?;

    rt.block_on(introspect_libsql_inner(path, None))
}

#[cfg(feature = "libsql")]
async fn introspect_libsql_inner(
    path: &str,
    _auth_token: Option<&str>,
) -> Result<IntrospectResult, CliError> {
    use drizzle_migrations::sqlite::{
        codegen::{generate_rust_schema, CodegenOptions},
        introspect::{
            process_columns, process_foreign_keys, process_indexes, queries, RawColumnInfo,
            RawForeignKey, RawIndexColumn, RawIndexInfo,
        },
        SQLiteDDL, Table,
    };
    use std::collections::{HashMap, HashSet};

    let db = libsql::Builder::new_local(path)
        .build()
        .await
        .map_err(|e| {
            CliError::ConnectionError(format!("Failed to open LibSQL database '{}': {}", path, e))
        })?;

    let conn = db
        .connect()
        .map_err(|e| CliError::ConnectionError(e.to_string()))?;

    // Query tables
    let mut tables_rows = conn
        .query(queries::TABLES_QUERY, ())
        .await
        .map_err(|e| CliError::Other(format!("Failed to query tables: {}", e)))?;

    let mut tables: Vec<(String, Option<String>)> = Vec::new();
    while let Ok(Some(row)) = tables_rows.next().await {
        let name: String = row.get(0).unwrap_or_default();
        let sql: Option<String> = row.get(1).ok();
        tables.push((name, sql));
    }

    let table_sql_map: HashMap<String, String> = tables
        .iter()
        .filter_map(|(name, sql)| sql.as_ref().map(|s| (name.clone(), s.clone())))
        .collect();

    // Query columns
    let mut columns_rows = conn
        .query(queries::COLUMNS_QUERY, ())
        .await
        .map_err(|e| CliError::Other(format!("Failed to query columns: {}", e)))?;

    let mut raw_columns: Vec<RawColumnInfo> = Vec::new();
    while let Ok(Some(row)) = columns_rows.next().await {
        raw_columns.push(RawColumnInfo {
            table: row.get(0).unwrap_or_default(),
            name: row.get(1).unwrap_or_default(),
            column_type: row.get(2).unwrap_or_default(),
            not_null: row.get::<i32>(3).unwrap_or(0) != 0,
            default_value: row.get(4).ok(),
            pk: row.get(5).unwrap_or(0),
            hidden: row.get(6).unwrap_or(0),
            sql: row.get(7).ok(),
        });
    }

    // Query indexes and foreign keys
    let mut all_indexes: Vec<RawIndexInfo> = Vec::new();
    let mut all_index_columns: Vec<RawIndexColumn> = Vec::new();
    let mut all_fks: Vec<RawForeignKey> = Vec::new();

    for (table_name, _) in &tables {
        // Indexes
        if let Ok(mut idx_rows) = conn.query(&queries::indexes_query(table_name), ()).await {
            while let Ok(Some(row)) = idx_rows.next().await {
                let idx = RawIndexInfo {
                    table: table_name.clone(),
                    name: row.get(1).unwrap_or_default(),
                    unique: row.get::<i32>(2).unwrap_or(0) != 0,
                    origin: row.get(3).unwrap_or_default(),
                    partial: row.get::<i32>(4).unwrap_or(0) != 0,
                };

                // Index columns
                if let Ok(mut col_rows) = conn.query(&queries::index_info_query(&idx.name), ()).await
                {
                    while let Ok(Some(col_row)) = col_rows.next().await {
                        all_index_columns.push(RawIndexColumn {
                            index_name: idx.name.clone(),
                            seqno: col_row.get(0).unwrap_or(0),
                            cid: col_row.get(1).unwrap_or(0),
                            name: col_row.get(2).ok(),
                            desc: col_row.get::<i32>(3).unwrap_or(0) != 0,
                            coll: col_row.get(4).unwrap_or_default(),
                            key: col_row.get::<i32>(5).unwrap_or(0) != 0,
                        });
                    }
                }

                all_indexes.push(idx);
            }
        }

        // Foreign keys
        if let Ok(mut fk_rows) = conn.query(&queries::foreign_keys_query(table_name), ()).await {
            while let Ok(Some(row)) = fk_rows.next().await {
                all_fks.push(RawForeignKey {
                    table: table_name.clone(),
                    id: row.get(0).unwrap_or(0),
                    seq: row.get(1).unwrap_or(0),
                    to_table: row.get(2).unwrap_or_default(),
                    from_column: row.get(3).unwrap_or_default(),
                    to_column: row.get(4).unwrap_or_default(),
                    on_update: row.get(5).unwrap_or_default(),
                    on_delete: row.get(6).unwrap_or_default(),
                    r#match: row.get(7).unwrap_or_default(),
                });
            }
        }
    }

    // Process into DDL
    let generated_columns = HashMap::new();
    let pk_columns: HashSet<(String, String)> = raw_columns
        .iter()
        .filter(|c| c.pk > 0)
        .map(|c| (c.table.clone(), c.name.clone()))
        .collect();

    let (columns, primary_keys) = process_columns(&raw_columns, &generated_columns, &pk_columns);
    let indexes = process_indexes(&all_indexes, &all_index_columns, &table_sql_map);
    let foreign_keys = process_foreign_keys(&all_fks);

    let mut ddl = SQLiteDDL::new();

    for (table_name, table_sql) in &tables {
        let mut table = Table::new(table_name.clone());
        if let Some(sql) = table_sql {
            let sql_upper = sql.to_uppercase();
            table.strict = sql_upper.contains(" STRICT");
            table.without_rowid = sql_upper.contains("WITHOUT ROWID");
        }
        ddl.tables.push(table);
    }

    for col in columns {
        ddl.columns.push(col);
    }
    for idx in indexes {
        ddl.indexes.push(idx);
    }
    for fk in foreign_keys {
        ddl.fks.push(fk);
    }
    for pk in primary_keys {
        ddl.pks.push(pk);
    }

    let options = CodegenOptions {
        module_doc: Some(format!("Schema introspected from {}", path)),
        include_schema: true,
        schema_name: "Schema".to_string(),
        use_pub: true,
    };

    let generated = generate_rust_schema(&ddl, &options);

    // Create snapshot from DDL
    let mut sqlite_snapshot = drizzle_migrations::sqlite::SQLiteSnapshot::new();
    for entity in ddl.to_entities() {
        sqlite_snapshot.add_entity(entity);
    }
    let snapshot = Snapshot::Sqlite(sqlite_snapshot);

    Ok(IntrospectResult {
        schema_code: generated.code,
        table_count: generated.tables.len(),
        index_count: generated.indexes.len(),
        view_count: ddl.views.len(),
        warnings: generated.warnings,
        snapshot,
        snapshot_path: std::path::PathBuf::new(),
    })
}

// ============================================================================
// Turso Introspection (remote)
// ============================================================================

#[cfg(feature = "turso")]
fn introspect_turso(url: &str, auth_token: Option<&str>) -> Result<IntrospectResult, CliError> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| CliError::Other(format!("Failed to create async runtime: {}", e)))?;

    rt.block_on(introspect_turso_inner(url, auth_token))
}

#[cfg(feature = "turso")]
async fn introspect_turso_inner(
    url: &str,
    auth_token: Option<&str>,
) -> Result<IntrospectResult, CliError> {
    use drizzle_migrations::sqlite::{
        codegen::{generate_rust_schema, CodegenOptions},
        introspect::{
            process_columns, process_foreign_keys, process_indexes, queries, RawColumnInfo,
            RawForeignKey, RawIndexColumn, RawIndexInfo,
        },
        SQLiteDDL, Table,
    };
    use std::collections::{HashMap, HashSet};

    let builder =
        libsql::Builder::new_remote(url.to_string(), auth_token.unwrap_or("").to_string());

    let db = builder.build().await.map_err(|e| {
        CliError::ConnectionError(format!("Failed to connect to Turso '{}': {}", url, e))
    })?;

    let conn = db
        .connect()
        .map_err(|e| CliError::ConnectionError(e.to_string()))?;

    // Query tables
    let mut tables_rows = conn
        .query(queries::TABLES_QUERY, ())
        .await
        .map_err(|e| CliError::Other(format!("Failed to query tables: {}", e)))?;

    let mut tables: Vec<(String, Option<String>)> = Vec::new();
    while let Ok(Some(row)) = tables_rows.next().await {
        let name: String = row.get(0).unwrap_or_default();
        let sql: Option<String> = row.get(1).ok();
        tables.push((name, sql));
    }

    let table_sql_map: HashMap<String, String> = tables
        .iter()
        .filter_map(|(name, sql)| sql.as_ref().map(|s| (name.clone(), s.clone())))
        .collect();

    // Query columns
    let mut columns_rows = conn
        .query(queries::COLUMNS_QUERY, ())
        .await
        .map_err(|e| CliError::Other(format!("Failed to query columns: {}", e)))?;

    let mut raw_columns: Vec<RawColumnInfo> = Vec::new();
    while let Ok(Some(row)) = columns_rows.next().await {
        raw_columns.push(RawColumnInfo {
            table: row.get(0).unwrap_or_default(),
            name: row.get(1).unwrap_or_default(),
            column_type: row.get(2).unwrap_or_default(),
            not_null: row.get::<i32>(3).unwrap_or(0) != 0,
            default_value: row.get(4).ok(),
            pk: row.get(5).unwrap_or(0),
            hidden: row.get(6).unwrap_or(0),
            sql: row.get(7).ok(),
        });
    }

    // Query indexes and foreign keys
    let mut all_indexes: Vec<RawIndexInfo> = Vec::new();
    let mut all_index_columns: Vec<RawIndexColumn> = Vec::new();
    let mut all_fks: Vec<RawForeignKey> = Vec::new();

    for (table_name, _) in &tables {
        // Indexes
        if let Ok(mut idx_rows) = conn.query(&queries::indexes_query(table_name), ()).await {
            while let Ok(Some(row)) = idx_rows.next().await {
                let idx = RawIndexInfo {
                    table: table_name.clone(),
                    name: row.get(1).unwrap_or_default(),
                    unique: row.get::<i32>(2).unwrap_or(0) != 0,
                    origin: row.get(3).unwrap_or_default(),
                    partial: row.get::<i32>(4).unwrap_or(0) != 0,
                };

                // Index columns
                if let Ok(mut col_rows) = conn.query(&queries::index_info_query(&idx.name), ()).await
                {
                    while let Ok(Some(col_row)) = col_rows.next().await {
                        all_index_columns.push(RawIndexColumn {
                            index_name: idx.name.clone(),
                            seqno: col_row.get(0).unwrap_or(0),
                            cid: col_row.get(1).unwrap_or(0),
                            name: col_row.get(2).ok(),
                            desc: col_row.get::<i32>(3).unwrap_or(0) != 0,
                            coll: col_row.get(4).unwrap_or_default(),
                            key: col_row.get::<i32>(5).unwrap_or(0) != 0,
                        });
                    }
                }

                all_indexes.push(idx);
            }
        }

        // Foreign keys
        if let Ok(mut fk_rows) = conn.query(&queries::foreign_keys_query(table_name), ()).await {
            while let Ok(Some(row)) = fk_rows.next().await {
                all_fks.push(RawForeignKey {
                    table: table_name.clone(),
                    id: row.get(0).unwrap_or(0),
                    seq: row.get(1).unwrap_or(0),
                    to_table: row.get(2).unwrap_or_default(),
                    from_column: row.get(3).unwrap_or_default(),
                    to_column: row.get(4).unwrap_or_default(),
                    on_update: row.get(5).unwrap_or_default(),
                    on_delete: row.get(6).unwrap_or_default(),
                    r#match: row.get(7).unwrap_or_default(),
                });
            }
        }
    }

    // Process into DDL
    let generated_columns = HashMap::new();
    let pk_columns: HashSet<(String, String)> = raw_columns
        .iter()
        .filter(|c| c.pk > 0)
        .map(|c| (c.table.clone(), c.name.clone()))
        .collect();

    let (columns, primary_keys) = process_columns(&raw_columns, &generated_columns, &pk_columns);
    let indexes = process_indexes(&all_indexes, &all_index_columns, &table_sql_map);
    let foreign_keys = process_foreign_keys(&all_fks);

    let mut ddl = SQLiteDDL::new();

    for (table_name, table_sql) in &tables {
        let mut table = Table::new(table_name.clone());
        if let Some(sql) = table_sql {
            let sql_upper = sql.to_uppercase();
            table.strict = sql_upper.contains(" STRICT");
            table.without_rowid = sql_upper.contains("WITHOUT ROWID");
        }
        ddl.tables.push(table);
    }

    for col in columns {
        ddl.columns.push(col);
    }
    for idx in indexes {
        ddl.indexes.push(idx);
    }
    for fk in foreign_keys {
        ddl.fks.push(fk);
    }
    for pk in primary_keys {
        ddl.pks.push(pk);
    }

    let options = CodegenOptions {
        module_doc: Some(format!("Schema introspected from Turso: {}", url)),
        include_schema: true,
        schema_name: "Schema".to_string(),
        use_pub: true,
    };

    let generated = generate_rust_schema(&ddl, &options);

    // Create snapshot from DDL
    let mut sqlite_snapshot = drizzle_migrations::sqlite::SQLiteSnapshot::new();
    for entity in ddl.to_entities() {
        sqlite_snapshot.add_entity(entity);
    }
    let snapshot = Snapshot::Sqlite(sqlite_snapshot);

    Ok(IntrospectResult {
        schema_code: generated.code,
        table_count: generated.tables.len(),
        index_count: generated.indexes.len(),
        view_count: ddl.views.len(),
        warnings: generated.warnings,
        snapshot,
        snapshot_path: std::path::PathBuf::new(),
    })
}

// ============================================================================
// PostgreSQL Introspection
// ============================================================================

#[cfg(feature = "postgres-sync")]
fn introspect_postgres_sync(_creds: &PostgresCreds) -> Result<IntrospectResult, CliError> {
    // TODO: Implement PostgreSQL introspection
    // For now, return a stub indicating it's not yet implemented
    Err(CliError::Other(
        "PostgreSQL introspection is not yet fully implemented. \
         Use the programmatic API or wait for a future release."
            .into(),
    ))
}

#[cfg(feature = "tokio-postgres")]
#[allow(dead_code)]
fn introspect_postgres_async(_creds: &PostgresCreds) -> Result<IntrospectResult, CliError> {
    // TODO: Implement PostgreSQL introspection
    Err(CliError::Other(
        "PostgreSQL introspection is not yet fully implemented. \
         Use the programmatic API or wait for a future release."
            .into(),
    ))
}

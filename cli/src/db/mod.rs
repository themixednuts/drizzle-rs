//! Database connection and migration execution for CLI commands
//!
//! This module provides database connectivity for running migrations and other
//! database operations from the CLI.

use std::path::Path;

#[cfg(any(feature = "postgres-sync", feature = "tokio-postgres"))]
use crate::config::PostgresCreds;
use crate::config::{Credentials, Dialect};
use crate::error::CliError;
use crate::output;
use drizzle_migrations::MigrationSet;
use drizzle_migrations::schema::Snapshot;

/// Result of a migration run
#[derive(Debug)]
pub struct MigrationResult {
    /// Number of migrations applied
    pub applied_count: usize,
    /// Tags of applied migrations
    pub applied_migrations: Vec<String>,
}

/// Planned SQL changes for `drizzle push`
#[derive(Debug, Clone)]
pub struct PushPlan {
    pub sql_statements: Vec<String>,
    pub warnings: Vec<String>,
    pub destructive: bool,
}

/// Plan a push by introspecting the live database and diffing against the desired snapshot.
pub fn plan_push(
    credentials: &Credentials,
    dialect: Dialect,
    desired: &Snapshot,
    breakpoints: bool,
) -> Result<PushPlan, CliError> {
    let current = introspect_database(credentials, dialect)?.snapshot;
    let (sql_statements, warnings) = generate_push_sql(&current, desired, breakpoints)?;
    let destructive = sql_statements.iter().any(|s| is_destructive_statement(s));

    Ok(PushPlan {
        sql_statements,
        warnings,
        destructive,
    })
}

/// Apply a previously planned push.
pub fn apply_push(
    credentials: &Credentials,
    dialect: Dialect,
    plan: &PushPlan,
    force: bool,
) -> Result<(), CliError> {
    if plan.sql_statements.is_empty() {
        return Ok(());
    }

    if plan.destructive && !force {
        let confirmed = confirm_destructive()?;
        if !confirmed {
            return Ok(());
        }
    }

    execute_statements(credentials, dialect, &plan.sql_statements)
}

/// Execute migrations against the database
///
/// This is the main entry point that dispatches to the appropriate driver
/// based on the credentials type.
pub fn run_migrations(
    credentials: &Credentials,
    dialect: Dialect,
    migrations_dir: &Path,
    migrations_table: &str,
    migrations_schema: &str,
) -> Result<MigrationResult, CliError> {
    // Load migrations from filesystem
    let mut set = MigrationSet::from_dir(migrations_dir, dialect.to_base())
        .map_err(|e| CliError::Other(format!("Failed to load migrations: {}", e)))?;

    // Apply overrides from config
    if !migrations_table.trim().is_empty() {
        set = set.with_table(migrations_table.to_string());
    }
    if dialect == Dialect::Postgresql && !migrations_schema.trim().is_empty() {
        set = set.with_schema(migrations_schema.to_string());
    }

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

        #[cfg(any(feature = "libsql", feature = "turso"))]
        Credentials::Turso { url, auth_token } => {
            let _auth_token = auth_token.as_deref();
            if is_local_libsql(url) {
                #[cfg(feature = "libsql")]
                {
                    run_libsql_local_migrations(&set, url)
                }
                #[cfg(not(feature = "libsql"))]
                {
                    Err(CliError::MissingDriver {
                        dialect: "LibSQL (local)",
                        feature: "libsql",
                    })
                }
            } else {
                #[cfg(feature = "turso")]
                {
                    run_turso_migrations(&set, url, _auth_token)
                }
                #[cfg(not(feature = "turso"))]
                {
                    Err(CliError::MissingDriver {
                        dialect: "Turso (remote)",
                        feature: "turso",
                    })
                }
            }
        }

        #[cfg(all(not(feature = "turso"), not(feature = "libsql")))]
        Credentials::Turso { .. } => Err(CliError::MissingDriver {
            dialect: "Turso",
            feature: "turso or libsql",
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
    }
}

fn is_destructive_statement(sql: &str) -> bool {
    let s = sql.trim().to_uppercase();
    s.contains("DROP TABLE")
        || s.contains("DROP COLUMN")
        || s.contains("DROP INDEX")
        || s.contains("TRUNCATE")
        || (s.contains("ALTER TABLE") && s.contains(" DROP "))
}

fn confirm_destructive() -> Result<bool, CliError> {
    use std::io::{self, IsTerminal, Write};

    if !io::stdin().is_terminal() {
        return Err(CliError::Other(
            "Refusing to apply potentially destructive changes in non-interactive mode. Use --explain or --force."
                .into(),
        ));
    }

    println!(
        "{}",
        output::warning(
            "Potentially destructive changes detected (DROP/TRUNCATE/etc)."
        )
    );
    print!("Apply anyway? [y/N]: ");
    io::stdout()
        .flush()
        .map_err(|e| CliError::IoError(e.to_string()))?;

    let mut line = String::new();
    io::stdin()
        .read_line(&mut line)
        .map_err(|e| CliError::IoError(e.to_string()))?;
    let ans = line.trim().to_ascii_lowercase();
    Ok(ans == "y" || ans == "yes")
}

fn generate_push_sql(
    current: &Snapshot,
    desired: &Snapshot,
    breakpoints: bool,
) -> Result<(Vec<String>, Vec<String>), CliError> {
    match (current, desired) {
        (Snapshot::Sqlite(prev_snap), Snapshot::Sqlite(curr_snap)) => {
            use drizzle_migrations::sqlite::collection::SQLiteDDL;
            use drizzle_migrations::sqlite::diff::compute_migration;

            let prev_ddl = SQLiteDDL::from_entities(prev_snap.ddl.clone());
            let cur_ddl = SQLiteDDL::from_entities(curr_snap.ddl.clone());

            let diff = compute_migration(&prev_ddl, &cur_ddl);
            Ok((diff.sql_statements, diff.warnings))
        }
        (Snapshot::Postgres(prev_snap), Snapshot::Postgres(curr_snap)) => {
            use drizzle_migrations::postgres::diff_full_snapshots;
            use drizzle_migrations::postgres::statements::PostgresGenerator;

            let diff = diff_full_snapshots(prev_snap, curr_snap);
            let generator = PostgresGenerator::new().with_breakpoints(breakpoints);
            Ok((generator.generate(&diff.diffs), Vec::new()))
        }
        _ => Err(CliError::DialectMismatch),
    }
}

fn execute_statements(
    credentials: &Credentials,
    _dialect: Dialect,
    statements: &[String],
) -> Result<(), CliError> {
    // In some feature combinations (no drivers), the match arms that would use `statements`
    // are compiled out. Touch it to avoid unused-parameter warnings.
    let _ = statements;

    match credentials {
        #[cfg(feature = "rusqlite")]
        Credentials::Sqlite { path } => execute_sqlite_statements(path, statements),

        #[cfg(not(feature = "rusqlite"))]
        Credentials::Sqlite { .. } => Err(CliError::MissingDriver {
            dialect: "SQLite",
            feature: "rusqlite",
        }),

        #[cfg(any(feature = "libsql", feature = "turso"))]
        Credentials::Turso { url, auth_token } => {
            let _auth_token = auth_token.as_deref();
            if is_local_libsql(url) {
                #[cfg(feature = "libsql")]
                {
                    execute_libsql_local_statements(url, statements)
                }
                #[cfg(not(feature = "libsql"))]
                {
                    Err(CliError::MissingDriver {
                        dialect: "LibSQL (local)",
                        feature: "libsql",
                    })
                }
            } else {
                #[cfg(feature = "turso")]
                {
                    execute_turso_statements(url, _auth_token, statements)
                }
                #[cfg(not(feature = "turso"))]
                {
                    Err(CliError::MissingDriver {
                        dialect: "Turso (remote)",
                        feature: "turso",
                    })
                }
            }
        }

        #[cfg(all(not(feature = "turso"), not(feature = "libsql")))]
        Credentials::Turso { .. } => Err(CliError::MissingDriver {
            dialect: "Turso",
            feature: "turso or libsql",
        }),

        #[cfg(feature = "postgres-sync")]
        Credentials::Postgres(creds) => execute_postgres_sync_statements(creds, statements),

        #[cfg(all(not(feature = "postgres-sync"), feature = "tokio-postgres"))]
        Credentials::Postgres(creds) => execute_postgres_async_statements(creds, statements),

        #[cfg(all(not(feature = "postgres-sync"), not(feature = "tokio-postgres")))]
        Credentials::Postgres(_) => Err(CliError::MissingDriver {
            dialect: "PostgreSQL",
            feature: "postgres-sync or tokio-postgres",
        }),
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

#[cfg(any(feature = "rusqlite", feature = "libsql", feature = "turso"))]
fn process_sqlite_uniques_from_indexes(
    raw_indexes: &[drizzle_migrations::sqlite::introspect::RawIndexInfo],
    index_columns: &[drizzle_migrations::sqlite::introspect::RawIndexColumn],
) -> Vec<drizzle_migrations::sqlite::UniqueConstraint> {
    use drizzle_migrations::sqlite::UniqueConstraint;

    let mut uniques = Vec::new();

    for idx in raw_indexes.iter().filter(|i| i.origin == "u") {
        let mut cols: Vec<(i32, String)> = index_columns
            .iter()
            .filter(|c| c.index_name == idx.name && c.key)
            .filter_map(|c| c.name.clone().map(|n| (c.seqno, n)))
            .collect();
        cols.sort_by_key(|(seq, _)| *seq);
        let col_names: Vec<String> = cols.into_iter().map(|(_, n)| n).collect();
        if col_names.is_empty() {
            continue;
        }

        let name_explicit = !idx.name.starts_with("sqlite_autoindex_");
        let constraint_name = if name_explicit {
            idx.name.clone()
        } else {
            let refs: Vec<&str> = col_names.iter().map(String::as_str).collect();
            drizzle_migrations::sqlite::ddl::name_for_unique(&idx.table, &refs)
        };

        let mut uniq = UniqueConstraint::from_strings(idx.table.clone(), constraint_name, col_names);
        uniq.name_explicit = name_explicit;
        uniques.push(uniq);
    }

    uniques
}

// ============================================================================
// SQLite (rusqlite)
// ============================================================================

#[cfg(feature = "rusqlite")]
fn execute_sqlite_statements(path: &str, statements: &[String]) -> Result<(), CliError> {
    let conn = rusqlite::Connection::open(path).map_err(|e| {
        CliError::ConnectionError(format!("Failed to open SQLite database '{}': {}", path, e))
    })?;

    conn.execute("BEGIN", [])
        .map_err(|e| CliError::MigrationError(e.to_string()))?;

    for stmt in statements {
        let s = stmt.trim();
        if s.is_empty() {
            continue;
        }
        if let Err(e) = conn.execute(s, []) {
            let _ = conn.execute("ROLLBACK", []);
            return Err(CliError::MigrationError(format!(
                "Statement failed: {}\n{}",
                e, s
            )));
        }
    }

    conn.execute("COMMIT", [])
        .map_err(|e| CliError::MigrationError(e.to_string()))?;

    Ok(())
}

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
fn execute_postgres_sync_statements(
    creds: &PostgresCreds,
    statements: &[String],
) -> Result<(), CliError> {
    let url = creds.connection_url();
    let mut client = postgres::Client::connect(&url, postgres::NoTls).map_err(|e| {
        CliError::ConnectionError(format!("Failed to connect to PostgreSQL: {}", e))
    })?;

    let mut tx = client
        .transaction()
        .map_err(|e| CliError::MigrationError(e.to_string()))?;

    for stmt in statements {
        let s = stmt.trim();
        if s.is_empty() {
            continue;
        }
        tx.execute(s, &[]).map_err(|e| {
            CliError::MigrationError(format!("Statement failed: {}\n{}", e, s))
        })?;
    }

    tx.commit()
        .map_err(|e| CliError::MigrationError(e.to_string()))?;

    Ok(())
}

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
fn execute_postgres_async_statements(
    creds: &PostgresCreds,
    statements: &[String],
) -> Result<(), CliError> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| CliError::Other(format!("Failed to create async runtime: {}", e)))?;

    rt.block_on(execute_postgres_async_inner(creds, statements))
}

#[cfg(feature = "tokio-postgres")]
async fn execute_postgres_async_inner(
    creds: &PostgresCreds,
    statements: &[String],
) -> Result<(), CliError> {
    let url = creds.connection_url();
    let (mut client, connection) = tokio_postgres::connect(&url, tokio_postgres::NoTls)
        .await
        .map_err(|e| CliError::ConnectionError(format!("Failed to connect to PostgreSQL: {}", e)))?;

    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!(
                "{}",
                output::err_line(&format!("PostgreSQL connection error: {e}"))
            );
        }
    });

    let tx = client
        .transaction()
        .await
        .map_err(|e| CliError::MigrationError(e.to_string()))?;

    for stmt in statements {
        let s = stmt.trim();
        if s.is_empty() {
            continue;
        }
        tx.execute(s, &[]).await.map_err(|e| {
            CliError::MigrationError(format!("Statement failed: {}\n{}", e, s))
        })?;
    }

    tx.commit()
        .await
        .map_err(|e| CliError::MigrationError(e.to_string()))?;

    Ok(())
}

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
            eprintln!(
                "{}",
                output::err_line(&format!("PostgreSQL connection error: {e}"))
            );
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
fn execute_libsql_local_statements(path: &str, statements: &[String]) -> Result<(), CliError> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| CliError::Other(format!("Failed to create async runtime: {}", e)))?;

    rt.block_on(execute_libsql_local_inner(path, statements))
}

#[cfg(feature = "libsql")]
async fn execute_libsql_local_inner(path: &str, statements: &[String]) -> Result<(), CliError> {
    let db = libsql::Builder::new_local(path)
        .build()
        .await
        .map_err(|e| {
            CliError::ConnectionError(format!("Failed to open LibSQL database '{}': {}", path, e))
        })?;

    let conn = db
        .connect()
        .map_err(|e| CliError::ConnectionError(e.to_string()))?;

    let tx = conn
        .transaction()
        .await
        .map_err(|e| CliError::MigrationError(e.to_string()))?;

    for stmt in statements {
        let s = stmt.trim();
        if s.is_empty() {
            continue;
        }
        if let Err(e) = tx.execute(s, ()).await {
            tx.rollback().await.ok();
            return Err(CliError::MigrationError(format!(
                "Statement failed: {}\n{}",
                e, s
            )));
        }
    }

    tx.commit()
        .await
        .map_err(|e| CliError::MigrationError(e.to_string()))?;

    Ok(())
}

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
fn execute_turso_statements(
    url: &str,
    auth_token: Option<&str>,
    statements: &[String],
) -> Result<(), CliError> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| CliError::Other(format!("Failed to create async runtime: {}", e)))?;

    rt.block_on(execute_turso_inner(url, auth_token, statements))
}

#[cfg(feature = "turso")]
async fn execute_turso_inner(
    url: &str,
    auth_token: Option<&str>,
    statements: &[String],
) -> Result<(), CliError> {
    let builder =
        libsql::Builder::new_remote(url.to_string(), auth_token.unwrap_or("").to_string());

    let db = builder.build().await.map_err(|e| {
        CliError::ConnectionError(format!("Failed to connect to Turso '{}': {}", url, e))
    })?;

    let conn = db
        .connect()
        .map_err(|e| CliError::ConnectionError(e.to_string()))?;

    let tx = conn
        .transaction()
        .await
        .map_err(|e| CliError::MigrationError(e.to_string()))?;

    for stmt in statements {
        let s = stmt.trim();
        if s.is_empty() {
            continue;
        }
        if let Err(e) = tx.execute(s, ()).await {
            tx.rollback().await.ok();
            return Err(CliError::MigrationError(format!(
                "Statement failed: {}\n{}",
                e, s
            )));
        }
    }

    tx.commit()
        .await
        .map_err(|e| CliError::MigrationError(e.to_string()))?;

    Ok(())
}

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
    init_metadata: bool,
    migrations_table: &str,
    migrations_schema: &str,
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
    journal
        .save(&journal_path)
        .map_err(|e| CliError::Other(format!("Failed to save journal: {}", e)))?;

    if init_metadata {
        apply_init_metadata(
            credentials,
            dialect,
            out_dir,
            migrations_table,
            migrations_schema,
        )?;
    }

    Ok(result)
}

// =============================================================================
// Init metadata handling
// =============================================================================

fn apply_init_metadata(
    credentials: &Credentials,
    dialect: Dialect,
    out_dir: &Path,
    migrations_table: &str,
    migrations_schema: &str,
) -> Result<(), CliError> {
    use drizzle_migrations::MigrationSet;

    let mut set = MigrationSet::from_dir(out_dir, dialect.to_base())
        .map_err(|e| CliError::Other(format!("Failed to load migrations: {}", e)))?;

    if !migrations_table.trim().is_empty() {
        set = set.with_table(migrations_table.to_string());
    }
    if dialect == Dialect::Postgresql && !migrations_schema.trim().is_empty() {
        set = set.with_schema(migrations_schema.to_string());
    }

    if set.all().is_empty() {
        return Err(CliError::Other(
            "--init can't be used with empty migrations".into(),
        ));
    }

    match credentials {
        #[cfg(feature = "rusqlite")]
        Credentials::Sqlite { path } => init_sqlite_metadata(path, &set),

        #[cfg(not(feature = "rusqlite"))]
        Credentials::Sqlite { .. } => Err(CliError::MissingDriver {
            dialect: "SQLite",
            feature: "rusqlite",
        }),

        #[cfg(any(feature = "libsql", feature = "turso"))]
        Credentials::Turso { url, auth_token } => {
            let _auth_token = auth_token.as_deref();
            if is_local_libsql(url) {
                #[cfg(feature = "libsql")]
                {
                    init_libsql_local_metadata(url, &set)
                }
                #[cfg(not(feature = "libsql"))]
                {
                    Err(CliError::MissingDriver {
                        dialect: "LibSQL (local)",
                        feature: "libsql",
                    })
                }
            } else {
                #[cfg(feature = "turso")]
                {
                    init_turso_metadata(url, _auth_token, &set)
                }
                #[cfg(not(feature = "turso"))]
                {
                    Err(CliError::MissingDriver {
                        dialect: "Turso (remote)",
                        feature: "turso",
                    })
                }
            }
        }

        #[cfg(all(not(feature = "turso"), not(feature = "libsql")))]
        Credentials::Turso { .. } => Err(CliError::MissingDriver {
            dialect: "Turso",
            feature: "turso or libsql",
        }),

        #[cfg(feature = "postgres-sync")]
        Credentials::Postgres(creds) => init_postgres_sync_metadata(creds, &set),

        #[cfg(all(not(feature = "postgres-sync"), feature = "tokio-postgres"))]
        Credentials::Postgres(creds) => init_postgres_async_metadata(creds, &set),

        #[cfg(all(not(feature = "postgres-sync"), not(feature = "tokio-postgres")))]
        Credentials::Postgres(_) => Err(CliError::MissingDriver {
            dialect: "PostgreSQL",
            feature: "postgres-sync or tokio-postgres",
        }),
    }
}

#[cfg(any(
    feature = "rusqlite",
    feature = "libsql",
    feature = "turso",
    feature = "postgres-sync",
    feature = "tokio-postgres"
))]
fn validate_init_metadata(applied_hashes: &[String], set: &MigrationSet) -> Result<(), CliError> {
    if !applied_hashes.is_empty() {
        return Err(CliError::Other(
            "--init can't be used when database already has migrations set".into(),
        ));
    }

    let first = set
        .all()
        .first()
        .ok_or_else(|| CliError::Other("--init can't be used with empty migrations".into()))?;

    let created_at = first.created_at();
    if set.all().iter().any(|m| m.created_at() != created_at) {
        return Err(CliError::Other(
            "--init can't be used with existing migrations".into(),
        ));
    }

    Ok(())
}

// =============================================================================
// Init metadata implementations
// =============================================================================

#[cfg(feature = "rusqlite")]
fn init_sqlite_metadata(path: &str, set: &MigrationSet) -> Result<(), CliError> {
    let conn = rusqlite::Connection::open(path).map_err(|e| {
        CliError::ConnectionError(format!("Failed to open SQLite database '{}': {}", path, e))
    })?;

    conn.execute(&set.create_table_sql(), []).map_err(|e| {
        CliError::MigrationError(format!("Failed to create migrations table: {}", e))
    })?;

    let applied_hashes = query_applied_hashes_sqlite(&conn, set)?;
    validate_init_metadata(&applied_hashes, set)?;

    let first = set
        .all()
        .first()
        .ok_or_else(|| CliError::Other("--init can't be used with empty migrations".into()))?;

    conn.execute(
        &set.record_migration_sql(first.hash(), first.created_at()),
        [],
    )
    .map_err(|e| CliError::MigrationError(e.to_string()))?;

    Ok(())
}

#[cfg(feature = "libsql")]
fn init_libsql_local_metadata(path: &str, set: &MigrationSet) -> Result<(), CliError> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| CliError::Other(format!("Failed to create async runtime: {}", e)))?;

    rt.block_on(init_libsql_local_metadata_inner(path, set))
}

#[cfg(feature = "libsql")]
async fn init_libsql_local_metadata_inner(path: &str, set: &MigrationSet) -> Result<(), CliError> {
    let db = libsql::Builder::new_local(path)
        .build()
        .await
        .map_err(|e| {
            CliError::ConnectionError(format!("Failed to open LibSQL database '{}': {}", path, e))
        })?;

    let conn = db
        .connect()
        .map_err(|e| CliError::ConnectionError(e.to_string()))?;

    conn.execute(&set.create_table_sql(), ())
        .await
        .map_err(|e| {
            CliError::MigrationError(format!("Failed to create migrations table: {}", e))
        })?;

    let applied_hashes = query_applied_hashes_libsql(&conn, set).await?;
    validate_init_metadata(&applied_hashes, set)?;

    let first = set
        .all()
        .first()
        .ok_or_else(|| CliError::Other("--init can't be used with empty migrations".into()))?;

    conn.execute(
        &set.record_migration_sql(first.hash(), first.created_at()),
        (),
    )
    .await
    .map_err(|e| CliError::MigrationError(e.to_string()))?;

    Ok(())
}

#[cfg(feature = "turso")]
fn init_turso_metadata(
    url: &str,
    auth_token: Option<&str>,
    set: &MigrationSet,
) -> Result<(), CliError> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| CliError::Other(format!("Failed to create async runtime: {}", e)))?;

    rt.block_on(init_turso_metadata_inner(url, auth_token, set))
}

#[cfg(feature = "turso")]
async fn init_turso_metadata_inner(
    url: &str,
    auth_token: Option<&str>,
    set: &MigrationSet,
) -> Result<(), CliError> {
    let builder =
        libsql::Builder::new_remote(url.to_string(), auth_token.unwrap_or("").to_string());

    let db = builder.build().await.map_err(|e| {
        CliError::ConnectionError(format!("Failed to connect to Turso '{}': {}", url, e))
    })?;

    let conn = db
        .connect()
        .map_err(|e| CliError::ConnectionError(e.to_string()))?;

    conn.execute(&set.create_table_sql(), ())
        .await
        .map_err(|e| {
            CliError::MigrationError(format!("Failed to create migrations table: {}", e))
        })?;

    let applied_hashes = query_applied_hashes_turso(&conn, set).await?;
    validate_init_metadata(&applied_hashes, set)?;

    let first = set
        .all()
        .first()
        .ok_or_else(|| CliError::Other("--init can't be used with empty migrations".into()))?;

    conn.execute(
        &set.record_migration_sql(first.hash(), first.created_at()),
        (),
    )
    .await
    .map_err(|e| CliError::MigrationError(e.to_string()))?;

    Ok(())
}

#[cfg(feature = "postgres-sync")]
fn init_postgres_sync_metadata(creds: &PostgresCreds, set: &MigrationSet) -> Result<(), CliError> {
    let url = creds.connection_url();
    let mut client = postgres::Client::connect(&url, postgres::NoTls).map_err(|e| {
        CliError::ConnectionError(format!("Failed to connect to PostgreSQL: {}", e))
    })?;

    if let Some(schema_sql) = set.create_schema_sql() {
        client
            .execute(&schema_sql, &[])
            .map_err(|e| CliError::MigrationError(e.to_string()))?;
    }

    client
        .execute(&set.create_table_sql(), &[])
        .map_err(|e| CliError::MigrationError(e.to_string()))?;

    let rows = client
        .query(&set.query_all_hashes_sql(), &[])
        .unwrap_or_default();
    let applied_hashes: Vec<String> = rows.iter().map(|r| r.get(0)).collect();

    validate_init_metadata(&applied_hashes, set)?;

    let first = set
        .all()
        .first()
        .ok_or_else(|| CliError::Other("--init can't be used with empty migrations".into()))?;

    client
        .execute(
            &set.record_migration_sql(first.hash(), first.created_at()),
            &[],
        )
        .map_err(|e| CliError::MigrationError(e.to_string()))?;

    Ok(())
}

#[cfg(feature = "tokio-postgres")]
fn init_postgres_async_metadata(creds: &PostgresCreds, set: &MigrationSet) -> Result<(), CliError> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| CliError::Other(format!("Failed to create async runtime: {}", e)))?;

    rt.block_on(init_postgres_async_inner(creds, set))
}

#[cfg(feature = "tokio-postgres")]
async fn init_postgres_async_inner(
    creds: &PostgresCreds,
    set: &MigrationSet,
) -> Result<(), CliError> {
    let url = creds.connection_url();
    let (client, connection) = tokio_postgres::connect(&url, tokio_postgres::NoTls)
        .await
        .map_err(|e| {
            CliError::ConnectionError(format!("Failed to connect to PostgreSQL: {}", e))
        })?;

    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!(
                "{}",
                output::err_line(&format!("PostgreSQL connection error: {e}"))
            );
        }
    });

    if let Some(schema_sql) = set.create_schema_sql() {
        client
            .execute(&schema_sql, &[])
            .await
            .map_err(|e| CliError::MigrationError(e.to_string()))?;
    }

    client
        .execute(&set.create_table_sql(), &[])
        .await
        .map_err(|e| CliError::MigrationError(e.to_string()))?;

    let rows = client
        .query(&set.query_all_hashes_sql(), &[])
        .await
        .unwrap_or_default();
    let applied_hashes: Vec<String> = rows.iter().map(|r| r.get(0)).collect();

    validate_init_metadata(&applied_hashes, set)?;

    let first = set
        .all()
        .first()
        .ok_or_else(|| CliError::Other("--init can't be used with empty migrations".into()))?;

    client
        .execute(
            &set.record_migration_sql(first.hash(), first.created_at()),
            &[],
        )
        .await
        .map_err(|e| CliError::MigrationError(e.to_string()))?;

    Ok(())
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

        #[cfg(any(feature = "libsql", feature = "turso"))]
        Credentials::Turso { url, auth_token } => {
            let _auth_token = auth_token.as_deref();
            if is_local_libsql(url) {
                #[cfg(feature = "libsql")]
                {
                    introspect_libsql_local(url)
                }
                #[cfg(not(feature = "libsql"))]
                {
                    Err(CliError::MissingDriver {
                        dialect: "LibSQL (local)",
                        feature: "libsql",
                    })
                }
            } else {
                #[cfg(feature = "turso")]
                {
                    introspect_turso(url, _auth_token)
                }
                #[cfg(not(feature = "turso"))]
                {
                    Err(CliError::MissingDriver {
                        dialect: "Turso (remote)",
                        feature: "turso",
                    })
                }
            }
        }

        #[cfg(all(not(feature = "turso"), not(feature = "libsql")))]
        Credentials::Turso { .. } => Err(CliError::MissingDriver {
            dialect: "Turso",
            feature: "turso or libsql",
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
        SQLiteDDL, Table, View,
        codegen::{CodegenOptions, generate_rust_schema},
        introspect::{
            parse_generated_columns_from_table_sql, parse_view_sql, RawColumnInfo, RawForeignKey,
            RawIndexColumn, RawIndexInfo, RawViewInfo, process_columns, process_foreign_keys,
            process_indexes, queries,
        },
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
                cid: row.get(1)?,
                name: row.get(2)?,
                column_type: row.get(3)?,
                not_null: row.get(4)?,
                default_value: row.get(5)?,
                pk: row.get(6)?,
                hidden: row.get(7)?,
                sql: row.get(8)?,
            })
        })
        .map_err(|e| CliError::Other(e.to_string()))?
        .filter_map(Result::ok)
        .collect();

    // Query indexes and foreign keys for each table
    let mut all_indexes: Vec<RawIndexInfo> = Vec::new();
    let mut all_index_columns: Vec<RawIndexColumn> = Vec::new();
    let mut all_fks: Vec<RawForeignKey> = Vec::new();
    let mut all_views: Vec<RawViewInfo> = Vec::new();

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
                if let Ok(mut col_stmt) = conn.prepare(&queries::index_info_query(&idx.name))
                    && let Ok(col_iter) = col_stmt.query_map([], |row| {
                        Ok(RawIndexColumn {
                            index_name: idx.name.clone(),
                            seqno: row.get(0)?,
                            cid: row.get(1)?,
                            name: row.get(2)?,
                            desc: row.get::<_, i32>(3)? != 0,
                            coll: row.get(4)?,
                            key: row.get::<_, i32>(5)? != 0,
                        })
                    })
                {
                    all_index_columns.extend(col_iter.filter_map(Result::ok));
                }
            }
            all_indexes.extend(indexes);
        }

        // Foreign keys
        if let Ok(mut fk_stmt) = conn.prepare(&queries::foreign_keys_query(table_name))
            && let Ok(fk_iter) = fk_stmt.query_map([], |row| {
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
            })
        {
            all_fks.extend(fk_iter.filter_map(Result::ok));
        }
    }

    // Views
    if let Ok(mut views_stmt) = conn.prepare(queries::VIEWS_QUERY)
        && let Ok(view_iter) = views_stmt.query_map([], |row| {
            Ok(RawViewInfo {
                name: row.get(0)?,
                sql: row.get(1)?,
            })
        })
    {
        all_views.extend(view_iter.filter_map(Result::ok));
    }

    // Process raw data into DDL entities
    let mut generated_columns: HashMap<String, drizzle_migrations::sqlite::ddl::ParsedGenerated> =
        HashMap::new();
    for (table, sql) in &table_sql_map {
        generated_columns.extend(parse_generated_columns_from_table_sql(table, sql));
    }
    let pk_columns: HashSet<(String, String)> = raw_columns
        .iter()
        .filter(|c| c.pk > 0)
        .map(|c| (c.table.clone(), c.name.clone()))
        .collect();

    let (columns, primary_keys) = process_columns(&raw_columns, &generated_columns, &pk_columns);
    let indexes = process_indexes(&all_indexes, &all_index_columns, &table_sql_map);
    let foreign_keys = process_foreign_keys(&all_fks);

    // Unique constraints (origin == 'u' indexes)
    let uniques = process_sqlite_uniques_from_indexes(&all_indexes, &all_index_columns);

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

    for u in uniques {
        ddl.uniques.push(u);
    }

    // Views
    for v in all_views {
        let mut view = View::new(v.name);
        if let Some(def) = parse_view_sql(&v.sql) {
            view.definition = Some(def.into());
        } else {
            view.error = Some("Failed to parse view SQL".into());
        }
        ddl.views.push(view);
    }

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
        SQLiteDDL, Table, View,
        codegen::{CodegenOptions, generate_rust_schema},
        introspect::{
            parse_generated_columns_from_table_sql, parse_view_sql, RawColumnInfo, RawForeignKey,
            RawIndexColumn, RawIndexInfo, RawViewInfo, process_columns, process_foreign_keys,
            process_indexes, queries,
        },
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
            cid: row.get(1).unwrap_or(0),
            name: row.get(2).unwrap_or_default(),
            column_type: row.get(3).unwrap_or_default(),
            not_null: row.get::<i32>(4).unwrap_or(0) != 0,
            default_value: row.get(5).ok(),
            pk: row.get(6).unwrap_or(0),
            hidden: row.get(7).unwrap_or(0),
            sql: row.get(8).ok(),
        });
    }

    // Query indexes and foreign keys
    let mut all_indexes: Vec<RawIndexInfo> = Vec::new();
    let mut all_index_columns: Vec<RawIndexColumn> = Vec::new();
    let mut all_fks: Vec<RawForeignKey> = Vec::new();
    let mut all_views: Vec<RawViewInfo> = Vec::new();

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
                if let Ok(mut col_rows) =
                    conn.query(&queries::index_info_query(&idx.name), ()).await
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
        if let Ok(mut fk_rows) = conn
            .query(&queries::foreign_keys_query(table_name), ())
            .await
        {
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

    // Views
    if let Ok(mut views_rows) = conn.query(queries::VIEWS_QUERY, ()).await {
        while let Ok(Some(row)) = views_rows.next().await {
            let name: String = row.get(0).unwrap_or_default();
            let sql: String = row.get(1).unwrap_or_default();
            all_views.push(RawViewInfo { name, sql });
        }
    }

    // Process into DDL
    let mut generated_columns: HashMap<String, drizzle_migrations::sqlite::ddl::ParsedGenerated> =
        HashMap::new();
    for (table, sql) in &table_sql_map {
        generated_columns.extend(parse_generated_columns_from_table_sql(table, sql));
    }
    let pk_columns: HashSet<(String, String)> = raw_columns
        .iter()
        .filter(|c| c.pk > 0)
        .map(|c| (c.table.clone(), c.name.clone()))
        .collect();

    let (columns, primary_keys) = process_columns(&raw_columns, &generated_columns, &pk_columns);
    let indexes = process_indexes(&all_indexes, &all_index_columns, &table_sql_map);
    let foreign_keys = process_foreign_keys(&all_fks);
    let uniques = process_sqlite_uniques_from_indexes(&all_indexes, &all_index_columns);

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

    for u in uniques {
        ddl.uniques.push(u);
    }

    for v in all_views {
        let mut view = View::new(v.name);
        if let Some(def) = parse_view_sql(&v.sql) {
            view.definition = Some(def.into());
        } else {
            view.error = Some("Failed to parse view SQL".into());
        }
        ddl.views.push(view);
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
        SQLiteDDL, Table, View,
        codegen::{CodegenOptions, generate_rust_schema},
        introspect::{
            parse_generated_columns_from_table_sql, parse_view_sql, RawColumnInfo, RawForeignKey,
            RawIndexColumn, RawIndexInfo, RawViewInfo, process_columns, process_foreign_keys,
            process_indexes, queries,
        },
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
            cid: row.get(1).unwrap_or(0),
            name: row.get(2).unwrap_or_default(),
            column_type: row.get(3).unwrap_or_default(),
            not_null: row.get::<i32>(4).unwrap_or(0) != 0,
            default_value: row.get(5).ok(),
            pk: row.get(6).unwrap_or(0),
            hidden: row.get(7).unwrap_or(0),
            sql: row.get(8).ok(),
        });
    }

    // Query indexes and foreign keys
    let mut all_indexes: Vec<RawIndexInfo> = Vec::new();
    let mut all_index_columns: Vec<RawIndexColumn> = Vec::new();
    let mut all_fks: Vec<RawForeignKey> = Vec::new();
    let mut all_views: Vec<RawViewInfo> = Vec::new();

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
                if let Ok(mut col_rows) =
                    conn.query(&queries::index_info_query(&idx.name), ()).await
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
        if let Ok(mut fk_rows) = conn
            .query(&queries::foreign_keys_query(table_name), ())
            .await
        {
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

    // Views
    if let Ok(mut views_rows) = conn.query(queries::VIEWS_QUERY, ()).await {
        while let Ok(Some(row)) = views_rows.next().await {
            let name: String = row.get(0).unwrap_or_default();
            let sql: String = row.get(1).unwrap_or_default();
            all_views.push(RawViewInfo { name, sql });
        }
    }

    // Process into DDL
    let mut generated_columns: HashMap<String, drizzle_migrations::sqlite::ddl::ParsedGenerated> =
        HashMap::new();
    for (table, sql) in &table_sql_map {
        generated_columns.extend(parse_generated_columns_from_table_sql(table, sql));
    }
    let pk_columns: HashSet<(String, String)> = raw_columns
        .iter()
        .filter(|c| c.pk > 0)
        .map(|c| (c.table.clone(), c.name.clone()))
        .collect();

    let (columns, primary_keys) = process_columns(&raw_columns, &generated_columns, &pk_columns);
    let indexes = process_indexes(&all_indexes, &all_index_columns, &table_sql_map);
    let foreign_keys = process_foreign_keys(&all_fks);
    let uniques = process_sqlite_uniques_from_indexes(&all_indexes, &all_index_columns);

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

    for u in uniques {
        ddl.uniques.push(u);
    }

    for v in all_views {
        let mut view = View::new(v.name);
        if let Some(def) = parse_view_sql(&v.sql) {
            view.definition = Some(def.into());
        } else {
            view.error = Some("Failed to parse view SQL".into());
        }
        ddl.views.push(view);
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
fn introspect_postgres_sync(creds: &PostgresCreds) -> Result<IntrospectResult, CliError> {
    use drizzle_migrations::postgres::{
        PostgresDDL,
        codegen::{CodegenOptions, generate_rust_schema},
        ddl::Schema,
        introspect::{
            RawCheckInfo, RawColumnInfo, RawEnumInfo, RawForeignKeyInfo, RawIndexInfo, RawPolicyInfo,
            RawPrimaryKeyInfo, RawRoleInfo, RawSequenceInfo,
            RawTableInfo, RawUniqueInfo, RawViewInfo, process_check_constraints, process_columns,
            process_enums, process_foreign_keys, process_indexes, process_policies,
            process_primary_keys, process_roles, process_sequences, process_tables,
            process_unique_constraints, process_views,
        },
    };

    let url = creds.connection_url();
    let mut client = postgres::Client::connect(&url, postgres::NoTls).map_err(|e| {
        CliError::ConnectionError(format!("Failed to connect to PostgreSQL: {}", e))
    })?;

    // Schemas
    let raw_schemas: Vec<RawSchemaInfo> = client
        .query(drizzle_migrations::postgres::introspect::queries::SCHEMAS_QUERY, &[])
        .map_err(|e| CliError::Other(format!("Failed to query schemas: {}", e)))?
        .into_iter()
        .map(|row| RawSchemaInfo {
            name: row.get::<_, String>(0),
        })
        .collect();

    // Tables
    let raw_tables: Vec<RawTableInfo> = client
        .query(drizzle_migrations::postgres::introspect::queries::TABLES_QUERY, &[])
        .map_err(|e| CliError::Other(format!("Failed to query tables: {}", e)))?
        .into_iter()
        .map(|row| RawTableInfo {
            schema: row.get::<_, String>(0),
            name: row.get::<_, String>(1),
            is_rls_enabled: row.get::<_, bool>(2),
        })
        .collect();

    // Columns
    let raw_columns: Vec<RawColumnInfo> = client
        .query(drizzle_migrations::postgres::introspect::queries::COLUMNS_QUERY, &[])
        .map_err(|e| CliError::Other(format!("Failed to query columns: {}", e)))?
        .into_iter()
        .map(|row| RawColumnInfo {
            schema: row.get::<_, String>(0),
            table: row.get::<_, String>(1),
            name: row.get::<_, String>(2),
            column_type: row.get::<_, String>(3),
            type_schema: row.get::<_, Option<String>>(4),
            not_null: row.get::<_, bool>(5),
            default_value: row.get::<_, Option<String>>(6),
            is_identity: row.get::<_, bool>(7),
            identity_type: row.get::<_, Option<String>>(8),
            is_generated: row.get::<_, bool>(9),
            generated_expression: row.get::<_, Option<String>>(10),
            ordinal_position: row.get::<_, i32>(11),
        })
        .collect();

    // Enums
    let raw_enums: Vec<RawEnumInfo> = client
        .query(drizzle_migrations::postgres::introspect::queries::ENUMS_QUERY, &[])
        .map_err(|e| CliError::Other(format!("Failed to query enums: {}", e)))?
        .into_iter()
        .map(|row| RawEnumInfo {
            schema: row.get::<_, String>(0),
            name: row.get::<_, String>(1),
            values: row.get::<_, Vec<String>>(2),
        })
        .collect();

    // Sequences
    let raw_sequences: Vec<RawSequenceInfo> = client
        .query(drizzle_migrations::postgres::introspect::queries::SEQUENCES_QUERY, &[])
        .map_err(|e| CliError::Other(format!("Failed to query sequences: {}", e)))?
        .into_iter()
        .map(|row| RawSequenceInfo {
            schema: row.get::<_, String>(0),
            name: row.get::<_, String>(1),
            data_type: row.get::<_, String>(2),
            start_value: row.get::<_, String>(3),
            min_value: row.get::<_, String>(4),
            max_value: row.get::<_, String>(5),
            increment: row.get::<_, String>(6),
            cycle: row.get::<_, bool>(7),
            cache_value: row.get::<_, String>(8),
        })
        .collect();

    // Views
    let raw_views: Vec<RawViewInfo> = client
        .query(drizzle_migrations::postgres::introspect::queries::VIEWS_QUERY, &[])
        .map_err(|e| CliError::Other(format!("Failed to query views: {}", e)))?
        .into_iter()
        .map(|row| RawViewInfo {
            schema: row.get::<_, String>(0),
            name: row.get::<_, String>(1),
            definition: row.get::<_, String>(2),
            is_materialized: row.get::<_, bool>(3),
        })
        .collect();

    // Indexes (custom query; drizzle_migrations provides processing types but not SQL)
    let raw_indexes: Vec<RawIndexInfo> = client
        .query(POSTGRES_INDEXES_QUERY, &[])
        .map_err(|e| CliError::Other(format!("Failed to query indexes: {}", e)))?
        .into_iter()
        .map(|row| {
            let cols: Vec<String> = row.get(6);
            RawIndexInfo {
                schema: row.get::<_, String>(0),
                table: row.get::<_, String>(1),
                name: row.get::<_, String>(2),
                is_unique: row.get::<_, bool>(3),
                is_primary: row.get::<_, bool>(4),
                method: row.get::<_, String>(5),
                columns: parse_postgres_index_columns(cols),
                where_clause: row.get::<_, Option<String>>(7),
                concurrent: false,
            }
        })
        .collect();

    // Foreign keys
    let raw_fks: Vec<RawForeignKeyInfo> = client
        .query(POSTGRES_FOREIGN_KEYS_QUERY, &[])
        .map_err(|e| CliError::Other(format!("Failed to query foreign keys: {}", e)))?
        .into_iter()
        .map(|row| RawForeignKeyInfo {
            schema: row.get::<_, String>(0),
            table: row.get::<_, String>(1),
            name: row.get::<_, String>(2),
            columns: row.get::<_, Vec<String>>(3),
            schema_to: row.get::<_, String>(4),
            table_to: row.get::<_, String>(5),
            columns_to: row.get::<_, Vec<String>>(6),
            on_update: pg_action_code_to_string(row.get::<_, String>(7)),
            on_delete: pg_action_code_to_string(row.get::<_, String>(8)),
        })
        .collect();

    // Primary keys
    let raw_pks: Vec<RawPrimaryKeyInfo> = client
        .query(POSTGRES_PRIMARY_KEYS_QUERY, &[])
        .map_err(|e| CliError::Other(format!("Failed to query primary keys: {}", e)))?
        .into_iter()
        .map(|row| RawPrimaryKeyInfo {
            schema: row.get::<_, String>(0),
            table: row.get::<_, String>(1),
            name: row.get::<_, String>(2),
            columns: row.get::<_, Vec<String>>(3),
        })
        .collect();

    // Unique constraints
    let raw_uniques: Vec<RawUniqueInfo> = client
        .query(POSTGRES_UNIQUES_QUERY, &[])
        .map_err(|e| CliError::Other(format!("Failed to query unique constraints: {}", e)))?
        .into_iter()
        .map(|row| RawUniqueInfo {
            schema: row.get::<_, String>(0),
            table: row.get::<_, String>(1),
            name: row.get::<_, String>(2),
            columns: row.get::<_, Vec<String>>(3),
            nulls_not_distinct: row.get::<_, bool>(4),
        })
        .collect();

    // Check constraints
    let raw_checks: Vec<RawCheckInfo> = client
        .query(POSTGRES_CHECKS_QUERY, &[])
        .map_err(|e| CliError::Other(format!("Failed to query check constraints: {}", e)))?
        .into_iter()
        .map(|row| RawCheckInfo {
            schema: row.get::<_, String>(0),
            table: row.get::<_, String>(1),
            name: row.get::<_, String>(2),
            expression: row.get::<_, String>(3),
        })
        .collect();

    // Roles (optional but useful for snapshot parity)
    let raw_roles: Vec<RawRoleInfo> = client
        .query(POSTGRES_ROLES_QUERY, &[])
        .map_err(|e| CliError::Other(format!("Failed to query roles: {}", e)))?
        .into_iter()
        .map(|row| RawRoleInfo {
            name: row.get::<_, String>(0),
            create_db: row.get::<_, bool>(1),
            create_role: row.get::<_, bool>(2),
            inherit: row.get::<_, bool>(3),
        })
        .collect();

    // Policies
    let raw_policies: Vec<RawPolicyInfo> = client
        .query(POSTGRES_POLICIES_QUERY, &[])
        .map_err(|e| CliError::Other(format!("Failed to query policies: {}", e)))?
        .into_iter()
        .map(|row| RawPolicyInfo {
            schema: row.get::<_, String>(0),
            table: row.get::<_, String>(1),
            name: row.get::<_, String>(2),
            as_clause: row.get::<_, String>(3),
            for_clause: row.get::<_, String>(4),
            to: row.get::<_, Vec<String>>(5),
            using: row.get::<_, Option<String>>(6),
            with_check: row.get::<_, Option<String>>(7),
        })
        .collect();

    // Process raw -> DDL entities
    let mut ddl = PostgresDDL::new();

    for s in raw_schemas.into_iter().map(|s| Schema::new(s.name)) {
        ddl.schemas.push(s);
    }
    for e in process_enums(&raw_enums) {
        ddl.enums.push(e);
    }
    for s in process_sequences(&raw_sequences) {
        ddl.sequences.push(s);
    }
    for r in process_roles(&raw_roles) {
        ddl.roles.push(r);
    }
    for p in process_policies(&raw_policies) {
        ddl.policies.push(p);
    }
    for t in process_tables(&raw_tables) {
        ddl.tables.push(t);
    }
    for c in process_columns(&raw_columns) {
        ddl.columns.push(c);
    }
    for i in process_indexes(&raw_indexes) {
        ddl.indexes.push(i);
    }
    for fk in process_foreign_keys(&raw_fks) {
        ddl.fks.push(fk);
    }
    for pk in process_primary_keys(&raw_pks) {
        ddl.pks.push(pk);
    }
    for u in process_unique_constraints(&raw_uniques) {
        ddl.uniques.push(u);
    }
    for c in process_check_constraints(&raw_checks) {
        ddl.checks.push(c);
    }
    for v in process_views(&raw_views) {
        ddl.views.push(v);
    }

    // Generate Rust schema code
    let options = CodegenOptions {
        module_doc: Some(format!("Schema introspected from {}", mask_url(&url))),
        include_schema: true,
        schema_name: "Schema".to_string(),
        use_pub: true,
    };
    let generated = generate_rust_schema(&ddl, &options);

    // Build snapshot
    let mut snap = drizzle_migrations::postgres::PostgresSnapshot::new();
    for entity in ddl.to_entities() {
        snap.add_entity(entity);
    }

    Ok(IntrospectResult {
        schema_code: generated.code,
        table_count: ddl.tables.list().len(),
        index_count: ddl.indexes.list().len(),
        view_count: ddl.views.list().len(),
        warnings: generated.warnings,
        snapshot: Snapshot::Postgres(snap),
        snapshot_path: std::path::PathBuf::new(),
    })
}

#[cfg(feature = "tokio-postgres")]
#[allow(dead_code)]
fn introspect_postgres_async(creds: &PostgresCreds) -> Result<IntrospectResult, CliError> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| CliError::Other(format!("Failed to create async runtime: {}", e)))?;

    rt.block_on(introspect_postgres_async_inner(creds))
}

#[cfg(feature = "tokio-postgres")]
async fn introspect_postgres_async_inner(creds: &PostgresCreds) -> Result<IntrospectResult, CliError> {
    use drizzle_migrations::postgres::{
        PostgresDDL,
        codegen::{CodegenOptions, generate_rust_schema},
        ddl::Schema,
        introspect::{
            RawCheckInfo, RawColumnInfo, RawEnumInfo, RawForeignKeyInfo, RawIndexInfo, RawPolicyInfo,
            RawPrimaryKeyInfo, RawRoleInfo, RawSequenceInfo, RawTableInfo, RawUniqueInfo, RawViewInfo,
            process_check_constraints, process_columns, process_enums, process_foreign_keys,
            process_indexes, process_policies, process_primary_keys, process_roles,
            process_sequences, process_tables, process_unique_constraints, process_views,
        },
    };

    let url = creds.connection_url();
    let (client, connection) = tokio_postgres::connect(&url, tokio_postgres::NoTls)
        .await
        .map_err(|e| CliError::ConnectionError(format!("Failed to connect to PostgreSQL: {}", e)))?;

    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!(
                "{}",
                output::err_line(&format!("PostgreSQL connection error: {e}"))
            );
        }
    });

    let raw_schemas: Vec<RawSchemaInfo> = client
        .query(drizzle_migrations::postgres::introspect::queries::SCHEMAS_QUERY, &[])
        .await
        .map_err(|e| CliError::Other(format!("Failed to query schemas: {}", e)))?
        .into_iter()
        .map(|row| RawSchemaInfo { name: row.get::<_, String>(0) })
        .collect();

    let raw_tables: Vec<RawTableInfo> = client
        .query(drizzle_migrations::postgres::introspect::queries::TABLES_QUERY, &[])
        .await
        .map_err(|e| CliError::Other(format!("Failed to query tables: {}", e)))?
        .into_iter()
        .map(|row| RawTableInfo {
            schema: row.get::<_, String>(0),
            name: row.get::<_, String>(1),
            is_rls_enabled: row.get::<_, bool>(2),
        })
        .collect();

    let raw_columns: Vec<RawColumnInfo> = client
        .query(drizzle_migrations::postgres::introspect::queries::COLUMNS_QUERY, &[])
        .await
        .map_err(|e| CliError::Other(format!("Failed to query columns: {}", e)))?
        .into_iter()
        .map(|row| RawColumnInfo {
            schema: row.get::<_, String>(0),
            table: row.get::<_, String>(1),
            name: row.get::<_, String>(2),
            column_type: row.get::<_, String>(3),
            type_schema: row.get::<_, Option<String>>(4),
            not_null: row.get::<_, bool>(5),
            default_value: row.get::<_, Option<String>>(6),
            is_identity: row.get::<_, bool>(7),
            identity_type: row.get::<_, Option<String>>(8),
            is_generated: row.get::<_, bool>(9),
            generated_expression: row.get::<_, Option<String>>(10),
            ordinal_position: row.get::<_, i32>(11),
        })
        .collect();

    let raw_enums: Vec<RawEnumInfo> = client
        .query(drizzle_migrations::postgres::introspect::queries::ENUMS_QUERY, &[])
        .await
        .map_err(|e| CliError::Other(format!("Failed to query enums: {}", e)))?
        .into_iter()
        .map(|row| RawEnumInfo {
            schema: row.get::<_, String>(0),
            name: row.get::<_, String>(1),
            values: row.get::<_, Vec<String>>(2),
        })
        .collect();

    let raw_sequences: Vec<RawSequenceInfo> = client
        .query(drizzle_migrations::postgres::introspect::queries::SEQUENCES_QUERY, &[])
        .await
        .map_err(|e| CliError::Other(format!("Failed to query sequences: {}", e)))?
        .into_iter()
        .map(|row| RawSequenceInfo {
            schema: row.get::<_, String>(0),
            name: row.get::<_, String>(1),
            data_type: row.get::<_, String>(2),
            start_value: row.get::<_, String>(3),
            min_value: row.get::<_, String>(4),
            max_value: row.get::<_, String>(5),
            increment: row.get::<_, String>(6),
            cycle: row.get::<_, bool>(7),
            cache_value: row.get::<_, String>(8),
        })
        .collect();

    let raw_views: Vec<RawViewInfo> = client
        .query(drizzle_migrations::postgres::introspect::queries::VIEWS_QUERY, &[])
        .await
        .map_err(|e| CliError::Other(format!("Failed to query views: {}", e)))?
        .into_iter()
        .map(|row| RawViewInfo {
            schema: row.get::<_, String>(0),
            name: row.get::<_, String>(1),
            definition: row.get::<_, String>(2),
            is_materialized: row.get::<_, bool>(3),
        })
        .collect();

    let raw_indexes: Vec<RawIndexInfo> = client
        .query(POSTGRES_INDEXES_QUERY, &[])
        .await
        .map_err(|e| CliError::Other(format!("Failed to query indexes: {}", e)))?
        .into_iter()
        .map(|row| {
            let cols: Vec<String> = row.get(6);
            RawIndexInfo {
                schema: row.get::<_, String>(0),
                table: row.get::<_, String>(1),
                name: row.get::<_, String>(2),
                is_unique: row.get::<_, bool>(3),
                is_primary: row.get::<_, bool>(4),
                method: row.get::<_, String>(5),
                columns: parse_postgres_index_columns(cols),
                where_clause: row.get::<_, Option<String>>(7),
                concurrent: false,
            }
        })
        .collect();

    let raw_fks: Vec<RawForeignKeyInfo> = client
        .query(POSTGRES_FOREIGN_KEYS_QUERY, &[])
        .await
        .map_err(|e| CliError::Other(format!("Failed to query foreign keys: {}", e)))?
        .into_iter()
        .map(|row| RawForeignKeyInfo {
            schema: row.get::<_, String>(0),
            table: row.get::<_, String>(1),
            name: row.get::<_, String>(2),
            columns: row.get::<_, Vec<String>>(3),
            schema_to: row.get::<_, String>(4),
            table_to: row.get::<_, String>(5),
            columns_to: row.get::<_, Vec<String>>(6),
            on_update: pg_action_code_to_string(row.get::<_, String>(7)),
            on_delete: pg_action_code_to_string(row.get::<_, String>(8)),
        })
        .collect();

    let raw_pks: Vec<RawPrimaryKeyInfo> = client
        .query(POSTGRES_PRIMARY_KEYS_QUERY, &[])
        .await
        .map_err(|e| CliError::Other(format!("Failed to query primary keys: {}", e)))?
        .into_iter()
        .map(|row| RawPrimaryKeyInfo {
            schema: row.get::<_, String>(0),
            table: row.get::<_, String>(1),
            name: row.get::<_, String>(2),
            columns: row.get::<_, Vec<String>>(3),
        })
        .collect();

    let raw_uniques: Vec<RawUniqueInfo> = client
        .query(POSTGRES_UNIQUES_QUERY, &[])
        .await
        .map_err(|e| CliError::Other(format!("Failed to query unique constraints: {}", e)))?
        .into_iter()
        .map(|row| RawUniqueInfo {
            schema: row.get::<_, String>(0),
            table: row.get::<_, String>(1),
            name: row.get::<_, String>(2),
            columns: row.get::<_, Vec<String>>(3),
            nulls_not_distinct: row.get::<_, bool>(4),
        })
        .collect();

    let raw_checks: Vec<RawCheckInfo> = client
        .query(POSTGRES_CHECKS_QUERY, &[])
        .await
        .map_err(|e| CliError::Other(format!("Failed to query check constraints: {}", e)))?
        .into_iter()
        .map(|row| RawCheckInfo {
            schema: row.get::<_, String>(0),
            table: row.get::<_, String>(1),
            name: row.get::<_, String>(2),
            expression: row.get::<_, String>(3),
        })
        .collect();

    let raw_roles: Vec<RawRoleInfo> = client
        .query(POSTGRES_ROLES_QUERY, &[])
        .await
        .map_err(|e| CliError::Other(format!("Failed to query roles: {}", e)))?
        .into_iter()
        .map(|row| RawRoleInfo {
            name: row.get::<_, String>(0),
            create_db: row.get::<_, bool>(1),
            create_role: row.get::<_, bool>(2),
            inherit: row.get::<_, bool>(3),
        })
        .collect();

    let raw_policies: Vec<RawPolicyInfo> = client
        .query(POSTGRES_POLICIES_QUERY, &[])
        .await
        .map_err(|e| CliError::Other(format!("Failed to query policies: {}", e)))?
        .into_iter()
        .map(|row| RawPolicyInfo {
            schema: row.get::<_, String>(0),
            table: row.get::<_, String>(1),
            name: row.get::<_, String>(2),
            as_clause: row.get::<_, String>(3),
            for_clause: row.get::<_, String>(4),
            to: row.get::<_, Vec<String>>(5),
            using: row.get::<_, Option<String>>(6),
            with_check: row.get::<_, Option<String>>(7),
        })
        .collect();

    let mut ddl = PostgresDDL::new();
    for s in raw_schemas.into_iter().map(|s| Schema::new(s.name)) {
        ddl.schemas.push(s);
    }
    for e in process_enums(&raw_enums) {
        ddl.enums.push(e);
    }
    for s in process_sequences(&raw_sequences) {
        ddl.sequences.push(s);
    }
    for r in process_roles(&raw_roles) {
        ddl.roles.push(r);
    }
    for p in process_policies(&raw_policies) {
        ddl.policies.push(p);
    }
    for t in process_tables(&raw_tables) {
        ddl.tables.push(t);
    }
    for c in process_columns(&raw_columns) {
        ddl.columns.push(c);
    }
    for i in process_indexes(&raw_indexes) {
        ddl.indexes.push(i);
    }
    for fk in process_foreign_keys(&raw_fks) {
        ddl.fks.push(fk);
    }
    for pk in process_primary_keys(&raw_pks) {
        ddl.pks.push(pk);
    }
    for u in process_unique_constraints(&raw_uniques) {
        ddl.uniques.push(u);
    }
    for c in process_check_constraints(&raw_checks) {
        ddl.checks.push(c);
    }
    for v in process_views(&raw_views) {
        ddl.views.push(v);
    }

    let options = CodegenOptions {
        module_doc: Some(format!("Schema introspected from {}", mask_url(&url))),
        include_schema: true,
        schema_name: "Schema".to_string(),
        use_pub: true,
    };
    let generated = generate_rust_schema(&ddl, &options);

    let mut snap = drizzle_migrations::postgres::PostgresSnapshot::new();
    for entity in ddl.to_entities() {
        snap.add_entity(entity);
    }

    Ok(IntrospectResult {
        schema_code: generated.code,
        table_count: ddl.tables.list().len(),
        index_count: ddl.indexes.list().len(),
        view_count: ddl.views.list().len(),
        warnings: generated.warnings,
        snapshot: Snapshot::Postgres(snap),
        snapshot_path: std::path::PathBuf::new(),
    })
}

// =============================================================================
// PostgreSQL Introspection Queries (CLI-side)
// =============================================================================

/// Minimal schema list for snapshot
#[cfg(any(feature = "postgres-sync", feature = "tokio-postgres"))]
#[derive(Debug, Clone)]
struct RawSchemaInfo {
    name: String,
}

#[cfg(any(feature = "postgres-sync", feature = "tokio-postgres"))]
const POSTGRES_INDEXES_QUERY: &str = r#"
SELECT
    ns.nspname AS schema,
    tbl.relname AS table,
    idx.relname AS name,
    ix.indisunique AS is_unique,
    ix.indisprimary AS is_primary,
    am.amname AS method,
    array_agg(pg_get_indexdef(ix.indexrelid, s.n, true) ORDER BY s.n) AS columns,
    pg_get_expr(ix.indpred, ix.indrelid) AS where_clause
FROM pg_index ix
JOIN pg_class idx ON idx.oid = ix.indexrelid
JOIN pg_class tbl ON tbl.oid = ix.indrelid
JOIN pg_namespace ns ON ns.oid = tbl.relnamespace
JOIN pg_am am ON am.oid = idx.relam
JOIN generate_series(1, ix.indnkeyatts) AS s(n) ON TRUE
WHERE ns.nspname NOT LIKE 'pg_%'
  AND ns.nspname <> 'information_schema'
GROUP BY ns.nspname, tbl.relname, idx.relname, ix.indisunique, ix.indisprimary, am.amname, ix.indpred, ix.indrelid
ORDER BY ns.nspname, tbl.relname, idx.relname
"#;

#[cfg(any(feature = "postgres-sync", feature = "tokio-postgres"))]
const POSTGRES_FOREIGN_KEYS_QUERY: &str = r#"
SELECT
    ns.nspname AS schema,
    tbl.relname AS table,
    con.conname AS name,
    array_agg(src.attname ORDER BY s.ord) AS columns,
    ns_to.nspname AS schema_to,
    tbl_to.relname AS table_to,
    array_agg(dst.attname ORDER BY s.ord) AS columns_to,
    con.confupdtype::text AS on_update,
    con.confdeltype::text AS on_delete
FROM pg_constraint con
JOIN pg_class tbl ON tbl.oid = con.conrelid
JOIN pg_namespace ns ON ns.oid = tbl.relnamespace
JOIN pg_class tbl_to ON tbl_to.oid = con.confrelid
JOIN pg_namespace ns_to ON ns_to.oid = tbl_to.relnamespace
JOIN unnest(con.conkey) WITH ORDINALITY AS s(attnum, ord) ON TRUE
JOIN pg_attribute src ON src.attrelid = tbl.oid AND src.attnum = s.attnum
JOIN unnest(con.confkey) WITH ORDINALITY AS r(attnum, ord) ON r.ord = s.ord
JOIN pg_attribute dst ON dst.attrelid = tbl_to.oid AND dst.attnum = r.attnum
WHERE con.contype = 'f'
  AND ns.nspname NOT LIKE 'pg_%'
  AND ns.nspname <> 'information_schema'
GROUP BY ns.nspname, tbl.relname, con.conname, ns_to.nspname, tbl_to.relname, con.confupdtype, con.confdeltype
ORDER BY ns.nspname, tbl.relname, con.conname
"#;

#[cfg(any(feature = "postgres-sync", feature = "tokio-postgres"))]
const POSTGRES_PRIMARY_KEYS_QUERY: &str = r#"
SELECT
    ns.nspname AS schema,
    tbl.relname AS table,
    con.conname AS name,
    array_agg(att.attname ORDER BY s.ord) AS columns
FROM pg_constraint con
JOIN pg_class tbl ON tbl.oid = con.conrelid
JOIN pg_namespace ns ON ns.oid = tbl.relnamespace
JOIN unnest(con.conkey) WITH ORDINALITY AS s(attnum, ord) ON TRUE
JOIN pg_attribute att ON att.attrelid = tbl.oid AND att.attnum = s.attnum
WHERE con.contype = 'p'
  AND ns.nspname NOT LIKE 'pg_%'
  AND ns.nspname <> 'information_schema'
GROUP BY ns.nspname, tbl.relname, con.conname
ORDER BY ns.nspname, tbl.relname, con.conname
"#;

#[cfg(any(feature = "postgres-sync", feature = "tokio-postgres"))]
const POSTGRES_UNIQUES_QUERY: &str = r#"
SELECT
    ns.nspname AS schema,
    tbl.relname AS table,
    con.conname AS name,
    array_agg(att.attname ORDER BY s.ord) AS columns,
    COALESCE(con.connullsnotdistinct, FALSE) AS nulls_not_distinct
FROM pg_constraint con
JOIN pg_class tbl ON tbl.oid = con.conrelid
JOIN pg_namespace ns ON ns.oid = tbl.relnamespace
JOIN unnest(con.conkey) WITH ORDINALITY AS s(attnum, ord) ON TRUE
JOIN pg_attribute att ON att.attrelid = tbl.oid AND att.attnum = s.attnum
WHERE con.contype = 'u'
  AND ns.nspname NOT LIKE 'pg_%'
  AND ns.nspname <> 'information_schema'
GROUP BY ns.nspname, tbl.relname, con.conname, con.connullsnotdistinct
ORDER BY ns.nspname, tbl.relname, con.conname
"#;

#[cfg(any(feature = "postgres-sync", feature = "tokio-postgres"))]
const POSTGRES_CHECKS_QUERY: &str = r#"
SELECT
    ns.nspname AS schema,
    tbl.relname AS table,
    con.conname AS name,
    pg_get_expr(con.conbin, con.conrelid) AS expression
FROM pg_constraint con
JOIN pg_class tbl ON tbl.oid = con.conrelid
JOIN pg_namespace ns ON ns.oid = tbl.relnamespace
WHERE con.contype = 'c'
  AND ns.nspname NOT LIKE 'pg_%'
  AND ns.nspname <> 'information_schema'
ORDER BY ns.nspname, tbl.relname, con.conname
"#;

#[cfg(any(feature = "postgres-sync", feature = "tokio-postgres"))]
const POSTGRES_ROLES_QUERY: &str = r#"
SELECT
    rolname AS name,
    rolcreatedb AS create_db,
    rolcreaterole AS create_role,
    rolinherit AS inherit
FROM pg_roles
ORDER BY rolname
"#;

#[cfg(any(feature = "postgres-sync", feature = "tokio-postgres"))]
const POSTGRES_POLICIES_QUERY: &str = r#"
SELECT
    schemaname AS schema,
    tablename AS table,
    policyname AS name,
    CASE WHEN permissive THEN 'PERMISSIVE' ELSE 'RESTRICTIVE' END AS as_clause,
    upper(cmd) AS for_clause,
    roles AS to,
    qual AS using,
    with_check AS with_check
FROM pg_policies
WHERE schemaname NOT LIKE 'pg_%'
  AND schemaname <> 'information_schema'
ORDER BY schemaname, tablename, policyname
"#;

#[cfg(any(feature = "postgres-sync", feature = "tokio-postgres"))]
fn pg_action_code_to_string(code: String) -> String {
    match code.as_str() {
        "a" => "NO ACTION",
        "r" => "RESTRICT",
        "c" => "CASCADE",
        "n" => "SET NULL",
        "d" => "SET DEFAULT",
        _ => "NO ACTION",
    }
    .to_string()
}

#[cfg(any(feature = "postgres-sync", feature = "tokio-postgres"))]
fn parse_postgres_index_columns(
    cols: Vec<String>,
) -> Vec<drizzle_migrations::postgres::introspect::RawIndexColumnInfo> {
    use drizzle_migrations::postgres::introspect::RawIndexColumnInfo;
    cols.into_iter()
        .map(|c| {
            let trimmed = c.trim().to_string();
            let upper = trimmed.to_uppercase();

            let asc = !upper.contains(" DESC");
            let nulls_first = upper.contains(" NULLS FIRST");

            // Strip sort/nulls directives for opclass parsing / expression detection.
            let mut core = trimmed.clone();
            for token in [" ASC", " DESC", " NULLS FIRST", " NULLS LAST"] {
                if let Some(pos) = core.to_uppercase().find(token) {
                    core.truncate(pos);
                    break;
                }
            }
            let core = core.trim().to_string();

            // Heuristic: treat as expression if it contains parentheses or spaces.
            let is_expression =
                core.contains('(') || core.contains(')') || core.contains(' ') || core.contains("::");

            // Heuristic opclass parsing: split whitespace and take second token if it looks like opclass.
            let mut opclass: Option<String> = None;
            let mut name = core.clone();
            let parts: Vec<&str> = core.split_whitespace().collect();
            if parts.len() >= 2 {
                let second = parts[1];
                if !matches!(second.to_uppercase().as_str(), "ASC" | "DESC" | "NULLS") {
                    opclass = Some(second.to_string());
                    name = parts[0].to_string();
                }
            }

            RawIndexColumnInfo {
                name,
                is_expression,
                asc,
                nulls_first,
                opclass,
            }
        })
        .collect()
}

#[cfg(any(feature = "postgres-sync", feature = "tokio-postgres"))]
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

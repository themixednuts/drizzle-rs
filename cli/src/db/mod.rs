//! Database connection and migration execution for CLI commands
//!
//! This module provides database connectivity for running migrations and other
//! database operations from the CLI.

use std::path::Path;

#[cfg(any(feature = "postgres-sync", feature = "tokio-postgres"))]
use crate::config::PostgresCreds;
use crate::config::{Credentials, Dialect, Extension, IntrospectCasing};
use crate::error::CliError;
use crate::output;
#[cfg(any(
    test,
    feature = "rusqlite",
    feature = "libsql",
    feature = "turso",
    feature = "postgres-sync",
    feature = "tokio-postgres",
    feature = "d1-http",
))]
use drizzle_migrations::Migrations;
use drizzle_migrations::schema::Snapshot;

#[cfg(feature = "d1-http")]
mod d1_http;

/// Result of a migration run
#[derive(Debug)]
pub struct MigrationResult {
    /// Number of migrations applied
    pub applied_count: usize,
    /// Tags of applied migrations
    pub applied_migrations: Vec<String>,
}

/// Planned migration execution details.
#[derive(Debug, Clone)]
pub struct MigrationPlan {
    /// Number of already-applied migrations found in the database metadata table.
    pub applied_count: usize,
    /// Number of pending migrations found locally.
    pub pending_count: usize,
    /// Pending migration tags in execution order.
    pub pending_migrations: Vec<String>,
    /// Total number of non-empty SQL statements in pending migrations.
    pub pending_statements: usize,
}

#[cfg(any(
    test,
    feature = "rusqlite",
    feature = "libsql",
    feature = "turso",
    feature = "postgres-sync",
    feature = "tokio-postgres",
    feature = "d1-http",
))]
#[derive(Debug, Clone)]
pub(crate) struct AppliedMigrationRecord {
    pub(crate) hash: String,
    pub(crate) name: String,
}

/// Planned SQL changes for `drizzle push`
#[derive(Debug, Clone)]
pub struct PushPlan {
    pub sql_statements: Vec<String>,
    pub warnings: Vec<String>,
    pub destructive: bool,
}

/// Optional filters for introspection and push planning.
#[derive(Debug, Clone, Default)]
pub struct SnapshotFilters {
    pub tables: Option<Vec<String>>,
    pub schemas: Option<Vec<String>>,
    pub extensions: Option<Vec<Extension>>,
}

impl SnapshotFilters {
    const fn is_empty(&self) -> bool {
        self.tables.is_none() && self.schemas.is_none() && self.extensions.is_none()
    }
}

/// Plan a push by introspecting the live database and diffing against the desired snapshot.
///
/// # Errors
///
/// Returns [`CliError`] if introspecting the live database fails, if applying
/// the given snapshot filters fails, or if generating the diff SQL fails.
pub fn plan_push(
    credentials: &Credentials,
    dialect: Dialect,
    desired: &Snapshot,
    breakpoints: bool,
    filters: &SnapshotFilters,
) -> Result<PushPlan, CliError> {
    let mut current = introspect_database(credentials, dialect)?.snapshot;
    apply_snapshot_filters(&mut current, dialect, filters)?;
    let (sql_statements, warnings) = generate_push_sql(&current, desired, breakpoints)?;
    let destructive = sql_statements.iter().any(|s| is_destructive_statement(s));

    Ok(PushPlan {
        sql_statements,
        warnings,
        destructive,
    })
}

/// Apply a previously planned push.
///
/// # Errors
///
/// Returns [`CliError`] if the confirmation prompt for a destructive plan
/// fails, or if executing the planned SQL statements against the database
/// fails.
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
///
/// # Errors
///
/// Returns [`CliError`] if no compiled driver matches the credentials, if
/// connecting to the database fails, or if reading the migration tracking
/// table or on-disk migration files fails.
#[allow(unused_variables)] // params consumed inside feature-gated block
pub fn plan_migrations(
    credentials: &Credentials,
    dialect: Dialect,
    migrations_dir: &Path,
    migrations_table: &str,
    migrations_schema: &str,
) -> Result<MigrationPlan, CliError> {
    #[cfg(any(
        feature = "rusqlite",
        feature = "libsql",
        feature = "turso",
        feature = "postgres-sync",
        feature = "tokio-postgres",
        feature = "d1-http",
    ))]
    let set = load_migration_set(dialect, migrations_dir, migrations_table, migrations_schema)?;

    match credentials {
        #[cfg(feature = "rusqlite")]
        Credentials::Sqlite { path } => inspect_sqlite_migrations(&set, path),

        #[cfg(not(feature = "rusqlite"))]
        Credentials::Sqlite { .. } => Err(CliError::MissingDriver {
            dialect: "SQLite",
            feature: "rusqlite",
        }),

        #[cfg(any(feature = "libsql", feature = "turso"))]
        Credentials::Turso { url, auth_token } => {
            if is_local_libsql(url) {
                #[cfg(feature = "libsql")]
                {
                    inspect_libsql_local_migrations(&set, url)
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
                    inspect_turso_migrations(&set, url, auth_token.as_deref())
                }
                #[cfg(not(feature = "turso"))]
                {
                    let _ = auth_token;
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

        Credentials::Postgres(creds) => {
            let _ = creds;
            core::cfg_select! {
                feature = "postgres-sync" => inspect_postgres_sync_migrations(&set, creds),
                feature = "tokio-postgres" => inspect_postgres_async_migrations(&set, creds),
                _ => Err(CliError::MissingDriver {
                    dialect: "PostgreSQL",
                    feature: "postgres-sync or tokio-postgres",
                }),
            }
        }

        #[cfg(feature = "d1-http")]
        Credentials::D1 {
            account_id,
            database_id,
            token,
        } => d1_http::inspect_migrations(&set, account_id, database_id, token),

        #[cfg(not(feature = "d1-http"))]
        Credentials::D1 { .. } => Err(CliError::MissingDriver {
            dialect: "Cloudflare D1 (HTTP)",
            feature: "d1-http",
        }),

        Credentials::AwsDataApi { .. } => Err(CliError::UnsupportedForDriver {
            operation: "Migration planning against AWS Data API",
            driver: "aws-data-api",
            hint: "AWS RDS Data API schema ops are not yet wired into this CLI. For now, \
                   run the generated SQL with `aws rds-data execute-statement --sql=...` \
                   (or use tokio-postgres with a temporary direct connection).",
        }),
    }
}

/// Verify migrations by re-running the planning logic without applying
/// anything, surfacing any inconsistencies between the on-disk migration
/// files and the tracking table.
///
/// # Errors
///
/// Returns the same errors as [`plan_migrations`].
pub fn verify_migrations(
    credentials: &Credentials,
    dialect: Dialect,
    migrations_dir: &Path,
    migrations_table: &str,
    migrations_schema: &str,
) -> Result<MigrationPlan, CliError> {
    plan_migrations(
        credentials,
        dialect,
        migrations_dir,
        migrations_table,
        migrations_schema,
    )
}

/// Apply any pending migrations against the database referenced by
/// `credentials`.
///
/// # Errors
///
/// Returns [`CliError`] if no compiled driver matches, if connecting or
/// starting a transaction fails, if executing a migration's SQL fails, or if
/// writing to the tracking table fails.
#[allow(unused_variables)] // params consumed inside feature-gated block
pub fn run_migrations(
    credentials: &Credentials,
    dialect: Dialect,
    migrations_dir: &Path,
    migrations_table: &str,
    migrations_schema: &str,
) -> Result<MigrationResult, CliError> {
    #[cfg(any(
        feature = "rusqlite",
        feature = "libsql",
        feature = "turso",
        feature = "postgres-sync",
        feature = "tokio-postgres",
        feature = "d1-http",
    ))]
    let set = load_migration_set(dialect, migrations_dir, migrations_table, migrations_schema)?;

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
                    run_turso_migrations(&set, url, auth_token.as_deref())
                }
                #[cfg(not(feature = "turso"))]
                {
                    let _ = auth_token;
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
        Credentials::Postgres(creds) => {
            let _ = creds;
            core::cfg_select! {
                feature = "postgres-sync" => run_postgres_sync_migrations(&set, creds),
                feature = "tokio-postgres" => run_postgres_async_migrations(&set, creds),
                _ => Err(CliError::MissingDriver {
                    dialect: "PostgreSQL",
                    feature: "postgres-sync or tokio-postgres",
                }),
            }
        }

        #[cfg(feature = "d1-http")]
        Credentials::D1 {
            account_id,
            database_id,
            token,
        } => d1_http::run_migrations(&set, account_id, database_id, token),

        #[cfg(not(feature = "d1-http"))]
        Credentials::D1 { .. } => Err(CliError::MissingDriver {
            dialect: "Cloudflare D1 (HTTP)",
            feature: "d1-http",
        }),

        Credentials::AwsDataApi { .. } => Err(CliError::UnsupportedForDriver {
            operation: "Running migrations against AWS Data API",
            driver: "aws-data-api",
            hint: "AWS RDS Data API migrations are not yet wired into this CLI. Run the \
                   generated SQL via `aws rds-data execute-statement` or connect directly \
                   with tokio-postgres.",
        }),
    }
}

#[cfg(any(
    feature = "rusqlite",
    feature = "libsql",
    feature = "turso",
    feature = "postgres-sync",
    feature = "tokio-postgres",
    feature = "d1-http",
))]
fn load_migration_set(
    dialect: Dialect,
    migrations_dir: &Path,
    migrations_table: &str,
    migrations_schema: &str,
) -> Result<Migrations, CliError> {
    let tracking = migration_tracking(dialect, migrations_table, migrations_schema);

    // Load migrations from filesystem
    let migrations = drizzle_migrations::MigrationDir::new(migrations_dir)
        .discover()
        .map_err(|e| CliError::Other(format!("Failed to load migrations: {e}")))?;
    Ok(Migrations::with_tracking(
        migrations,
        dialect.to_base(),
        tracking,
    ))
}

#[cfg(any(
    feature = "rusqlite",
    feature = "libsql",
    feature = "turso",
    feature = "postgres-sync",
    feature = "tokio-postgres",
    feature = "d1-http",
))]
fn migration_tracking(
    dialect: Dialect,
    migrations_table: &str,
    migrations_schema: &str,
) -> drizzle_types::MigrationTracking {
    let mut tracking = match dialect {
        Dialect::Postgresql => drizzle_types::MigrationTracking::POSTGRES,
        _ => drizzle_types::MigrationTracking::SQLITE,
    };

    if !migrations_table.trim().is_empty() {
        tracking = tracking.table(migrations_table.to_owned());
    }

    if dialect == Dialect::Postgresql && !migrations_schema.trim().is_empty() {
        tracking = tracking.schema(migrations_schema.to_owned());
    }

    tracking
}

#[cfg(any(
    test,
    feature = "rusqlite",
    feature = "libsql",
    feature = "turso",
    feature = "postgres-sync",
    feature = "tokio-postgres",
    feature = "d1-http",
))]
pub(crate) fn build_migration_plan(
    set: &Migrations,
    applied: &[AppliedMigrationRecord],
) -> Result<MigrationPlan, CliError> {
    verify_applied_migrations_consistency(set, applied)?;

    let applied_names = applied.iter().map(|m| m.name.clone()).collect::<Vec<_>>();
    let pending = set.pending(&applied_names).collect::<Vec<_>>();

    let pending_statements = pending
        .iter()
        .map(|m| {
            m.statements()
                .iter()
                .filter(|stmt| !stmt.trim().is_empty())
                .count()
        })
        .sum();

    Ok(MigrationPlan {
        applied_count: applied.len(),
        pending_count: pending.len(),
        pending_migrations: pending.iter().map(|m| m.tag().to_string()).collect(),
        pending_statements,
    })
}

#[cfg(any(
    test,
    feature = "rusqlite",
    feature = "libsql",
    feature = "turso",
    feature = "postgres-sync",
    feature = "tokio-postgres",
    feature = "d1-http",
))]
fn verify_applied_migrations_consistency(
    set: &Migrations,
    applied: &[AppliedMigrationRecord],
) -> Result<(), CliError> {
    use std::collections::{HashMap, HashSet};

    let mut local_by_name = HashMap::<&str, &str>::new();
    for migration in set.all() {
        if local_by_name
            .insert(migration.name(), migration.hash())
            .is_some()
        {
            return Err(CliError::MigrationError(format!(
                "Local migrations contain duplicate name: {}",
                migration.name()
            )));
        }
    }

    let mut seen_db_names = HashSet::<&str>::new();
    for applied_row in applied {
        if !seen_db_names.insert(applied_row.name.as_str()) {
            return Err(CliError::MigrationError(format!(
                "Database migration metadata contains duplicate name: {}",
                applied_row.name
            )));
        }

        let Some(local_hash) = local_by_name.get(applied_row.name.as_str()) else {
            return Err(CliError::MigrationError(format!(
                "Database contains applied migration not found locally (name: {})",
                applied_row.name
            )));
        };

        if *local_hash != applied_row.hash {
            return Err(CliError::MigrationError(format!(
                "Migration hash mismatch for {}: database={}, local={}",
                applied_row.name, applied_row.hash, local_hash
            )));
        }
    }

    Ok(())
}

#[cfg(any(
    feature = "rusqlite",
    feature = "libsql",
    feature = "turso",
    feature = "postgres-sync",
    feature = "tokio-postgres",
))]
fn escape_sql_literal(value: &str) -> String {
    value.replace('\'', "''")
}

#[cfg(feature = "rusqlite")]
fn ensure_sqlite_tracking_table(
    conn: &rusqlite::Connection,
    set: &Migrations,
) -> Result<(), CliError> {
    conn.execute(&set.create_table_sql(), [])
        .map_err(|e| CliError::MigrationError(format!("Failed to create migrations table: {e}")))?;

    let pragma_sql = format!(
        "SELECT name FROM pragma_table_info('{}')",
        escape_sql_literal(set.table_name())
    );
    let mut stmt = conn
        .prepare(&pragma_sql)
        .map_err(|e| CliError::MigrationError(e.to_string()))?;
    let columns = stmt
        .query_map([], |row| row.get::<_, String>(0))
        .map_err(|e| CliError::MigrationError(e.to_string()))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| CliError::MigrationError(e.to_string()))?;
    if columns.iter().any(|column| column == "name") {
        return Ok(());
    }

    let mut stmt = conn
        .prepare(&format!(
            "SELECT id, hash, created_at FROM {} ORDER BY id ASC",
            set.table_ident_sql()
        ))
        .map_err(|e| CliError::MigrationError(e.to_string()))?;
    let applied = stmt
        .query_map([], |row| {
            Ok(drizzle_migrations::AppliedMigrationMetadata {
                id: row.get::<_, Option<i64>>(0)?,
                hash: row.get::<_, String>(1)?,
                created_at: row.get::<_, i64>(2)?,
            })
        })
        .map_err(|e| CliError::MigrationError(e.to_string()))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| CliError::MigrationError(e.to_string()))?;

    let matched = drizzle_migrations::match_applied_migration_metadata(set.all(), &applied)
        .map_err(|e| CliError::MigrationError(e.to_string()))?;

    conn.execute("BEGIN", [])
        .map_err(|e| CliError::MigrationError(e.to_string()))?;
    let result = (|| -> Result<(), CliError> {
        conn.execute(
            &format!(
                "ALTER TABLE {} ADD COLUMN \"name\" text",
                set.table_ident_sql()
            ),
            [],
        )
        .map_err(|e| CliError::MigrationError(e.to_string()))?;
        conn.execute(
            &format!(
                "ALTER TABLE {} ADD COLUMN \"applied_at\" TEXT",
                set.table_ident_sql()
            ),
            [],
        )
        .map_err(|e| CliError::MigrationError(e.to_string()))?;

        for row in matched {
            let where_clause = if let Some(id) = row.id {
                format!("\"id\" = {id}")
            } else {
                format!(
                    "\"created_at\" = {} AND \"hash\" = '{}'",
                    row.created_at,
                    escape_sql_literal(&row.hash)
                )
            };
            conn.execute(
                &format!(
                    "UPDATE {} SET \"name\" = '{}', \"applied_at\" = NULL WHERE {}",
                    set.table_ident_sql(),
                    escape_sql_literal(&row.name),
                    where_clause
                ),
                [],
            )
            .map_err(|e| CliError::MigrationError(e.to_string()))?;
        }

        Ok(())
    })();

    match result {
        Ok(()) => {
            conn.execute("COMMIT", [])
                .map_err(|e| CliError::MigrationError(e.to_string()))?;
            Ok(())
        }
        Err(err) => {
            let _ = conn.execute("ROLLBACK", []);
            Err(err)
        }
    }
}

#[cfg(feature = "libsql")]
async fn ensure_sqlite_tracking_table_libsql(
    conn: &libsql::Connection,
    set: &Migrations,
) -> Result<(), CliError> {
    conn.execute(&set.create_table_sql(), ())
        .await
        .map_err(|e| CliError::MigrationError(format!("Failed to create migrations table: {e}")))?;

    let pragma_sql = format!(
        "SELECT name FROM pragma_table_info('{}')",
        escape_sql_literal(set.table_name())
    );
    let mut rows = conn
        .query(&pragma_sql, ())
        .await
        .map_err(|e| CliError::MigrationError(e.to_string()))?;
    let mut has_name = false;
    while let Ok(Some(row)) = rows.next().await {
        if let Ok(name) = row.get::<String>(0)
            && name == "name"
        {
            has_name = true;
            break;
        }
    }
    if has_name {
        return Ok(());
    }

    let mut rows = conn
        .query(
            &format!(
                "SELECT id, hash, created_at FROM {} ORDER BY id ASC",
                set.table_ident_sql()
            ),
            (),
        )
        .await
        .map_err(|e| CliError::MigrationError(e.to_string()))?;
    let mut applied = Vec::new();
    while let Ok(Some(row)) = rows.next().await {
        let hash = row
            .get::<String>(1)
            .map_err(|e| CliError::MigrationError(e.to_string()))?;
        let created_at = row
            .get::<i64>(2)
            .map_err(|e| CliError::MigrationError(e.to_string()))?;
        applied.push(drizzle_migrations::AppliedMigrationMetadata {
            id: row.get::<Option<i64>>(0).ok().flatten(),
            hash,
            created_at,
        });
    }

    let matched = drizzle_migrations::match_applied_migration_metadata(set.all(), &applied)
        .map_err(|e| CliError::MigrationError(e.to_string()))?;

    let tx = conn
        .transaction()
        .await
        .map_err(|e| CliError::MigrationError(e.to_string()))?;
    tx.execute(
        &format!(
            "ALTER TABLE {} ADD COLUMN \"name\" text",
            set.table_ident_sql()
        ),
        (),
    )
    .await
    .map_err(|e| CliError::MigrationError(e.to_string()))?;
    tx.execute(
        &format!(
            "ALTER TABLE {} ADD COLUMN \"applied_at\" TEXT",
            set.table_ident_sql()
        ),
        (),
    )
    .await
    .map_err(|e| CliError::MigrationError(e.to_string()))?;

    for row in matched {
        let where_clause = if let Some(id) = row.id {
            format!("\"id\" = {id}")
        } else {
            format!(
                "\"created_at\" = {} AND \"hash\" = '{}'",
                row.created_at,
                escape_sql_literal(&row.hash)
            )
        };
        tx.execute(
            &format!(
                "UPDATE {} SET \"name\" = '{}', \"applied_at\" = NULL WHERE {}",
                set.table_ident_sql(),
                escape_sql_literal(&row.name),
                where_clause
            ),
            (),
        )
        .await
        .map_err(|e| CliError::MigrationError(e.to_string()))?;
    }

    tx.commit()
        .await
        .map_err(|e| CliError::MigrationError(e.to_string()))?;
    Ok(())
}

#[cfg(feature = "postgres-sync")]
fn ensure_postgres_tracking_table_sync(
    client: &mut postgres::Client,
    set: &Migrations,
) -> Result<(), CliError> {
    client
        .execute(&set.create_table_sql(), &[])
        .map_err(|e| CliError::MigrationError(format!("Failed to create migrations table: {e}")))?;

    let schema = set.schema_name().unwrap_or("public");
    let rows = client
        .query(
            "SELECT column_name FROM information_schema.columns WHERE table_schema = $1 AND table_name = $2",
            &[&schema, &set.table_name()],
        )
        .map_err(|e| CliError::MigrationError(e.to_string()))?;
    if rows
        .iter()
        .filter_map(|row| row.try_get::<_, String>(0).ok())
        .any(|column| column == "name")
    {
        return Ok(());
    }

    let rows = client
        .query(
            &format!(
                "SELECT id, hash, created_at FROM {} ORDER BY id ASC",
                set.table_ident_sql()
            ),
            &[],
        )
        .map_err(|e| CliError::MigrationError(e.to_string()))?;
    let applied = rows
        .iter()
        .map(|row| {
            Ok(drizzle_migrations::AppliedMigrationMetadata {
                id: row.try_get::<_, Option<i64>>(0).ok().flatten(),
                hash: row.try_get::<_, String>(1)?,
                created_at: row.try_get::<_, i64>(2)?,
            })
        })
        .collect::<Result<Vec<_>, postgres::Error>>()
        .map_err(|e| CliError::MigrationError(e.to_string()))?;

    let matched = drizzle_migrations::match_applied_migration_metadata(set.all(), &applied)
        .map_err(|e| CliError::MigrationError(e.to_string()))?;

    client
        .execute(
            &format!(
                "ALTER TABLE {} ADD COLUMN \"name\" TEXT",
                set.table_ident_sql()
            ),
            &[],
        )
        .map_err(|e| CliError::MigrationError(e.to_string()))?;
    client
        .execute(
            &format!(
                "ALTER TABLE {} ADD COLUMN \"applied_at\" TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP",
                set.table_ident_sql()
            ),
            &[],
        )
        .map_err(|e| CliError::MigrationError(e.to_string()))?;

    for row in matched {
        let where_clause = if let Some(id) = row.id {
            format!("\"id\" = {id}")
        } else {
            format!(
                "\"created_at\" = {} AND \"hash\" = '{}'",
                row.created_at,
                escape_sql_literal(&row.hash)
            )
        };
        client
            .execute(
                &format!(
                    "UPDATE {} SET \"name\" = '{}', \"applied_at\" = NULL WHERE {}",
                    set.table_ident_sql(),
                    escape_sql_literal(&row.name),
                    where_clause
                ),
                &[],
            )
            .map_err(|e| CliError::MigrationError(e.to_string()))?;
    }

    Ok(())
}

#[cfg(feature = "tokio-postgres")]
async fn ensure_postgres_tracking_table_async(
    client: &tokio_postgres::Client,
    set: &Migrations,
) -> Result<(), CliError> {
    client
        .execute(&set.create_table_sql(), &[])
        .await
        .map_err(|e| CliError::MigrationError(format!("Failed to create migrations table: {e}")))?;

    let schema = set.schema_name().unwrap_or("public");
    let rows = client
        .query(
            "SELECT column_name FROM information_schema.columns WHERE table_schema = $1 AND table_name = $2",
            &[&schema, &set.table_name()],
        )
        .await
        .map_err(|e| CliError::MigrationError(e.to_string()))?;
    if rows
        .iter()
        .filter_map(|row| row.try_get::<_, String>(0).ok())
        .any(|column| column == "name")
    {
        return Ok(());
    }

    let rows = client
        .query(
            &format!(
                "SELECT id, hash, created_at FROM {} ORDER BY id ASC",
                set.table_ident_sql()
            ),
            &[],
        )
        .await
        .map_err(|e| CliError::MigrationError(e.to_string()))?;
    let applied = rows
        .iter()
        .map(|row| {
            Ok(drizzle_migrations::AppliedMigrationMetadata {
                id: row.try_get::<_, Option<i64>>(0).ok().flatten(),
                hash: row.try_get::<_, String>(1)?,
                created_at: row.try_get::<_, i64>(2)?,
            })
        })
        .collect::<Result<Vec<_>, tokio_postgres::Error>>()
        .map_err(|e| CliError::MigrationError(e.to_string()))?;

    let matched = drizzle_migrations::match_applied_migration_metadata(set.all(), &applied)
        .map_err(|e| CliError::MigrationError(e.to_string()))?;

    client
        .execute(
            &format!(
                "ALTER TABLE {} ADD COLUMN \"name\" TEXT",
                set.table_ident_sql()
            ),
            &[],
        )
        .await
        .map_err(|e| CliError::MigrationError(e.to_string()))?;
    client
        .execute(
            &format!(
                "ALTER TABLE {} ADD COLUMN \"applied_at\" TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP",
                set.table_ident_sql()
            ),
            &[],
        )
        .await
        .map_err(|e| CliError::MigrationError(e.to_string()))?;

    for row in matched {
        let where_clause = if let Some(id) = row.id {
            format!("\"id\" = {id}")
        } else {
            format!(
                "\"created_at\" = {} AND \"hash\" = '{}'",
                row.created_at,
                escape_sql_literal(&row.hash)
            )
        };
        client
            .execute(
                &format!(
                    "UPDATE {} SET \"name\" = '{}', \"applied_at\" = NULL WHERE {}",
                    set.table_ident_sql(),
                    escape_sql_literal(&row.name),
                    where_clause
                ),
                &[],
            )
            .await
            .map_err(|e| CliError::MigrationError(e.to_string()))?;
    }

    Ok(())
}

fn is_destructive_statement(sql: &str) -> bool {
    let s = sql.trim().to_uppercase();
    s.contains("DROP TABLE")
        || s.contains("DROP COLUMN")
        || s.contains("DROP INDEX")
        || s.contains("DROP VIEW")
        || s.contains("DROP MATERIALIZED VIEW")
        || s.contains("DROP TYPE")
        || s.contains("DROP SCHEMA")
        || s.contains("DROP SEQUENCE")
        || s.contains("DROP ROLE")
        || s.contains("DROP POLICY")
        || s.contains("TRUNCATE")
        || (s.contains("ALTER TABLE") && s.contains(" DROP "))
}

#[cfg(any(feature = "postgres-sync", feature = "tokio-postgres"))]
fn is_postgres_concurrent_index_statement(sql: &str) -> bool {
    let s = sql.trim().to_ascii_uppercase();
    (s.starts_with("CREATE") || s.starts_with("DROP")) && s.contains("INDEX CONCURRENTLY")
}

#[cfg(any(feature = "postgres-sync", feature = "tokio-postgres"))]
fn has_postgres_concurrent_index(statements: &[String]) -> bool {
    statements
        .iter()
        .any(|stmt| is_postgres_concurrent_index_statement(stmt))
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
        output::warning("Potentially destructive changes detected (DROP/TRUNCATE/etc).")
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
                    execute_turso_statements(url, auth_token.as_deref(), statements)
                }
                #[cfg(not(feature = "turso"))]
                {
                    let _ = auth_token;
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

        Credentials::Postgres(creds) => {
            let _ = (creds, &statements);
            core::cfg_select! {
                feature = "postgres-sync" => execute_postgres_sync_statements(creds, statements),
                feature = "tokio-postgres" => execute_postgres_async_statements(creds, statements),
                _ => Err(CliError::MissingDriver {
                    dialect: "PostgreSQL",
                    feature: "postgres-sync or tokio-postgres",
                }),
            }
        }

        #[cfg(feature = "d1-http")]
        Credentials::D1 {
            account_id,
            database_id,
            token,
        } => d1_http::execute_statements(account_id, database_id, token, statements),

        #[cfg(not(feature = "d1-http"))]
        Credentials::D1 { .. } => Err(CliError::MissingDriver {
            dialect: "Cloudflare D1 (HTTP)",
            feature: "d1-http",
        }),

        Credentials::AwsDataApi { .. } => Err(CliError::UnsupportedForDriver {
            operation: "Direct SQL execution against AWS Data API",
            driver: "aws-data-api",
            hint: "Use `aws rds-data execute-statement --sql=\"...\"` with the matching \
                   --resource-arn / --secret-arn / --database, or connect directly via \
                   tokio-postgres.",
        }),
    }
}

/// Check if a Turso URL is a local libsql database
#[allow(dead_code)]
fn is_local_libsql(url: &str) -> bool {
    url.starts_with("file:")
        || url.starts_with("./")
        || url.starts_with('/')
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

        let mut uniq =
            UniqueConstraint::from_strings(idx.table.clone(), constraint_name, col_names);
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
        CliError::ConnectionError(format!("Failed to open SQLite database '{path}': {e}"))
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
                "Statement failed: {e}\n{s}"
            )));
        }
    }

    conn.execute("COMMIT", [])
        .map_err(|e| CliError::MigrationError(e.to_string()))?;

    Ok(())
}

#[cfg(feature = "rusqlite")]
fn run_sqlite_migrations(set: &Migrations, path: &str) -> Result<MigrationResult, CliError> {
    let conn = rusqlite::Connection::open(path).map_err(|e| {
        CliError::ConnectionError(format!("Failed to open SQLite database '{path}': {e}"))
    })?;

    ensure_sqlite_tracking_table(&conn, set)?;

    // Query applied migration names
    let applied_names = query_applied_names_sqlite(&conn, set)?;

    // Get pending migrations
    let pending: Vec<_> = set.pending(&applied_names).collect();
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
        if let Err(e) = conn.execute(&set.record_migration_sql(migration), []) {
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
fn inspect_sqlite_migrations(set: &Migrations, path: &str) -> Result<MigrationPlan, CliError> {
    let conn = rusqlite::Connection::open(path).map_err(|e| {
        CliError::ConnectionError(format!("Failed to open SQLite database '{path}': {e}"))
    })?;

    ensure_sqlite_tracking_table(&conn, set)?;
    let applied = query_applied_records_sqlite(&conn, set)?;
    build_migration_plan(set, &applied)
}

#[cfg(feature = "rusqlite")]
fn query_applied_records_sqlite(
    conn: &rusqlite::Connection,
    set: &Migrations,
) -> Result<Vec<AppliedMigrationRecord>, CliError> {
    let sql = format!(
        r#"SELECT hash, "name" FROM {} WHERE "name" IS NOT NULL ORDER BY id;"#,
        set.table_ident_sql()
    );
    let Ok(mut stmt) = conn.prepare(&sql) else {
        return Ok(vec![]); // Table might not exist yet
    };

    let mut applied = Vec::new();
    let rows = stmt
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(|e| CliError::MigrationError(e.to_string()))?;

    for row in rows {
        let (hash, name) = row.map_err(|e| CliError::MigrationError(e.to_string()))?;
        applied.push(AppliedMigrationRecord { hash, name });
    }

    Ok(applied)
}

#[cfg(feature = "rusqlite")]
fn query_applied_names_sqlite(
    conn: &rusqlite::Connection,
    set: &Migrations,
) -> Result<Vec<String>, CliError> {
    let Ok(mut stmt) = conn.prepare(&set.applied_names_sql()) else {
        return Ok(vec![]); // Table might not exist yet
    };

    let names = stmt
        .query_map([], |row| row.get::<_, String>(0))
        .map_err(|e| CliError::MigrationError(e.to_string()))?
        .filter_map(Result::ok)
        .collect();

    Ok(names)
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
    let mut client = postgres::Client::connect(&url, postgres::NoTls)
        .map_err(|e| CliError::ConnectionError(format!("Failed to connect to PostgreSQL: {e}")))?;

    if has_postgres_concurrent_index(statements) {
        // CREATE/DROP INDEX CONCURRENTLY cannot run inside a transaction block.
        for stmt in statements {
            let s = stmt.trim();
            if s.is_empty() {
                continue;
            }
            client
                .execute(s, &[])
                .map_err(|e| CliError::MigrationError(format!("Statement failed: {e}\n{s}")))?;
        }
    } else {
        let mut tx = client
            .transaction()
            .map_err(|e| CliError::MigrationError(e.to_string()))?;

        for stmt in statements {
            let s = stmt.trim();
            if s.is_empty() {
                continue;
            }
            tx.execute(s, &[])
                .map_err(|e| CliError::MigrationError(format!("Statement failed: {e}\n{s}")))?;
        }

        tx.commit()
            .map_err(|e| CliError::MigrationError(e.to_string()))?;
    }

    Ok(())
}

#[cfg(feature = "postgres-sync")]
fn run_postgres_sync_migrations(
    set: &Migrations,
    creds: &PostgresCreds,
) -> Result<MigrationResult, CliError> {
    let url = creds.connection_url();
    let mut client = postgres::Client::connect(&url, postgres::NoTls)
        .map_err(|e| CliError::ConnectionError(format!("Failed to connect to PostgreSQL: {e}")))?;

    // Create schema if needed
    if let Some(schema_sql) = set.create_schema_sql() {
        client
            .execute(&schema_sql, &[])
            .map_err(|e| CliError::MigrationError(e.to_string()))?;
    }

    ensure_postgres_tracking_table_sync(&mut client, set)?;

    // Query applied migration names
    let rows = client
        .query(&set.applied_names_sql(), &[])
        .unwrap_or_default();
    let applied_names: Vec<String> = rows.iter().filter_map(|r| r.try_get(0).ok()).collect();

    // Get pending migrations
    let pending: Vec<_> = set.pending(&applied_names).collect();
    if pending.is_empty() {
        return Ok(MigrationResult {
            applied_count: 0,
            applied_migrations: vec![],
        });
    }

    let has_concurrent = pending
        .iter()
        .any(|m| has_postgres_concurrent_index(m.statements()));

    if has_concurrent {
        // CREATE/DROP INDEX CONCURRENTLY cannot run inside a transaction block.
        let mut applied = Vec::new();
        for migration in &pending {
            for stmt in migration.statements() {
                if !stmt.trim().is_empty() {
                    client.execute(stmt, &[]).map_err(|e| {
                        CliError::MigrationError(format!(
                            "Migration '{}' failed: {}",
                            migration.hash(),
                            e
                        ))
                    })?;
                }
            }
            client
                .execute(&set.record_migration_sql(migration), &[])
                .map_err(|e| CliError::MigrationError(e.to_string()))?;
            applied.push(migration.hash().to_string());
        }

        return Ok(MigrationResult {
            applied_count: applied.len(),
            applied_migrations: applied,
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
        tx.execute(&set.record_migration_sql(migration), &[])
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

#[cfg(feature = "postgres-sync")]
fn inspect_postgres_sync_migrations(
    set: &Migrations,
    creds: &PostgresCreds,
) -> Result<MigrationPlan, CliError> {
    let url = creds.connection_url();
    let mut client = postgres::Client::connect(&url, postgres::NoTls)
        .map_err(|e| CliError::ConnectionError(format!("Failed to connect to PostgreSQL: {e}")))?;

    if let Some(schema_sql) = set.create_schema_sql() {
        client
            .execute(&schema_sql, &[])
            .map_err(|e| CliError::MigrationError(e.to_string()))?;
    }

    ensure_postgres_tracking_table_sync(&mut client, set)?;
    let applied = query_applied_records_postgres_sync(&mut client, set);
    build_migration_plan(set, &applied)
}

#[cfg(feature = "postgres-sync")]
fn query_applied_records_postgres_sync(
    client: &mut postgres::Client,
    set: &Migrations,
) -> Vec<AppliedMigrationRecord> {
    let sql = format!(
        r#"SELECT hash, "name" FROM {} WHERE "name" IS NOT NULL ORDER BY id;"#,
        set.table_ident_sql()
    );
    let rows = client.query(&sql, &[]).unwrap_or_default();

    let mut applied = Vec::new();
    for row in rows {
        let hash = row.try_get::<_, Option<String>>(0).ok().flatten();
        let name = row.try_get::<_, Option<String>>(1).ok().flatten();
        if let (Some(hash), Some(name)) = (hash, name) {
            applied.push(AppliedMigrationRecord { hash, name });
        }
    }

    applied
}

// ============================================================================
// PostgreSQL (tokio-postgres - async)
// ============================================================================

#[cfg(all(feature = "tokio-postgres", not(feature = "postgres-sync")))]
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

#[cfg(all(feature = "tokio-postgres", not(feature = "postgres-sync")))]
async fn execute_postgres_async_inner(
    creds: &PostgresCreds,
    statements: &[String],
) -> Result<(), CliError> {
    let url = creds.connection_url();
    let (mut client, connection) = tokio_postgres::connect(&url, tokio_postgres::NoTls)
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

    if has_postgres_concurrent_index(statements) {
        // CREATE/DROP INDEX CONCURRENTLY cannot run inside a transaction block.
        for stmt in statements {
            let s = stmt.trim();
            if s.is_empty() {
                continue;
            }
            client
                .execute(s, &[])
                .await
                .map_err(|e| CliError::MigrationError(format!("Statement failed: {}\n{}", e, s)))?;
        }
    } else {
        let tx = client
            .transaction()
            .await
            .map_err(|e| CliError::MigrationError(e.to_string()))?;

        for stmt in statements {
            let s = stmt.trim();
            if s.is_empty() {
                continue;
            }
            tx.execute(s, &[])
                .await
                .map_err(|e| CliError::MigrationError(format!("Statement failed: {}\n{}", e, s)))?;
        }

        tx.commit()
            .await
            .map_err(|e| CliError::MigrationError(e.to_string()))?;
    }

    Ok(())
}

#[cfg(feature = "tokio-postgres")]
#[allow(dead_code)] // Used when postgres-sync is not enabled
fn run_postgres_async_migrations(
    set: &Migrations,
    creds: &PostgresCreds,
) -> Result<MigrationResult, CliError> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| CliError::Other(format!("Failed to create async runtime: {e}")))?;

    rt.block_on(run_postgres_async_inner(set, creds))
}

#[cfg(feature = "tokio-postgres")]
#[allow(dead_code)] // Used when postgres-sync is not enabled
fn inspect_postgres_async_migrations(
    set: &Migrations,
    creds: &PostgresCreds,
) -> Result<MigrationPlan, CliError> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| CliError::Other(format!("Failed to create async runtime: {e}")))?;

    rt.block_on(inspect_postgres_async_inner(set, creds))
}

#[cfg(feature = "tokio-postgres")]
#[allow(dead_code)]
async fn inspect_postgres_async_inner(
    set: &Migrations,
    creds: &PostgresCreds,
) -> Result<MigrationPlan, CliError> {
    let url = creds.connection_url();
    let (client, connection) = tokio_postgres::connect(&url, tokio_postgres::NoTls)
        .await
        .map_err(|e| CliError::ConnectionError(format!("Failed to connect to PostgreSQL: {e}")))?;

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

    ensure_postgres_tracking_table_async(&client, set).await?;
    let applied = query_applied_records_postgres_async(&client, set).await;
    build_migration_plan(set, &applied)
}

#[cfg(feature = "tokio-postgres")]
async fn query_applied_records_postgres_async(
    client: &tokio_postgres::Client,
    set: &Migrations,
) -> Vec<AppliedMigrationRecord> {
    let sql = format!(
        r#"SELECT hash, "name" FROM {} WHERE "name" IS NOT NULL ORDER BY id;"#,
        set.table_ident_sql()
    );
    let rows = client.query(&sql, &[]).await.unwrap_or_default();

    let mut applied = Vec::new();
    for row in rows {
        let hash = row.try_get::<_, Option<String>>(0).ok().flatten();
        let name = row.try_get::<_, Option<String>>(1).ok().flatten();
        if let (Some(hash), Some(name)) = (hash, name) {
            applied.push(AppliedMigrationRecord { hash, name });
        }
    }

    applied
}

#[cfg(feature = "tokio-postgres")]
#[allow(dead_code)]
async fn run_postgres_async_inner(
    set: &Migrations,
    creds: &PostgresCreds,
) -> Result<MigrationResult, CliError> {
    let url = creds.connection_url();
    let (mut client, connection) = tokio_postgres::connect(&url, tokio_postgres::NoTls)
        .await
        .map_err(|e| CliError::ConnectionError(format!("Failed to connect to PostgreSQL: {e}")))?;

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

    ensure_postgres_tracking_table_async(&client, set).await?;

    // Query applied migration names
    let rows = client
        .query(&set.applied_names_sql(), &[])
        .await
        .unwrap_or_default();
    let applied_names: Vec<String> = rows.iter().filter_map(|r| r.try_get(0).ok()).collect();

    // Get pending migrations
    let pending: Vec<_> = set.pending(&applied_names).collect();
    if pending.is_empty() {
        return Ok(MigrationResult {
            applied_count: 0,
            applied_migrations: vec![],
        });
    }

    let has_concurrent = pending
        .iter()
        .any(|m| has_postgres_concurrent_index(m.statements()));

    if has_concurrent {
        // CREATE/DROP INDEX CONCURRENTLY cannot run inside a transaction block.
        let mut applied = Vec::new();
        for migration in &pending {
            for stmt in migration.statements() {
                if !stmt.trim().is_empty() {
                    client.execute(stmt, &[]).await.map_err(|e| {
                        CliError::MigrationError(format!(
                            "Migration '{}' failed: {}",
                            migration.hash(),
                            e
                        ))
                    })?;
                }
            }
            client
                .execute(&set.record_migration_sql(migration), &[])
                .await
                .map_err(|e| CliError::MigrationError(e.to_string()))?;
            applied.push(migration.hash().to_string());
        }

        return Ok(MigrationResult {
            applied_count: applied.len(),
            applied_migrations: applied,
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
        tx.execute(&set.record_migration_sql(migration), &[])
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
        .map_err(|e| CliError::Other(format!("Failed to create async runtime: {e}")))?;

    rt.block_on(execute_libsql_local_inner(path, statements))
}

#[cfg(feature = "libsql")]
async fn execute_libsql_local_inner(path: &str, statements: &[String]) -> Result<(), CliError> {
    let db = libsql::Builder::new_local(path)
        .build()
        .await
        .map_err(|e| {
            CliError::ConnectionError(format!("Failed to open LibSQL database '{path}': {e}"))
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
                "Statement failed: {e}\n{s}"
            )));
        }
    }

    tx.commit()
        .await
        .map_err(|e| CliError::MigrationError(e.to_string()))?;

    Ok(())
}

#[cfg(feature = "libsql")]
fn run_libsql_local_migrations(set: &Migrations, path: &str) -> Result<MigrationResult, CliError> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| CliError::Other(format!("Failed to create async runtime: {e}")))?;

    rt.block_on(run_libsql_local_inner(set, path))
}

#[cfg(feature = "libsql")]
fn inspect_libsql_local_migrations(
    set: &Migrations,
    path: &str,
) -> Result<MigrationPlan, CliError> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| CliError::Other(format!("Failed to create async runtime: {e}")))?;

    rt.block_on(inspect_libsql_local_inner(set, path))
}

#[cfg(feature = "libsql")]
async fn inspect_libsql_local_inner(
    set: &Migrations,
    path: &str,
) -> Result<MigrationPlan, CliError> {
    let db = libsql::Builder::new_local(path)
        .build()
        .await
        .map_err(|e| {
            CliError::ConnectionError(format!("Failed to open LibSQL database '{path}': {e}"))
        })?;

    let conn = db
        .connect()
        .map_err(|e| CliError::ConnectionError(e.to_string()))?;

    ensure_sqlite_tracking_table_libsql(&conn, set).await?;
    let applied = query_applied_records_libsql(&conn, set).await?;
    build_migration_plan(set, &applied)
}

#[cfg(feature = "libsql")]
async fn run_libsql_local_inner(set: &Migrations, path: &str) -> Result<MigrationResult, CliError> {
    let db = libsql::Builder::new_local(path)
        .build()
        .await
        .map_err(|e| {
            CliError::ConnectionError(format!("Failed to open LibSQL database '{path}': {e}"))
        })?;

    let conn = db
        .connect()
        .map_err(|e| CliError::ConnectionError(e.to_string()))?;

    ensure_sqlite_tracking_table_libsql(&conn, set).await?;

    // Query applied migration names
    let applied_names = query_applied_names_libsql(&conn, set).await?;

    // Get pending migrations
    let pending: Vec<_> = set.pending(&applied_names).collect();
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
        if let Err(e) = tx.execute(&set.record_migration_sql(migration), ()).await {
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
async fn query_applied_names_libsql(
    conn: &libsql::Connection,
    set: &Migrations,
) -> Result<Vec<String>, CliError> {
    let Ok(mut rows) = conn.query(&set.applied_names_sql(), ()).await else {
        return Ok(vec![]); // Table might not exist yet
    };

    let mut names = Vec::new();
    while let Ok(Some(row)) = rows.next().await {
        if let Ok(name) = row.get::<String>(0) {
            names.push(name);
        }
    }

    Ok(names)
}

#[cfg(feature = "libsql")]
async fn query_applied_records_libsql(
    conn: &libsql::Connection,
    set: &Migrations,
) -> Result<Vec<AppliedMigrationRecord>, CliError> {
    let sql = format!(
        r#"SELECT hash, "name" FROM {} WHERE "name" IS NOT NULL ORDER BY id;"#,
        set.table_ident_sql()
    );
    let Ok(mut rows) = conn.query(&sql, ()).await else {
        return Ok(vec![]); // Table might not exist yet
    };

    let mut applied = Vec::new();
    while let Ok(Some(row)) = rows.next().await {
        if let (Ok(hash), Ok(name)) = (row.get::<String>(0), row.get::<String>(1)) {
            applied.push(AppliedMigrationRecord { hash, name });
        }
    }

    Ok(applied)
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
        .map_err(|e| CliError::Other(format!("Failed to create async runtime: {e}")))?;

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
        CliError::ConnectionError(format!("Failed to connect to Turso '{url}': {e}"))
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
                "Statement failed: {e}\n{s}"
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
    set: &Migrations,
    url: &str,
    auth_token: Option<&str>,
) -> Result<MigrationResult, CliError> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| CliError::Other(format!("Failed to create async runtime: {e}")))?;

    rt.block_on(run_turso_inner(set, url, auth_token))
}

#[cfg(feature = "turso")]
fn inspect_turso_migrations(
    set: &Migrations,
    url: &str,
    auth_token: Option<&str>,
) -> Result<MigrationPlan, CliError> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| CliError::Other(format!("Failed to create async runtime: {e}")))?;

    rt.block_on(inspect_turso_inner(set, url, auth_token))
}

#[cfg(feature = "turso")]
async fn inspect_turso_inner(
    set: &Migrations,
    url: &str,
    auth_token: Option<&str>,
) -> Result<MigrationPlan, CliError> {
    let builder =
        libsql::Builder::new_remote(url.to_string(), auth_token.unwrap_or("").to_string());

    let db = builder.build().await.map_err(|e| {
        CliError::ConnectionError(format!("Failed to connect to Turso '{url}': {e}"))
    })?;

    let conn = db
        .connect()
        .map_err(|e| CliError::ConnectionError(e.to_string()))?;

    ensure_sqlite_tracking_table_libsql(&conn, set).await?;
    let applied = query_applied_records_turso(&conn, set).await?;
    build_migration_plan(set, &applied)
}

#[cfg(feature = "turso")]
async fn run_turso_inner(
    set: &Migrations,
    url: &str,
    auth_token: Option<&str>,
) -> Result<MigrationResult, CliError> {
    let builder =
        libsql::Builder::new_remote(url.to_string(), auth_token.unwrap_or("").to_string());

    let db = builder.build().await.map_err(|e| {
        CliError::ConnectionError(format!("Failed to connect to Turso '{url}': {e}"))
    })?;

    let conn = db
        .connect()
        .map_err(|e| CliError::ConnectionError(e.to_string()))?;

    ensure_sqlite_tracking_table_libsql(&conn, set).await?;

    // Query applied migration names
    let applied_names = query_applied_names_turso(&conn, set).await?;

    // Get pending migrations
    let pending: Vec<_> = set.pending(&applied_names).collect();
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
        if let Err(e) = tx.execute(&set.record_migration_sql(migration), ()).await {
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
async fn query_applied_names_turso(
    conn: &libsql::Connection,
    set: &Migrations,
) -> Result<Vec<String>, CliError> {
    let Ok(mut rows) = conn.query(&set.applied_names_sql(), ()).await else {
        return Ok(vec![]); // Table might not exist yet
    };

    let mut names = Vec::new();
    while let Ok(Some(row)) = rows.next().await {
        if let Ok(name) = row.get::<String>(0) {
            names.push(name);
        }
    }

    Ok(names)
}

#[cfg(feature = "turso")]
async fn query_applied_records_turso(
    conn: &libsql::Connection,
    set: &Migrations,
) -> Result<Vec<AppliedMigrationRecord>, CliError> {
    let sql = format!(
        r#"SELECT hash, "name" FROM {} WHERE "name" IS NOT NULL ORDER BY id;"#,
        set.table_ident_sql()
    );
    let Ok(mut rows) = conn.query(&sql, ()).await else {
        return Ok(vec![]); // Table might not exist yet
    };

    let mut applied = Vec::new();
    while let Ok(Some(row)) = rows.next().await {
        if let (Ok(hash), Ok(name)) = (row.get::<String>(0), row.get::<String>(1)) {
            applied.push(AppliedMigrationRecord { hash, name });
        }
    }

    Ok(applied)
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

/// Introspect a database and write schema/snapshot files.
///
/// This is the main entry point for CLI introspection.
///
/// # Errors
///
/// Returns [`CliError`] if connecting to the database fails, if querying the
/// catalogs fails, if applying the configured snapshot filters fails, or if
/// writing the generated schema and snapshot files to disk fails.
#[allow(clippy::too_many_arguments)]
pub fn run_introspection(
    credentials: &Credentials,
    dialect: Dialect,
    out_dir: &Path,
    init_metadata: bool,
    breakpoints: bool,
    introspect_casing: Option<IntrospectCasing>,
    filters: &SnapshotFilters,
    migrations_table: &str,
    migrations_schema: &str,
) -> Result<IntrospectResult, CliError> {
    use drizzle_migrations::words::generate_migration_tag;

    // Perform introspection
    let mut result = introspect_database(credentials, dialect)?;
    apply_snapshot_filters(&mut result.snapshot, dialect, filters)?;
    if !filters.is_empty() || introspect_casing.is_some() {
        regenerate_schema_from_snapshot(&mut result, dialect, introspect_casing);
    }

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

    let journal_path = out_dir.join("meta").join("_journal.json");
    if journal_path.exists() {
        return Err(CliError::Other(
            "Detected old drizzle-kit migration folders. Upgrade them before writing new migrations."
                .to_string(),
        ));
    }

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
    let sql_statements =
        generate_introspect_migration(&empty_snapshot, &result.snapshot, breakpoints)?;

    // Write migration.sql: {out}/{tag}/migration.sql
    let migration_sql_path = migration_dir.join("migration.sql");
    let sql_content = format_migration_sql(&sql_statements, breakpoints);
    std::fs::write(&migration_sql_path, &sql_content).map_err(|e| {
        CliError::Other(format!(
            "Failed to write migration file '{}': {}",
            migration_sql_path.display(),
            e
        ))
    })?;

    // Update result with path
    result.snapshot_path = snapshot_path;

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

#[allow(unused_variables)] // params consumed inside feature-gated block
fn apply_init_metadata(
    credentials: &Credentials,
    dialect: Dialect,
    out_dir: &Path,
    migrations_table: &str,
    migrations_schema: &str,
) -> Result<(), CliError> {
    #[cfg(any(
        feature = "rusqlite",
        feature = "libsql",
        feature = "turso",
        feature = "postgres-sync",
        feature = "tokio-postgres",
        feature = "d1-http",
    ))]
    let set = load_migration_set(dialect, out_dir, migrations_table, migrations_schema)?;

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
                    init_turso_metadata(url, auth_token.as_deref(), &set)
                }
                #[cfg(not(feature = "turso"))]
                {
                    let _ = auth_token;
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

        Credentials::Postgres(creds) => {
            let _ = creds;
            core::cfg_select! {
                feature = "postgres-sync" => init_postgres_sync_metadata(creds, &set),
                feature = "tokio-postgres" => init_postgres_async_metadata(creds, &set),
                _ => Err(CliError::MissingDriver {
                    dialect: "PostgreSQL",
                    feature: "postgres-sync or tokio-postgres",
                }),
            }
        }

        #[cfg(feature = "d1-http")]
        Credentials::D1 {
            account_id,
            database_id,
            token,
        } => d1_http::init_metadata(&set, account_id, database_id, token),

        #[cfg(not(feature = "d1-http"))]
        Credentials::D1 { .. } => Err(CliError::MissingDriver {
            dialect: "Cloudflare D1 (HTTP)",
            feature: "d1-http",
        }),

        Credentials::AwsDataApi { .. } => Err(CliError::UnsupportedForDriver {
            operation: "Migration metadata init against AWS Data API",
            driver: "aws-data-api",
            hint: "AWS RDS Data API schema ops are not yet wired into this CLI. Seed the \
                   migrations table manually via `aws rds-data execute-statement` or \
                   tokio-postgres.",
        }),
    }
}

#[cfg(any(
    feature = "rusqlite",
    feature = "libsql",
    feature = "turso",
    feature = "postgres-sync",
    feature = "tokio-postgres",
    feature = "d1-http",
))]
pub(crate) fn validate_init_metadata(
    applied_names: &[String],
    set: &Migrations,
) -> Result<(), CliError> {
    if !applied_names.is_empty() {
        return Err(CliError::Other(
            "--init can't be used when database already has migrations set".into(),
        ));
    }

    if set.all().len() > 1 {
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
fn init_sqlite_metadata(path: &str, set: &Migrations) -> Result<(), CliError> {
    let conn = rusqlite::Connection::open(path).map_err(|e| {
        CliError::ConnectionError(format!("Failed to open SQLite database '{path}': {e}"))
    })?;

    ensure_sqlite_tracking_table(&conn, set)?;

    let applied_names = query_applied_names_sqlite(&conn, set)?;
    validate_init_metadata(&applied_names, set)?;

    let Some(first) = set.all().first() else {
        return Ok(());
    };

    conn.execute(&set.record_migration_sql(first), [])
        .map_err(|e| CliError::MigrationError(e.to_string()))?;

    Ok(())
}

#[cfg(feature = "libsql")]
fn init_libsql_local_metadata(path: &str, set: &Migrations) -> Result<(), CliError> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| CliError::Other(format!("Failed to create async runtime: {e}")))?;

    rt.block_on(init_libsql_local_metadata_inner(path, set))
}

#[cfg(feature = "libsql")]
async fn init_libsql_local_metadata_inner(path: &str, set: &Migrations) -> Result<(), CliError> {
    let db = libsql::Builder::new_local(path)
        .build()
        .await
        .map_err(|e| {
            CliError::ConnectionError(format!("Failed to open LibSQL database '{path}': {e}"))
        })?;

    let conn = db
        .connect()
        .map_err(|e| CliError::ConnectionError(e.to_string()))?;

    ensure_sqlite_tracking_table_libsql(&conn, set).await?;

    let applied_names = query_applied_names_libsql(&conn, set).await?;
    validate_init_metadata(&applied_names, set)?;

    let Some(first) = set.all().first() else {
        return Ok(());
    };

    conn.execute(&set.record_migration_sql(first), ())
        .await
        .map_err(|e| CliError::MigrationError(e.to_string()))?;

    Ok(())
}

#[cfg(feature = "turso")]
fn init_turso_metadata(
    url: &str,
    auth_token: Option<&str>,
    set: &Migrations,
) -> Result<(), CliError> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| CliError::Other(format!("Failed to create async runtime: {e}")))?;

    rt.block_on(init_turso_metadata_inner(url, auth_token, set))
}

#[cfg(feature = "turso")]
async fn init_turso_metadata_inner(
    url: &str,
    auth_token: Option<&str>,
    set: &Migrations,
) -> Result<(), CliError> {
    let builder =
        libsql::Builder::new_remote(url.to_string(), auth_token.unwrap_or("").to_string());

    let db = builder.build().await.map_err(|e| {
        CliError::ConnectionError(format!("Failed to connect to Turso '{url}': {e}"))
    })?;

    let conn = db
        .connect()
        .map_err(|e| CliError::ConnectionError(e.to_string()))?;

    ensure_sqlite_tracking_table_libsql(&conn, set).await?;

    let applied_names = query_applied_names_turso(&conn, set).await?;
    validate_init_metadata(&applied_names, set)?;

    let Some(first) = set.all().first() else {
        return Ok(());
    };

    conn.execute(&set.record_migration_sql(first), ())
        .await
        .map_err(|e| CliError::MigrationError(e.to_string()))?;

    Ok(())
}

#[cfg(feature = "postgres-sync")]
fn init_postgres_sync_metadata(creds: &PostgresCreds, set: &Migrations) -> Result<(), CliError> {
    let url = creds.connection_url();
    let mut client = postgres::Client::connect(&url, postgres::NoTls)
        .map_err(|e| CliError::ConnectionError(format!("Failed to connect to PostgreSQL: {e}")))?;

    if let Some(schema_sql) = set.create_schema_sql() {
        client
            .execute(&schema_sql, &[])
            .map_err(|e| CliError::MigrationError(e.to_string()))?;
    }

    ensure_postgres_tracking_table_sync(&mut client, set)?;

    let rows = client
        .query(&set.applied_names_sql(), &[])
        .unwrap_or_default();
    let applied_names: Vec<String> = rows.iter().filter_map(|r| r.try_get(0).ok()).collect();

    validate_init_metadata(&applied_names, set)?;

    let Some(first) = set.all().first() else {
        return Ok(());
    };

    client
        .execute(&set.record_migration_sql(first), &[])
        .map_err(|e| CliError::MigrationError(e.to_string()))?;

    Ok(())
}

#[cfg(all(feature = "tokio-postgres", not(feature = "postgres-sync")))]
fn init_postgres_async_metadata(creds: &PostgresCreds, set: &Migrations) -> Result<(), CliError> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| CliError::Other(format!("Failed to create async runtime: {}", e)))?;

    rt.block_on(init_postgres_async_inner(creds, set))
}

#[cfg(all(feature = "tokio-postgres", not(feature = "postgres-sync")))]
async fn init_postgres_async_inner(
    creds: &PostgresCreds,
    set: &Migrations,
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

    ensure_postgres_tracking_table_async(&client, set).await?;

    let rows = client
        .query(&set.applied_names_sql(), &[])
        .await
        .unwrap_or_default();
    let applied_names: Vec<String> = rows.iter().filter_map(|r| r.try_get(0).ok()).collect();

    validate_init_metadata(&applied_names, set)?;

    let Some(first) = set.all().first() else {
        return Ok(());
    };

    client
        .execute(&set.record_migration_sql(first), &[])
        .await
        .map_err(|e| CliError::MigrationError(e.to_string()))?;

    Ok(())
}

/// Generate migration SQL from snapshot diff (for introspection)
fn generate_introspect_migration(
    prev: &Snapshot,
    current: &Snapshot,
    breakpoints: bool,
) -> Result<Vec<String>, CliError> {
    match (prev, current) {
        (Snapshot::Sqlite(prev_snap), Snapshot::Sqlite(curr_snap)) => {
            use drizzle_migrations::sqlite::diff_snapshots;
            use drizzle_migrations::sqlite::statements::SqliteGenerator;

            let diff = diff_snapshots(prev_snap, curr_snap);
            let generator = SqliteGenerator::new().with_breakpoints(breakpoints);
            Ok(generator.generate_migration(&diff))
        }
        (Snapshot::Postgres(prev_snap), Snapshot::Postgres(curr_snap)) => {
            use drizzle_migrations::postgres::diff_full_snapshots;
            use drizzle_migrations::postgres::statements::PostgresGenerator;

            let diff = diff_full_snapshots(prev_snap, curr_snap);
            let generator = PostgresGenerator::new().with_breakpoints(breakpoints);
            Ok(generator.generate(&diff.diffs))
        }
        _ => Err(CliError::DialectMismatch),
    }
}

fn format_migration_sql(sql_statements: &[String], breakpoints: bool) -> String {
    if sql_statements.is_empty() {
        "-- No tables to create (empty database)\n".to_string()
    } else if breakpoints {
        sql_statements.join("\n--> statement-breakpoint\n")
    } else {
        sql_statements.join("\n\n")
    }
}

/// Apply the configured filters (tables, schemas, extensions) in-place on a
/// snapshot, removing entities that do not match.
///
/// # Errors
///
/// Returns [`CliError::DialectMismatch`] if `dialect` does not agree with the
/// variant of `snapshot`, or [`CliError`] if compiling a filter pattern fails.
pub fn apply_snapshot_filters(
    snapshot: &mut Snapshot,
    dialect: Dialect,
    filters: &SnapshotFilters,
) -> Result<(), CliError> {
    if filters.is_empty() {
        return Ok(());
    }

    match (dialect, snapshot) {
        (Dialect::Sqlite | Dialect::Turso, Snapshot::Sqlite(sqlite)) => {
            apply_sqlite_snapshot_filters(sqlite, filters)
        }
        (Dialect::Postgresql, Snapshot::Postgres(postgres)) => {
            apply_postgres_snapshot_filters(postgres, filters)
        }
        _ => Err(CliError::DialectMismatch),
    }
}

fn apply_sqlite_snapshot_filters(
    snapshot: &mut drizzle_migrations::sqlite::SQLiteSnapshot,
    filters: &SnapshotFilters,
) -> Result<(), CliError> {
    use drizzle_types::sqlite::ddl::SqliteEntity;
    use std::collections::HashSet;

    let table_patterns = compile_patterns(filters.tables.as_deref())?;
    if table_patterns.is_none() {
        return Ok(());
    }

    let mut keep_tables: HashSet<String> = HashSet::new();
    for entity in &snapshot.ddl {
        if let SqliteEntity::Table(table) = entity
            && matches_patterns(table.name.as_ref(), table_patterns.as_deref())
        {
            keep_tables.insert(table.name.to_string());
        }
    }

    snapshot.ddl.retain(|entity| match entity {
        SqliteEntity::Table(t) => keep_tables.contains(t.name.as_ref()),
        SqliteEntity::Column(c) => keep_tables.contains(c.table.as_ref()),
        SqliteEntity::Index(i) => keep_tables.contains(i.table.as_ref()),
        SqliteEntity::ForeignKey(fk) => {
            keep_tables.contains(fk.table.as_ref()) && keep_tables.contains(fk.table_to.as_ref())
        }
        SqliteEntity::PrimaryKey(pk) => keep_tables.contains(pk.table.as_ref()),
        SqliteEntity::UniqueConstraint(u) => keep_tables.contains(u.table.as_ref()),
        SqliteEntity::CheckConstraint(c) => keep_tables.contains(c.table.as_ref()),
        SqliteEntity::View(v) => matches_patterns(v.name.as_ref(), table_patterns.as_deref()),
    });

    Ok(())
}

fn apply_postgres_snapshot_filters(
    snapshot: &mut drizzle_migrations::postgres::PostgresSnapshot,
    filters: &SnapshotFilters,
) -> Result<(), CliError> {
    use drizzle_types::postgres::ddl::PostgresEntity;
    use std::collections::HashSet;

    let schema_patterns = compile_patterns(filters.schemas.as_deref())?;
    let table_patterns = compile_patterns(filters.tables.as_deref())?;
    let exclude_postgis = filters
        .extensions
        .as_ref()
        .is_some_and(|v| v.contains(&Extension::Postgis));

    let is_schema_allowed = |schema: &str| -> bool {
        if exclude_postgis && matches!(schema, "topology" | "tiger" | "tiger_data") {
            return false;
        }
        matches_patterns(schema, schema_patterns.as_deref())
    };

    let mut keep_tables: HashSet<(String, String)> = HashSet::new();
    for entity in &snapshot.ddl {
        if let PostgresEntity::Table(table) = entity {
            let schema = table.schema.as_ref();
            let name = table.name.as_ref();
            if !is_schema_allowed(schema) {
                continue;
            }
            if exclude_postgis
                && matches!(
                    name,
                    "spatial_ref_sys"
                        | "geometry_columns"
                        | "geography_columns"
                        | "raster_columns"
                        | "raster_overviews"
                )
            {
                continue;
            }

            if matches_patterns(name, table_patterns.as_deref()) {
                keep_tables.insert((schema.to_string(), name.to_string()));
            }
        }
    }

    let mut keep_schemas: HashSet<String> = keep_tables.iter().map(|(s, _)| s.clone()).collect();
    if table_patterns.is_none() {
        for entity in &snapshot.ddl {
            if let PostgresEntity::Schema(s) = entity
                && is_schema_allowed(s.name.as_ref())
            {
                keep_schemas.insert(s.name.to_string());
            }
        }
    }

    snapshot.ddl.retain(|entity| match entity {
        PostgresEntity::Schema(s) => keep_schemas.contains(s.name.as_ref()),
        PostgresEntity::Enum(e) => keep_schemas.contains(e.schema.as_ref()),
        PostgresEntity::Sequence(s) => keep_schemas.contains(s.schema.as_ref()),
        PostgresEntity::Role(_) | PostgresEntity::Privilege(_) => true,
        PostgresEntity::Policy(p) => {
            keep_tables.contains(&(p.schema.to_string(), p.table.to_string()))
        }
        PostgresEntity::Table(t) => {
            keep_tables.contains(&(t.schema.to_string(), t.name.to_string()))
        }
        PostgresEntity::Column(c) => {
            keep_tables.contains(&(c.schema.to_string(), c.table.to_string()))
        }
        PostgresEntity::Index(i) => {
            keep_tables.contains(&(i.schema.to_string(), i.table.to_string()))
        }
        PostgresEntity::ForeignKey(fk) => {
            keep_tables.contains(&(fk.schema.to_string(), fk.table.to_string()))
                && keep_tables.contains(&(fk.schema_to.to_string(), fk.table_to.to_string()))
        }
        PostgresEntity::PrimaryKey(pk) => {
            keep_tables.contains(&(pk.schema.to_string(), pk.table.to_string()))
        }
        PostgresEntity::UniqueConstraint(u) => {
            keep_tables.contains(&(u.schema.to_string(), u.table.to_string()))
        }
        PostgresEntity::CheckConstraint(c) => {
            keep_tables.contains(&(c.schema.to_string(), c.table.to_string()))
        }
        PostgresEntity::View(v) => {
            if !keep_schemas.contains(v.schema.as_ref()) {
                return false;
            }
            matches_patterns(v.name.as_ref(), table_patterns.as_deref())
        }
    });

    Ok(())
}

#[derive(Debug, Clone)]
struct FilterPattern {
    pattern: glob::Pattern,
    negated: bool,
}

fn compile_patterns(patterns: Option<&[String]>) -> Result<Option<Vec<FilterPattern>>, CliError> {
    let Some(patterns) = patterns else {
        return Ok(None);
    };
    if patterns.is_empty() {
        return Ok(None);
    }

    let mut compiled = Vec::with_capacity(patterns.len());
    for p in patterns {
        let raw = p.trim();
        let (negated, source) = raw
            .strip_prefix('!')
            .map_or((false, raw), |stripped| (true, stripped));
        if source.is_empty() {
            return Err(CliError::Other(format!(
                "invalid filter pattern '{p}': empty pattern"
            )));
        }

        compiled.push(FilterPattern {
            pattern: glob::Pattern::new(source)
                .map_err(|e| CliError::Other(format!("invalid filter pattern '{p}': {e}")))?,
            negated,
        });
    }
    Ok(Some(compiled))
}

fn matches_patterns(value: &str, patterns: Option<&[FilterPattern]>) -> bool {
    match patterns {
        None => true,
        Some(v) => {
            let has_positive = v.iter().any(|m| !m.negated);
            let mut matched_positive = false;

            for matcher in v {
                if matcher.negated {
                    if matcher.pattern.matches(value) {
                        return false;
                    }
                } else if matcher.pattern.matches(value) {
                    matched_positive = true;
                }
            }

            if has_positive { matched_positive } else { true }
        }
    }
}

fn regenerate_schema_from_snapshot(
    result: &mut IntrospectResult,
    dialect: Dialect,
    introspect_casing: Option<IntrospectCasing>,
) {
    match (&result.snapshot, dialect) {
        (Snapshot::Sqlite(snap), Dialect::Sqlite | Dialect::Turso) => {
            use drizzle_migrations::sqlite::SQLiteDDL;
            use drizzle_migrations::sqlite::codegen::{
                CodegenOptions, FieldCasing, generate_rust_schema,
            };

            let field_casing = match introspect_casing {
                Some(IntrospectCasing::Camel) => FieldCasing::Camel,
                Some(IntrospectCasing::Preserve) => FieldCasing::Preserve,
                None => FieldCasing::Snake,
            };

            let ddl = SQLiteDDL::from_entities(snap.ddl.clone());
            let generated = generate_rust_schema(
                &ddl,
                &CodegenOptions {
                    module_doc: Some("Schema introspected from filtered database objects".into()),
                    include_schema: true,
                    schema_name: "Schema".into(),
                    use_pub: true,
                    field_casing,
                },
            );

            result.schema_code = generated.code;
            result.table_count = generated.tables.len();
            result.index_count = generated.indexes.len();
            result.view_count = ddl.views.list().len();
            result.warnings = generated.warnings;
        }
        (Snapshot::Postgres(snap), Dialect::Postgresql) => {
            use drizzle_migrations::postgres::PostgresDDL;
            use drizzle_migrations::postgres::codegen::{
                CodegenOptions, FieldCasing, generate_rust_schema,
            };

            let field_casing = match introspect_casing {
                Some(IntrospectCasing::Camel) => FieldCasing::Camel,
                Some(IntrospectCasing::Preserve) => FieldCasing::Preserve,
                None => FieldCasing::Snake,
            };

            let ddl = PostgresDDL::from_entities(snap.ddl.clone());
            let generated = generate_rust_schema(
                &ddl,
                &CodegenOptions {
                    module_doc: Some("Schema introspected from filtered database objects".into()),
                    include_schema: true,
                    schema_name: "Schema".into(),
                    use_pub: true,
                    field_casing,
                },
            );

            result.schema_code = generated.code;
            result.table_count = generated.tables.len();
            result.index_count = generated.indexes.len();
            result.view_count = generated.views.len();
            result.warnings = generated.warnings;
        }
        _ => {}
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
                    introspect_turso(url, auth_token.as_deref())
                }
                #[cfg(not(feature = "turso"))]
                {
                    let _ = auth_token;
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

/// Introspect `PostgreSQL` databases
fn introspect_postgres_dialect(credentials: &Credentials) -> Result<IntrospectResult, CliError> {
    match credentials {
        Credentials::Postgres(creds) => {
            let _ = creds;
            core::cfg_select! {
                feature = "postgres-sync" => introspect_postgres_sync(creds),
                feature = "tokio-postgres" => introspect_postgres_async(creds),
                _ => Err(CliError::MissingDriver {
                    dialect: "PostgreSQL",
                    feature: "postgres-sync or tokio-postgres",
                }),
            }
        }

        _ => Err(CliError::Other(
            "PostgreSQL introspection requires postgres credentials".into(),
        )),
    }
}

// ============================================================================
// SQLite Introspection (rusqlite)
// ============================================================================

#[cfg(any(feature = "rusqlite", feature = "libsql", feature = "turso"))]
struct SqliteRawData {
    tables: Vec<(String, Option<String>)>,
    raw_columns: Vec<drizzle_migrations::sqlite::introspect::RawColumnInfo>,
    all_indexes: Vec<drizzle_migrations::sqlite::introspect::RawIndexInfo>,
    all_index_columns: Vec<drizzle_migrations::sqlite::introspect::RawIndexColumn>,
    all_fks: Vec<drizzle_migrations::sqlite::introspect::RawForeignKey>,
    all_views: Vec<drizzle_migrations::sqlite::introspect::RawViewInfo>,
}

#[cfg(any(feature = "rusqlite", feature = "libsql", feature = "turso"))]
impl SqliteRawData {
    const fn empty() -> Self {
        Self {
            tables: Vec::new(),
            raw_columns: Vec::new(),
            all_indexes: Vec::new(),
            all_index_columns: Vec::new(),
            all_fks: Vec::new(),
            all_views: Vec::new(),
        }
    }
}

#[cfg(feature = "rusqlite")]
fn query_rusqlite_tables_and_columns(
    conn: &rusqlite::Connection,
    raw: &mut SqliteRawData,
) -> Result<(), CliError> {
    use drizzle_migrations::sqlite::introspect::{RawColumnInfo, queries};

    let mut tables_stmt = conn
        .prepare(queries::TABLES_QUERY)
        .map_err(|e| CliError::Other(format!("Failed to prepare tables query: {e}")))?;

    raw.tables = tables_stmt
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
        .map_err(|e| CliError::Other(e.to_string()))?
        .filter_map(Result::ok)
        .collect();

    let mut columns_stmt = conn
        .prepare(queries::COLUMNS_QUERY)
        .map_err(|e| CliError::Other(format!("Failed to prepare columns query: {e}")))?;

    raw.raw_columns = columns_stmt
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

    Ok(())
}

#[cfg(feature = "rusqlite")]
fn query_rusqlite_per_table(
    conn: &rusqlite::Connection,
    raw: &mut SqliteRawData,
) -> Result<(), CliError> {
    use drizzle_migrations::sqlite::introspect::{
        RawForeignKey, RawIndexColumn, RawIndexInfo, queries,
    };

    // Collect table names up-front so we can mutate other fields on `raw` while iterating.
    let table_names: Vec<String> = raw.tables.iter().map(|(n, _)| n.clone()).collect();

    for table_name in &table_names {
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
                    raw.all_index_columns
                        .extend(col_iter.filter_map(Result::ok));
                }
            }
            raw.all_indexes.extend(indexes);
        }

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
            raw.all_fks.extend(fk_iter.filter_map(Result::ok));
        }
    }

    Ok(())
}

#[cfg(feature = "rusqlite")]
fn query_rusqlite_views_and_view_columns(conn: &rusqlite::Connection, raw: &mut SqliteRawData) {
    use drizzle_migrations::sqlite::introspect::{RawColumnInfo, RawViewInfo, queries};

    if let Ok(mut views_stmt) = conn.prepare(queries::VIEWS_QUERY)
        && let Ok(view_iter) = views_stmt.query_map([], |row| {
            Ok(RawViewInfo {
                name: row.get(0)?,
                sql: row.get(1)?,
            })
        })
    {
        raw.all_views.extend(view_iter.filter_map(Result::ok));
    }

    if let Ok(mut view_cols_stmt) = conn.prepare(queries::VIEW_COLUMNS_QUERY)
        && let Ok(col_iter) = view_cols_stmt.query_map([], |row| {
            Ok(RawColumnInfo {
                table: row.get(0)?,
                cid: row.get(1)?,
                name: row.get(2)?,
                column_type: row.get(3)?,
                not_null: row.get::<_, i32>(4)? != 0,
                default_value: row.get(5)?,
                pk: row.get(6)?,
                hidden: row.get(7)?,
                sql: row.get(8)?,
            })
        })
    {
        raw.raw_columns.extend(col_iter.filter_map(Result::ok));
    }
}

#[cfg(feature = "rusqlite")]
fn query_rusqlite_raw(conn: &rusqlite::Connection) -> Result<SqliteRawData, CliError> {
    let mut raw = SqliteRawData::empty();
    query_rusqlite_tables_and_columns(conn, &mut raw)?;
    query_rusqlite_per_table(conn, &mut raw)?;
    query_rusqlite_views_and_view_columns(conn, &mut raw);
    Ok(raw)
}

#[cfg(any(feature = "rusqlite", feature = "libsql", feature = "turso"))]
fn build_sqlite_ddl(raw: SqliteRawData) -> drizzle_migrations::sqlite::SQLiteDDL {
    use drizzle_migrations::sqlite::{
        SQLiteDDL, Table, View,
        introspect::{
            parse_generated_columns_from_table_sql, parse_view_sql, process_columns,
            process_foreign_keys, process_indexes,
        },
    };
    use std::collections::{HashMap, HashSet};

    let table_sql_map: HashMap<String, String> = raw
        .tables
        .iter()
        .filter_map(|(name, sql)| sql.as_ref().map(|s| (name.clone(), s.clone())))
        .collect();

    let mut generated_columns: HashMap<String, drizzle_migrations::sqlite::ddl::ParsedGenerated> =
        HashMap::new();
    for (table, sql) in &table_sql_map {
        generated_columns.extend(parse_generated_columns_from_table_sql(table, sql));
    }
    let pk_columns: HashSet<(String, String)> = raw
        .raw_columns
        .iter()
        .filter(|c| c.pk > 0)
        .map(|c| (c.table.clone(), c.name.clone()))
        .collect();

    let (columns, primary_keys) =
        process_columns(&raw.raw_columns, &generated_columns, &pk_columns);
    let indexes = process_indexes(&raw.all_indexes, &raw.all_index_columns, &table_sql_map);
    let foreign_keys = process_foreign_keys(&raw.all_fks);
    let uniques = process_sqlite_uniques_from_indexes(&raw.all_indexes, &raw.all_index_columns);

    let mut ddl = SQLiteDDL::new();

    for (table_name, table_sql) in &raw.tables {
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

    for v in raw.all_views {
        let mut view = View::new(v.name);
        if let Some(def) = parse_view_sql(&v.sql) {
            view.definition = Some(def.into());
        } else {
            view.error = Some("Failed to parse view SQL".into());
        }
        ddl.views.push(view);
    }

    ddl
}

#[cfg(feature = "rusqlite")]
fn introspect_rusqlite(path: &str) -> Result<IntrospectResult, CliError> {
    use drizzle_migrations::sqlite::codegen::{CodegenOptions, FieldCasing, generate_rust_schema};

    let conn = rusqlite::Connection::open(path).map_err(|e| {
        CliError::ConnectionError(format!("Failed to open SQLite database '{path}': {e}"))
    })?;

    let raw = query_rusqlite_raw(&conn)?;
    let ddl = build_sqlite_ddl(raw);

    let options = CodegenOptions {
        module_doc: Some(format!("Schema introspected from {path}")),
        include_schema: true,
        schema_name: "Schema".to_string(),
        use_pub: true,
        field_casing: FieldCasing::default(),
    };

    let generated = generate_rust_schema(&ddl, &options);

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
        .map_err(|e| CliError::Other(format!("Failed to create async runtime: {e}")))?;

    rt.block_on(introspect_libsql_inner(path, None))
}

#[cfg(any(feature = "libsql", feature = "turso"))]
async fn query_libsql_tables_and_columns(
    conn: &libsql::Connection,
) -> Result<
    (
        Vec<(String, Option<String>)>,
        Vec<drizzle_migrations::sqlite::introspect::RawColumnInfo>,
    ),
    CliError,
> {
    use drizzle_migrations::sqlite::introspect::{RawColumnInfo, queries};

    let mut tables_rows = conn
        .query(queries::TABLES_QUERY, ())
        .await
        .map_err(|e| CliError::Other(format!("Failed to query tables: {e}")))?;

    let mut tables: Vec<(String, Option<String>)> = Vec::new();
    while let Ok(Some(row)) = tables_rows.next().await {
        let name: String = row.get(0).unwrap_or_default();
        let sql: Option<String> = row.get(1).ok();
        tables.push((name, sql));
    }

    let mut columns_rows = conn
        .query(queries::COLUMNS_QUERY, ())
        .await
        .map_err(|e| CliError::Other(format!("Failed to query columns: {e}")))?;

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

    Ok((tables, raw_columns))
}

#[cfg(any(feature = "libsql", feature = "turso"))]
async fn query_libsql_per_table(
    conn: &libsql::Connection,
    tables: &[(String, Option<String>)],
) -> (
    Vec<drizzle_migrations::sqlite::introspect::RawIndexInfo>,
    Vec<drizzle_migrations::sqlite::introspect::RawIndexColumn>,
    Vec<drizzle_migrations::sqlite::introspect::RawForeignKey>,
) {
    use drizzle_migrations::sqlite::introspect::{
        RawForeignKey, RawIndexColumn, RawIndexInfo, queries,
    };

    let mut all_indexes: Vec<RawIndexInfo> = Vec::new();
    let mut all_index_columns: Vec<RawIndexColumn> = Vec::new();
    let mut all_fks: Vec<RawForeignKey> = Vec::new();

    for (table_name, _) in tables {
        if let Ok(mut idx_rows) = conn.query(&queries::indexes_query(table_name), ()).await {
            while let Ok(Some(row)) = idx_rows.next().await {
                let idx = RawIndexInfo {
                    table: table_name.clone(),
                    name: row.get(1).unwrap_or_default(),
                    unique: row.get::<i32>(2).unwrap_or(0) != 0,
                    origin: row.get(3).unwrap_or_default(),
                    partial: row.get::<i32>(4).unwrap_or(0) != 0,
                };

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

    (all_indexes, all_index_columns, all_fks)
}

#[cfg(any(feature = "libsql", feature = "turso"))]
async fn query_libsql_views_and_view_columns(
    conn: &libsql::Connection,
    raw_columns: &mut Vec<drizzle_migrations::sqlite::introspect::RawColumnInfo>,
) -> Vec<drizzle_migrations::sqlite::introspect::RawViewInfo> {
    use drizzle_migrations::sqlite::introspect::{RawColumnInfo, RawViewInfo, queries};

    let mut all_views: Vec<RawViewInfo> = Vec::new();
    if let Ok(mut views_rows) = conn.query(queries::VIEWS_QUERY, ()).await {
        while let Ok(Some(row)) = views_rows.next().await {
            let name: String = row.get(0).unwrap_or_default();
            let sql: String = row.get(1).unwrap_or_default();
            all_views.push(RawViewInfo { name, sql });
        }
    }

    if let Ok(mut view_cols_rows) = conn.query(queries::VIEW_COLUMNS_QUERY, ()).await {
        while let Ok(Some(row)) = view_cols_rows.next().await {
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
    }

    all_views
}

#[cfg(any(feature = "libsql", feature = "turso"))]
async fn query_libsql_raw(conn: &libsql::Connection) -> Result<SqliteRawData, CliError> {
    let (tables, mut raw_columns) = query_libsql_tables_and_columns(conn).await?;
    let (all_indexes, all_index_columns, all_fks) = query_libsql_per_table(conn, &tables).await;
    let all_views = query_libsql_views_and_view_columns(conn, &mut raw_columns).await;

    Ok(SqliteRawData {
        tables,
        raw_columns,
        all_indexes,
        all_index_columns,
        all_fks,
        all_views,
    })
}

#[cfg(feature = "libsql")]
async fn introspect_libsql_inner(
    path: &str,
    _auth_token: Option<&str>,
) -> Result<IntrospectResult, CliError> {
    use drizzle_migrations::sqlite::codegen::{CodegenOptions, FieldCasing, generate_rust_schema};

    let db = libsql::Builder::new_local(path)
        .build()
        .await
        .map_err(|e| {
            CliError::ConnectionError(format!("Failed to open LibSQL database '{path}': {e}"))
        })?;

    let conn = db
        .connect()
        .map_err(|e| CliError::ConnectionError(e.to_string()))?;

    let raw = query_libsql_raw(&conn).await?;
    let ddl = build_sqlite_ddl(raw);

    let options = CodegenOptions {
        module_doc: Some(format!("Schema introspected from {path}")),
        include_schema: true,
        schema_name: "Schema".to_string(),
        use_pub: true,
        field_casing: FieldCasing::default(),
    };

    let generated = generate_rust_schema(&ddl, &options);

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
        .map_err(|e| CliError::Other(format!("Failed to create async runtime: {e}")))?;

    rt.block_on(introspect_turso_inner(url, auth_token))
}

#[cfg(feature = "turso")]
async fn introspect_turso_inner(
    url: &str,
    auth_token: Option<&str>,
) -> Result<IntrospectResult, CliError> {
    use drizzle_migrations::sqlite::codegen::{CodegenOptions, FieldCasing, generate_rust_schema};

    let builder =
        libsql::Builder::new_remote(url.to_string(), auth_token.unwrap_or("").to_string());

    let db = builder.build().await.map_err(|e| {
        CliError::ConnectionError(format!("Failed to connect to Turso '{url}': {e}"))
    })?;

    let conn = db
        .connect()
        .map_err(|e| CliError::ConnectionError(e.to_string()))?;

    let raw = query_libsql_raw(&conn).await?;
    let ddl = build_sqlite_ddl(raw);

    let options = CodegenOptions {
        module_doc: Some(format!("Schema introspected from Turso: {url}")),
        include_schema: true,
        schema_name: "Schema".to_string(),
        use_pub: true,
        field_casing: FieldCasing::default(),
    };

    let generated = generate_rust_schema(&ddl, &options);

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
    let url = creds.connection_url();
    let mut client = postgres::Client::connect(&url, postgres::NoTls)
        .map_err(|e| CliError::ConnectionError(format!("Failed to connect to PostgreSQL: {e}")))?;

    let raw = query_postgres_sync_raw(&mut client)?;
    let ddl = build_postgres_ddl(raw);

    Ok(finalize_postgres_introspection(&ddl, &url))
}

#[cfg(any(feature = "postgres-sync", feature = "tokio-postgres"))]
impl PostgresRawData {
    const fn empty() -> Self {
        Self {
            schemas: Vec::new(),
            tables: Vec::new(),
            columns: Vec::new(),
            enums: Vec::new(),
            sequences: Vec::new(),
            views: Vec::new(),
            indexes: Vec::new(),
            foreign_keys: Vec::new(),
            primary_keys: Vec::new(),
            uniques: Vec::new(),
            checks: Vec::new(),
            roles: Vec::new(),
            policies: Vec::new(),
        }
    }
}

#[cfg(feature = "postgres-sync")]
fn query_postgres_sync_raw(client: &mut postgres::Client) -> Result<PostgresRawData, CliError> {
    let mut raw = PostgresRawData::empty();
    query_pg_sync_core(client, &mut raw)?;
    query_pg_sync_codegen_meta(client, &mut raw)?;
    query_pg_sync_constraints(client, &mut raw)?;
    query_pg_sync_security(client, &mut raw)?;
    Ok(raw)
}

#[cfg(feature = "postgres-sync")]
fn query_pg_sync_core(
    client: &mut postgres::Client,
    raw: &mut PostgresRawData,
) -> Result<(), CliError> {
    use drizzle_migrations::postgres::introspect::{RawColumnInfo, RawTableInfo, queries};

    raw.schemas = client
        .query(queries::SCHEMAS_QUERY, &[])
        .map_err(|e| CliError::Other(format!("Failed to query schemas: {e}")))?
        .into_iter()
        .map(|row| RawSchemaInfo {
            name: row.get::<_, String>(0),
        })
        .collect();

    raw.tables = client
        .query(queries::TABLES_QUERY, &[])
        .map_err(|e| CliError::Other(format!("Failed to query tables: {e}")))?
        .into_iter()
        .map(|row| RawTableInfo {
            schema: row.get::<_, String>(0),
            name: row.get::<_, String>(1),
            is_rls_enabled: row.get::<_, bool>(2),
        })
        .collect();

    raw.columns = client
        .query(queries::COLUMNS_QUERY, &[])
        .map_err(|e| CliError::Other(format!("Failed to query columns: {e}")))?
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

    Ok(())
}

#[cfg(feature = "postgres-sync")]
fn query_pg_sync_codegen_meta(
    client: &mut postgres::Client,
    raw: &mut PostgresRawData,
) -> Result<(), CliError> {
    use drizzle_migrations::postgres::introspect::{
        RawEnumInfo, RawSequenceInfo, RawViewInfo, queries,
    };

    raw.enums = client
        .query(queries::ENUMS_QUERY, &[])
        .map_err(|e| CliError::Other(format!("Failed to query enums: {e}")))?
        .into_iter()
        .map(|row| RawEnumInfo {
            schema: row.get::<_, String>(0),
            name: row.get::<_, String>(1),
            values: row.get::<_, Vec<String>>(2),
        })
        .collect();

    raw.sequences = client
        .query(queries::SEQUENCES_QUERY, &[])
        .map_err(|e| CliError::Other(format!("Failed to query sequences: {e}")))?
        .into_iter()
        .map(|row| RawSequenceInfo {
            schema: row.get::<_, String>(0),
            name: row.get::<_, String>(1),
            data_type: row.get::<_, Option<String>>(2),
            start_value: row.get::<_, Option<String>>(3),
            min_value: row.get::<_, Option<String>>(4),
            max_value: row.get::<_, Option<String>>(5),
            increment: row.get::<_, Option<String>>(6),
            cycle: row.get::<_, Option<bool>>(7),
            cache_value: row.get::<_, Option<String>>(8),
        })
        .collect();

    let view_schema_filters: Option<Vec<String>> = None;
    raw.views = client
        .query(queries::VIEWS_QUERY, &[&view_schema_filters])
        .map_err(|e| CliError::Other(format!("Failed to query views: {e}")))?
        .into_iter()
        .map(|row| RawViewInfo {
            schema: row.get::<_, String>(0),
            name: row.get::<_, String>(1),
            definition: row.get::<_, String>(2),
            is_materialized: row.get::<_, bool>(3),
        })
        .collect();

    Ok(())
}

#[cfg(feature = "postgres-sync")]
fn query_pg_sync_constraints(
    client: &mut postgres::Client,
    raw: &mut PostgresRawData,
) -> Result<(), CliError> {
    use drizzle_migrations::postgres::introspect::{
        RawCheckInfo, RawForeignKeyInfo, RawIndexInfo, RawPrimaryKeyInfo, RawUniqueInfo,
        parse_index_columns, pg_action_code_to_string, queries,
    };

    raw.indexes = client
        .query(queries::INDEXES_QUERY, &[])
        .map_err(|e| CliError::Other(format!("Failed to query indexes: {e}")))?
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
                columns: parse_index_columns(cols),
                where_clause: row.get::<_, Option<String>>(7),
                concurrent: false,
            }
        })
        .collect();

    raw.foreign_keys = client
        .query(queries::FOREIGN_KEYS_QUERY, &[])
        .map_err(|e| CliError::Other(format!("Failed to query foreign keys: {e}")))?
        .into_iter()
        .map(|row| RawForeignKeyInfo {
            schema: row.get::<_, String>(0),
            table: row.get::<_, String>(1),
            name: row.get::<_, String>(2),
            columns: row.get::<_, Vec<String>>(3),
            schema_to: row.get::<_, String>(4),
            table_to: row.get::<_, String>(5),
            columns_to: row.get::<_, Vec<String>>(6),
            on_update: pg_action_code_to_string(&row.get::<_, String>(7)),
            on_delete: pg_action_code_to_string(&row.get::<_, String>(8)),
        })
        .collect();

    raw.primary_keys = client
        .query(queries::PRIMARY_KEYS_QUERY, &[])
        .map_err(|e| CliError::Other(format!("Failed to query primary keys: {e}")))?
        .into_iter()
        .map(|row| RawPrimaryKeyInfo {
            schema: row.get::<_, String>(0),
            table: row.get::<_, String>(1),
            name: row.get::<_, String>(2),
            columns: row.get::<_, Vec<String>>(3),
        })
        .collect();

    raw.uniques = client
        .query(queries::UNIQUES_QUERY, &[])
        .map_err(|e| CliError::Other(format!("Failed to query unique constraints: {e}")))?
        .into_iter()
        .map(|row| RawUniqueInfo {
            schema: row.get::<_, String>(0),
            table: row.get::<_, String>(1),
            name: row.get::<_, String>(2),
            columns: row.get::<_, Vec<String>>(3),
            nulls_not_distinct: row.get::<_, bool>(4),
        })
        .collect();

    raw.checks = client
        .query(queries::CHECKS_QUERY, &[])
        .map_err(|e| CliError::Other(format!("Failed to query check constraints: {e}")))?
        .into_iter()
        .map(|row| RawCheckInfo {
            schema: row.get::<_, String>(0),
            table: row.get::<_, String>(1),
            name: row.get::<_, String>(2),
            expression: row.get::<_, String>(3),
        })
        .collect();

    Ok(())
}

#[cfg(feature = "postgres-sync")]
fn query_pg_sync_security(
    client: &mut postgres::Client,
    raw: &mut PostgresRawData,
) -> Result<(), CliError> {
    use drizzle_migrations::postgres::introspect::{RawPolicyInfo, RawRoleInfo, queries};

    raw.roles = client
        .query(queries::ROLES_QUERY, &[])
        .map_err(|e| CliError::Other(format!("Failed to query roles: {e}")))?
        .into_iter()
        .map(|row| RawRoleInfo {
            name: row.get::<_, String>(0),
            create_db: row.get::<_, bool>(1),
            create_role: row.get::<_, bool>(2),
            inherit: row.get::<_, bool>(3),
        })
        .collect();

    raw.policies = client
        .query(queries::POLICIES_QUERY, &[])
        .map_err(|e| CliError::Other(format!("Failed to query policies: {e}")))?
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

    Ok(())
}

#[cfg(all(not(feature = "postgres-sync"), feature = "tokio-postgres"))]
fn introspect_postgres_async(creds: &PostgresCreds) -> Result<IntrospectResult, CliError> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| CliError::Other(format!("Failed to create async runtime: {e}")))?;

    rt.block_on(introspect_postgres_async_inner(creds))
}

#[cfg(all(not(feature = "postgres-sync"), feature = "tokio-postgres"))]
async fn introspect_postgres_async_inner(
    creds: &PostgresCreds,
) -> Result<IntrospectResult, CliError> {
    let url = creds.connection_url();
    let (client, connection) = tokio_postgres::connect(&url, tokio_postgres::NoTls)
        .await
        .map_err(|e| CliError::ConnectionError(format!("Failed to connect to PostgreSQL: {e}")))?;

    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!(
                "{}",
                output::err_line(&format!("PostgreSQL connection error: {e}"))
            );
        }
    });

    let raw = query_postgres_async_raw(&client).await?;
    let ddl = build_postgres_ddl(raw);

    Ok(finalize_postgres_introspection(&ddl, &url))
}

#[cfg(all(not(feature = "postgres-sync"), feature = "tokio-postgres"))]
async fn query_postgres_async_raw(
    client: &tokio_postgres::Client,
) -> Result<PostgresRawData, CliError> {
    let mut raw = PostgresRawData::empty();
    query_pg_async_core(client, &mut raw).await?;
    query_pg_async_codegen_meta(client, &mut raw).await?;
    query_pg_async_constraints(client, &mut raw).await?;
    query_pg_async_security(client, &mut raw).await?;
    Ok(raw)
}

#[cfg(all(not(feature = "postgres-sync"), feature = "tokio-postgres"))]
async fn query_pg_async_core(
    client: &tokio_postgres::Client,
    raw: &mut PostgresRawData,
) -> Result<(), CliError> {
    use drizzle_migrations::postgres::introspect::{RawColumnInfo, RawTableInfo, queries};

    raw.schemas = client
        .query(queries::SCHEMAS_QUERY, &[])
        .await
        .map_err(|e| CliError::Other(format!("Failed to query schemas: {e}")))?
        .into_iter()
        .map(|row| RawSchemaInfo {
            name: row.get::<_, String>(0),
        })
        .collect();

    raw.tables = client
        .query(queries::TABLES_QUERY, &[])
        .await
        .map_err(|e| CliError::Other(format!("Failed to query tables: {e}")))?
        .into_iter()
        .map(|row| RawTableInfo {
            schema: row.get::<_, String>(0),
            name: row.get::<_, String>(1),
            is_rls_enabled: row.get::<_, bool>(2),
        })
        .collect();

    raw.columns = client
        .query(queries::COLUMNS_QUERY, &[])
        .await
        .map_err(|e| CliError::Other(format!("Failed to query columns: {e}")))?
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

    Ok(())
}

#[cfg(all(not(feature = "postgres-sync"), feature = "tokio-postgres"))]
async fn query_pg_async_codegen_meta(
    client: &tokio_postgres::Client,
    raw: &mut PostgresRawData,
) -> Result<(), CliError> {
    use drizzle_migrations::postgres::introspect::{
        RawEnumInfo, RawSequenceInfo, RawViewInfo, queries,
    };

    raw.enums = client
        .query(queries::ENUMS_QUERY, &[])
        .await
        .map_err(|e| CliError::Other(format!("Failed to query enums: {e}")))?
        .into_iter()
        .map(|row| RawEnumInfo {
            schema: row.get::<_, String>(0),
            name: row.get::<_, String>(1),
            values: row.get::<_, Vec<String>>(2),
        })
        .collect();

    raw.sequences = client
        .query(queries::SEQUENCES_QUERY, &[])
        .await
        .map_err(|e| CliError::Other(format!("Failed to query sequences: {e}")))?
        .into_iter()
        .map(|row| RawSequenceInfo {
            schema: row.get::<_, String>(0),
            name: row.get::<_, String>(1),
            data_type: row.get::<_, Option<String>>(2),
            start_value: row.get::<_, Option<String>>(3),
            min_value: row.get::<_, Option<String>>(4),
            max_value: row.get::<_, Option<String>>(5),
            increment: row.get::<_, Option<String>>(6),
            cycle: row.get::<_, Option<bool>>(7),
            cache_value: row.get::<_, Option<String>>(8),
        })
        .collect();

    let view_schema_filters: Option<Vec<String>> = None;
    raw.views = client
        .query(queries::VIEWS_QUERY, &[&view_schema_filters])
        .await
        .map_err(|e| CliError::Other(format!("Failed to query views: {e}")))?
        .into_iter()
        .map(|row| RawViewInfo {
            schema: row.get::<_, String>(0),
            name: row.get::<_, String>(1),
            definition: row.get::<_, String>(2),
            is_materialized: row.get::<_, bool>(3),
        })
        .collect();

    Ok(())
}

#[cfg(all(not(feature = "postgres-sync"), feature = "tokio-postgres"))]
async fn query_pg_async_constraints(
    client: &tokio_postgres::Client,
    raw: &mut PostgresRawData,
) -> Result<(), CliError> {
    use drizzle_migrations::postgres::introspect::{
        RawCheckInfo, RawForeignKeyInfo, RawIndexInfo, RawPrimaryKeyInfo, RawUniqueInfo,
        parse_index_columns, pg_action_code_to_string, queries,
    };

    raw.indexes = client
        .query(queries::INDEXES_QUERY, &[])
        .await
        .map_err(|e| CliError::Other(format!("Failed to query indexes: {e}")))?
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
                columns: parse_index_columns(cols),
                where_clause: row.get::<_, Option<String>>(7),
                concurrent: false,
            }
        })
        .collect();

    raw.foreign_keys = client
        .query(queries::FOREIGN_KEYS_QUERY, &[])
        .await
        .map_err(|e| CliError::Other(format!("Failed to query foreign keys: {e}")))?
        .into_iter()
        .map(|row| RawForeignKeyInfo {
            schema: row.get::<_, String>(0),
            table: row.get::<_, String>(1),
            name: row.get::<_, String>(2),
            columns: row.get::<_, Vec<String>>(3),
            schema_to: row.get::<_, String>(4),
            table_to: row.get::<_, String>(5),
            columns_to: row.get::<_, Vec<String>>(6),
            on_update: pg_action_code_to_string(&row.get::<_, String>(7)),
            on_delete: pg_action_code_to_string(&row.get::<_, String>(8)),
        })
        .collect();

    raw.primary_keys = client
        .query(queries::PRIMARY_KEYS_QUERY, &[])
        .await
        .map_err(|e| CliError::Other(format!("Failed to query primary keys: {e}")))?
        .into_iter()
        .map(|row| RawPrimaryKeyInfo {
            schema: row.get::<_, String>(0),
            table: row.get::<_, String>(1),
            name: row.get::<_, String>(2),
            columns: row.get::<_, Vec<String>>(3),
        })
        .collect();

    raw.uniques = client
        .query(queries::UNIQUES_QUERY, &[])
        .await
        .map_err(|e| CliError::Other(format!("Failed to query unique constraints: {e}")))?
        .into_iter()
        .map(|row| RawUniqueInfo {
            schema: row.get::<_, String>(0),
            table: row.get::<_, String>(1),
            name: row.get::<_, String>(2),
            columns: row.get::<_, Vec<String>>(3),
            nulls_not_distinct: row.get::<_, bool>(4),
        })
        .collect();

    raw.checks = client
        .query(queries::CHECKS_QUERY, &[])
        .await
        .map_err(|e| CliError::Other(format!("Failed to query check constraints: {e}")))?
        .into_iter()
        .map(|row| RawCheckInfo {
            schema: row.get::<_, String>(0),
            table: row.get::<_, String>(1),
            name: row.get::<_, String>(2),
            expression: row.get::<_, String>(3),
        })
        .collect();

    Ok(())
}

#[cfg(all(not(feature = "postgres-sync"), feature = "tokio-postgres"))]
async fn query_pg_async_security(
    client: &tokio_postgres::Client,
    raw: &mut PostgresRawData,
) -> Result<(), CliError> {
    use drizzle_migrations::postgres::introspect::{RawPolicyInfo, RawRoleInfo, queries};

    raw.roles = client
        .query(queries::ROLES_QUERY, &[])
        .await
        .map_err(|e| CliError::Other(format!("Failed to query roles: {e}")))?
        .into_iter()
        .map(|row| RawRoleInfo {
            name: row.get::<_, String>(0),
            create_db: row.get::<_, bool>(1),
            create_role: row.get::<_, bool>(2),
            inherit: row.get::<_, bool>(3),
        })
        .collect();

    raw.policies = client
        .query(queries::POLICIES_QUERY, &[])
        .await
        .map_err(|e| CliError::Other(format!("Failed to query policies: {e}")))?
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

    Ok(())
}

/// Minimal schema list for snapshot
#[cfg(any(feature = "postgres-sync", feature = "tokio-postgres"))]
#[derive(Debug, Clone)]
struct RawSchemaInfo {
    name: String,
}

/// Aggregated raw introspection data collected from a `PostgreSQL` database.
///
/// Shared between `postgres-sync` and `tokio-postgres` code paths — the
/// collection phase differs (blocking vs async), but the downstream
/// processing is identical.
#[cfg(any(feature = "postgres-sync", feature = "tokio-postgres"))]
struct PostgresRawData {
    schemas: Vec<RawSchemaInfo>,
    tables: Vec<drizzle_migrations::postgres::introspect::RawTableInfo>,
    columns: Vec<drizzle_migrations::postgres::introspect::RawColumnInfo>,
    enums: Vec<drizzle_migrations::postgres::introspect::RawEnumInfo>,
    sequences: Vec<drizzle_migrations::postgres::introspect::RawSequenceInfo>,
    views: Vec<drizzle_migrations::postgres::introspect::RawViewInfo>,
    indexes: Vec<drizzle_migrations::postgres::introspect::RawIndexInfo>,
    foreign_keys: Vec<drizzle_migrations::postgres::introspect::RawForeignKeyInfo>,
    primary_keys: Vec<drizzle_migrations::postgres::introspect::RawPrimaryKeyInfo>,
    uniques: Vec<drizzle_migrations::postgres::introspect::RawUniqueInfo>,
    checks: Vec<drizzle_migrations::postgres::introspect::RawCheckInfo>,
    roles: Vec<drizzle_migrations::postgres::introspect::RawRoleInfo>,
    policies: Vec<drizzle_migrations::postgres::introspect::RawPolicyInfo>,
}

/// Build a [`PostgresDDL`] from raw introspection data.
///
/// Identical across the sync and async paths — only the query phase differs.
#[cfg(any(feature = "postgres-sync", feature = "tokio-postgres"))]
fn build_postgres_ddl(
    raw: PostgresRawData,
) -> drizzle_migrations::postgres::collection::PostgresDDL {
    use drizzle_migrations::postgres::{
        PostgresDDL,
        ddl::Schema,
        introspect::{
            process_check_constraints, process_columns, process_enums, process_foreign_keys,
            process_indexes, process_policies, process_primary_keys, process_roles,
            process_sequences, process_tables, process_unique_constraints, process_views,
        },
    };

    let mut ddl = PostgresDDL::new();
    for s in raw.schemas.into_iter().map(|s| Schema::new(s.name)) {
        ddl.schemas.push(s);
    }
    for e in process_enums(&raw.enums) {
        ddl.enums.push(e);
    }
    for s in process_sequences(&raw.sequences) {
        ddl.sequences.push(s);
    }
    for r in process_roles(&raw.roles) {
        ddl.roles.push(r);
    }
    for p in process_policies(&raw.policies) {
        ddl.policies.push(p);
    }
    for t in process_tables(&raw.tables) {
        ddl.tables.push(t);
    }
    for c in process_columns(&raw.columns) {
        ddl.columns.push(c);
    }
    for i in process_indexes(&raw.indexes) {
        ddl.indexes.push(i);
    }
    for fk in process_foreign_keys(&raw.foreign_keys) {
        ddl.fks.push(fk);
    }
    for pk in process_primary_keys(&raw.primary_keys) {
        ddl.pks.push(pk);
    }
    for u in process_unique_constraints(&raw.uniques) {
        ddl.uniques.push(u);
    }
    for c in process_check_constraints(&raw.checks) {
        ddl.checks.push(c);
    }
    for v in process_views(&raw.views) {
        ddl.views.push(v);
    }
    ddl
}

/// Package a generated DDL + generated code into an [`IntrospectResult`].
#[cfg(any(feature = "postgres-sync", feature = "tokio-postgres"))]
fn finalize_postgres_introspection(
    ddl: &drizzle_migrations::postgres::collection::PostgresDDL,
    url: &str,
) -> IntrospectResult {
    use drizzle_migrations::postgres::codegen::{
        CodegenOptions, FieldCasing, generate_rust_schema,
    };

    let options = CodegenOptions {
        module_doc: Some(format!("Schema introspected from {}", mask_url(url))),
        include_schema: true,
        schema_name: "Schema".to_string(),
        use_pub: true,
        field_casing: FieldCasing::default(),
    };
    let generated = generate_rust_schema(ddl, &options);

    let mut snap = drizzle_migrations::postgres::PostgresSnapshot::new();
    for entity in ddl.to_entities() {
        snap.add_entity(entity);
    }

    IntrospectResult {
        schema_code: generated.code,
        table_count: ddl.tables.list().len(),
        index_count: ddl.indexes.list().len(),
        view_count: ddl.views.list().len(),
        warnings: generated.warnings,
        snapshot: Snapshot::Postgres(snap),
        snapshot_path: std::path::PathBuf::new(),
    }
}

#[cfg(any(feature = "postgres-sync", feature = "tokio-postgres"))]
fn mask_url(url: &str) -> String {
    if let Some(at) = url.find('@')
        && let Some(colon) = url[..at].rfind(':')
    {
        let scheme_end = url.find("://").map_or(0, |p| p + 3);
        if colon > scheme_end {
            return format!("{}****{}", &url[..=colon], &url[at..]);
        }
    }
    url.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(any(feature = "postgres-sync", feature = "tokio-postgres"))]
    fn test_postgres_url() -> String {
        std::env::var("DATABASE_URL")
            .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5432/drizzle_test".into())
    }

    #[cfg(any(feature = "postgres-sync", feature = "tokio-postgres"))]
    fn test_postgres_creds() -> crate::config::PostgresCreds {
        crate::config::PostgresCreds::Url(test_postgres_url().into_boxed_str())
    }

    #[cfg(any(feature = "postgres-sync", feature = "tokio-postgres"))]
    fn unique_pg_name(prefix: &str) -> String {
        use std::time::{SystemTime, UNIX_EPOCH};

        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time went backwards")
            .as_nanos();
        format!("{}_{}_{}", prefix, std::process::id(), nanos)
    }

    #[test]
    fn destructive_statement_detection_covers_drop_variants() {
        assert!(is_destructive_statement("DROP TABLE users;"));
        assert!(is_destructive_statement("DROP VIEW active_users;"));
        assert!(is_destructive_statement("DROP TYPE status;"));
        assert!(is_destructive_statement("DROP SCHEMA auth;"));
        assert!(is_destructive_statement("DROP ROLE app_user;"));
        assert!(is_destructive_statement(
            "DROP POLICY users_rls_policy ON users;"
        ));
        assert!(is_destructive_statement("TRUNCATE users;"));
        assert!(is_destructive_statement(
            "ALTER TABLE users DROP CONSTRAINT users_email_key;"
        ));

        assert!(!is_destructive_statement("CREATE TABLE users(id INTEGER);"));
        assert!(!is_destructive_statement(
            "ALTER TABLE users ADD COLUMN email text;"
        ));
    }

    #[cfg(feature = "rusqlite")]
    #[test]
    fn sqlite_migrations_run_both_when_created_at_collides() {
        // Regression for drizzle-orm beta.19: migration identity is the folder
        // name, not the `created_at` timestamp. Two migrations that share a
        // wall-second must both apply.
        use drizzle_migrations::{Migration, Migrations};

        let dir = tempfile::tempdir().expect("tempdir");
        let db_path = dir.path().join("migrate.sqlite");
        let db_path_str = db_path.to_string_lossy().to_string();

        let first_set = Migrations::new(
            vec![Migration::with_hash(
                "20230331141203_first",
                "hash_one",
                1_680_271_923_000,
                vec!["CREATE TABLE created_at_dedupe_a (id INTEGER PRIMARY KEY)".to_string()],
            )],
            drizzle_types::Dialect::SQLite,
        );

        let first =
            run_sqlite_migrations(&first_set, &db_path_str).expect("first migrate succeeds");
        assert_eq!(first.applied_count, 1);

        let second_set = Migrations::new(
            vec![
                Migration::with_hash(
                    "20230331141203_first",
                    "hash_one",
                    1_680_271_923_000,
                    vec!["CREATE TABLE created_at_dedupe_a (id INTEGER PRIMARY KEY)".to_string()],
                ),
                Migration::with_hash(
                    "20230331141203_second",
                    "hash_two",
                    1_680_271_923_000,
                    vec!["CREATE TABLE created_at_dedupe_b (id INTEGER PRIMARY KEY)".to_string()],
                ),
            ],
            drizzle_types::Dialect::SQLite,
        );

        let second =
            run_sqlite_migrations(&second_set, &db_path_str).expect("second migrate succeeds");
        assert_eq!(
            second.applied_count, 1,
            "only the second (newly introduced by name) migration should apply"
        );

        let conn = rusqlite::Connection::open(&db_path).expect("open sqlite");
        let rows: i64 = conn
            .query_row("SELECT COUNT(*) FROM __drizzle_migrations", [], |row| {
                row.get(0)
            })
            .expect("count migrations rows");
        assert_eq!(rows, 2, "both migration records should be stored");

        let table_b_exists: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='created_at_dedupe_b'",
                [],
                |row| row.get(0),
            )
            .expect("query sqlite_master");
        assert_eq!(
            table_b_exists, 1,
            "the new-name migration must execute even though created_at collides"
        );
    }

    #[cfg(feature = "rusqlite")]
    #[test]
    fn run_migrations_creates_metadata_table_with_no_local_migrations() {
        let dir = tempfile::tempdir().expect("tempdir");
        let db_path = dir.path().join("empty.sqlite");
        let migrations_dir = dir.path().join("migrations");
        std::fs::create_dir_all(&migrations_dir).expect("create migrations dir");

        let creds = crate::config::Credentials::Sqlite {
            path: db_path.to_string_lossy().to_string().into_boxed_str(),
        };

        let result = run_migrations(
            &creds,
            crate::config::Dialect::Sqlite,
            &migrations_dir,
            "__drizzle_migrations",
            "drizzle",
        )
        .expect("run migrations");
        assert_eq!(result.applied_count, 0);

        let conn = rusqlite::Connection::open(&db_path).expect("open sqlite");
        let exists: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='__drizzle_migrations'",
                [],
                |row| row.get(0),
            )
            .expect("check metadata table");
        assert_eq!(exists, 1, "migrations metadata table should be created");
    }

    #[cfg(any(
        feature = "rusqlite",
        feature = "libsql",
        feature = "turso",
        feature = "postgres-sync",
        feature = "tokio-postgres"
    ))]
    #[test]
    fn validate_init_metadata_matches_drizzle_orm_semantics() {
        use drizzle_migrations::{Migration, Migrations};

        let empty_set = Migrations::empty(drizzle_types::Dialect::SQLite);
        validate_init_metadata(&[], &empty_set).expect("empty local migrations should be allowed");

        let single = Migrations::new(
            vec![Migration::with_hash(
                "20230331141203_init",
                "hash_single",
                1_680_271_923_000,
                vec!["CREATE TABLE t(id INTEGER PRIMARY KEY)".to_string()],
            )],
            drizzle_types::Dialect::SQLite,
        );
        validate_init_metadata(&[], &single).expect("single local migration should be allowed");

        let multiple = Migrations::new(
            vec![
                Migration::with_hash(
                    "20230331141203_first",
                    "hash_a",
                    1_680_271_923_000,
                    vec!["CREATE TABLE a(id INTEGER PRIMARY KEY)".to_string()],
                ),
                Migration::with_hash(
                    "20230331150000_second",
                    "hash_b",
                    1_680_275_400_000,
                    vec!["CREATE TABLE b(id INTEGER PRIMARY KEY)".to_string()],
                ),
            ],
            drizzle_types::Dialect::SQLite,
        );

        let err =
            validate_init_metadata(&[], &multiple).expect_err("multiple local migrations rejected");
        assert_eq!(
            err.to_string(),
            "--init can't be used with existing migrations"
        );

        let err = validate_init_metadata(&["20230331141203_init".to_string()], &single)
            .expect_err("existing db metadata should be rejected");
        assert_eq!(
            err.to_string(),
            "--init can't be used when database already has migrations set"
        );
    }

    #[test]
    fn verify_applied_migrations_detects_hash_mismatch() {
        use drizzle_migrations::{Migration, Migrations};

        let set = Migrations::new(
            vec![Migration::with_hash(
                "20230331141203_verify",
                "local_hash",
                1_680_271_923_000,
                vec!["CREATE TABLE t(id INTEGER PRIMARY KEY)".to_string()],
            )],
            drizzle_types::Dialect::SQLite,
        );

        let applied = vec![AppliedMigrationRecord {
            hash: "db_hash".to_string(),
            name: "20230331141203_verify".to_string(),
        }];

        let err = verify_applied_migrations_consistency(&set, &applied)
            .expect_err("hash mismatch should fail verification");
        assert_eq!(
            err.to_string(),
            "Migration failed: Migration hash mismatch for 20230331141203_verify: database=db_hash, local=local_hash"
        );
    }

    #[test]
    fn build_migration_plan_counts_pending_statements() {
        use drizzle_migrations::{Migration, Migrations};

        let set = Migrations::new(
            vec![
                Migration::with_hash(
                    "20230331141203_first",
                    "hash_a",
                    1_680_271_923_000,
                    vec![
                        "CREATE TABLE a(id INTEGER PRIMARY KEY)".to_string(),
                        "CREATE INDEX a_id_idx ON a(id)".to_string(),
                    ],
                ),
                Migration::with_hash(
                    "20230331150000_second",
                    "hash_b",
                    1_680_275_400_000,
                    vec!["CREATE TABLE b(id INTEGER PRIMARY KEY)".to_string()],
                ),
            ],
            drizzle_types::Dialect::SQLite,
        );

        let applied = vec![AppliedMigrationRecord {
            hash: "hash_a".to_string(),
            name: "20230331141203_first".to_string(),
        }];

        let plan = build_migration_plan(&set, &applied).expect("build migration plan");
        assert_eq!(plan.applied_count, 1);
        assert_eq!(plan.pending_count, 1);
        assert_eq!(plan.pending_statements, 1);
        assert_eq!(plan.pending_migrations, vec!["20230331150000_second"]);
    }

    #[cfg(any(feature = "postgres-sync", feature = "tokio-postgres"))]
    #[test]
    fn postgres_concurrent_index_detection_is_case_insensitive() {
        assert!(is_postgres_concurrent_index_statement(
            "CREATE INDEX CONCURRENTLY users_email_idx ON users (email);"
        ));
        assert!(is_postgres_concurrent_index_statement(
            "CREATE UNIQUE INDEX CONCURRENTLY users_email_idx ON users (email);"
        ));
        assert!(is_postgres_concurrent_index_statement(
            "drop index concurrently users_email_idx;"
        ));
        assert!(!is_postgres_concurrent_index_statement(
            "CREATE INDEX users_email_idx ON users (email);"
        ));
    }

    #[cfg(any(feature = "postgres-sync", feature = "tokio-postgres"))]
    #[test]
    fn postgres_concurrent_index_detection_over_statement_list() {
        let with_concurrent = vec![
            "CREATE TABLE users(id integer);".to_string(),
            "CREATE INDEX CONCURRENTLY users_email_idx ON users (id);".to_string(),
        ];
        let without_concurrent = vec![
            "CREATE TABLE users(id integer);".to_string(),
            "CREATE INDEX users_email_idx ON users (id);".to_string(),
        ];

        assert!(has_postgres_concurrent_index(&with_concurrent));
        assert!(!has_postgres_concurrent_index(&without_concurrent));
    }

    #[test]
    fn generate_push_sql_includes_concurrent_postgres_index() {
        use crate::snapshot::parse_result_to_snapshot;
        use drizzle_migrations::parser::SchemaParser;
        use drizzle_types::Dialect;

        let previous = r#"
#[PostgresTable]
pub struct Users {
    #[column(primary)]
    pub id: i32,
    pub email: String,
}
"#;

        let current = r#"
#[PostgresTable]
pub struct Users {
    #[column(primary)]
    pub id: i32,
    pub email: String,
}

#[PostgresIndex(concurrent)]
pub struct UsersEmailIdx(Users::email);
"#;

        let prev =
            parse_result_to_snapshot(&SchemaParser::parse(previous), Dialect::PostgreSQL, None);
        let curr =
            parse_result_to_snapshot(&SchemaParser::parse(current), Dialect::PostgreSQL, None);

        let (sql, warnings) = generate_push_sql(&prev, &curr, false).expect("push sql generation");
        assert!(warnings.is_empty());
        assert_eq!(sql.len(), 1);
        assert_eq!(
            sql[0],
            "CREATE INDEX CONCURRENTLY \"users_email_idx\" ON \"users\" USING btree (\"email\" NULLS LAST);"
        );
    }

    #[test]
    fn filter_patterns_support_negation_globs() {
        let raw = vec![
            "users_*".to_string(),
            "!users_4".to_string(),
            "!ad*".to_string(),
        ];
        let patterns = compile_patterns(Some(&raw)).expect("compile patterns");

        assert!(matches_patterns("users_1", patterns.as_deref()));
        assert!(!matches_patterns("users_4", patterns.as_deref()));
        assert!(!matches_patterns("admin", patterns.as_deref()));
        assert!(!matches_patterns("audit", patterns.as_deref()));
    }

    #[test]
    fn postgres_table_filter_matches_table_name_only() {
        use crate::snapshot::parse_result_to_snapshot;
        use drizzle_migrations::parser::SchemaParser;
        use drizzle_migrations::schema::Snapshot;
        use drizzle_types::Dialect as BaseDialect;

        let code = r#"
#[PostgresTable(schema = "admin")]
pub struct AuditLog {
    #[column(primary)]
    pub id: i32,
}

#[PostgresTable(schema = "public")]
pub struct Users {
    #[column(primary)]
    pub id: i32,
}
"#;

        let parsed = SchemaParser::parse(code);
        let mut snapshot = parse_result_to_snapshot(&parsed, BaseDialect::PostgreSQL, None);
        let filters = SnapshotFilters {
            tables: Some(vec!["admin.*".to_string()]),
            schemas: None,
            extensions: None,
        };

        apply_snapshot_filters(&mut snapshot, crate::config::Dialect::Postgresql, &filters)
            .expect("apply filters");

        let remaining_tables = match snapshot {
            Snapshot::Postgres(s) => s
                .ddl
                .iter()
                .filter_map(|e| {
                    if let drizzle_types::postgres::ddl::PostgresEntity::Table(t) = e {
                        Some((t.schema.to_string(), t.name.to_string()))
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>(),
            _ => panic!("expected postgres snapshot"),
        };

        assert!(remaining_tables.is_empty());
    }

    #[test]
    fn postgres_schema_and_table_filters_intersect() {
        use crate::snapshot::parse_result_to_snapshot;
        use drizzle_migrations::parser::SchemaParser;
        use drizzle_migrations::schema::Snapshot;
        use drizzle_types::Dialect as BaseDialect;

        let code = r#"
#[PostgresTable(schema = "dev")]
pub struct UsersDev {
    #[column(primary)]
    pub id: i32,
}

#[PostgresTable(schema = "public")]
pub struct UsersPublic {
    #[column(primary)]
    pub id: i32,
}
"#;

        let parsed = SchemaParser::parse(code);
        let mut snapshot = parse_result_to_snapshot(&parsed, BaseDialect::PostgreSQL, None);
        let filters = SnapshotFilters {
            tables: Some(vec!["users_*".to_string()]),
            schemas: Some(vec!["!dev".to_string()]),
            extensions: None,
        };

        apply_snapshot_filters(&mut snapshot, crate::config::Dialect::Postgresql, &filters)
            .expect("apply filters");

        let remaining_tables = match snapshot {
            Snapshot::Postgres(s) => s
                .ddl
                .iter()
                .filter_map(|e| {
                    if let drizzle_types::postgres::ddl::PostgresEntity::Table(t) = e {
                        Some((t.schema.to_string(), t.name.to_string()))
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>(),
            _ => panic!("expected postgres snapshot"),
        };

        assert_eq!(
            remaining_tables,
            vec![("public".to_string(), "users_public".to_string())]
        );
    }

    #[test]
    fn postgres_extensions_filter_excludes_postgis_internal_objects() {
        use crate::snapshot::parse_result_to_snapshot;
        use drizzle_migrations::parser::SchemaParser;
        use drizzle_migrations::schema::Snapshot;
        use drizzle_types::Dialect as BaseDialect;

        let code = r#"
#[PostgresTable(schema = "topology")]
pub struct TopologyLayer {
    #[column(primary)]
    pub id: i32,
}

#[PostgresTable]
pub struct SpatialRefSys {
    #[column(primary)]
    pub id: i32,
}

#[PostgresTable]
pub struct Users {
    #[column(primary)]
    pub id: i32,
}
"#;

        let parsed = SchemaParser::parse(code);
        let mut snapshot = parse_result_to_snapshot(&parsed, BaseDialect::PostgreSQL, None);
        let filters = SnapshotFilters {
            tables: None,
            schemas: None,
            extensions: Some(vec![Extension::Postgis]),
        };

        apply_snapshot_filters(&mut snapshot, crate::config::Dialect::Postgresql, &filters)
            .expect("apply filters");

        let remaining_tables = match snapshot {
            Snapshot::Postgres(s) => s
                .ddl
                .iter()
                .filter_map(|e| {
                    if let drizzle_types::postgres::ddl::PostgresEntity::Table(t) = e {
                        Some((t.schema.to_string(), t.name.to_string()))
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>(),
            _ => panic!("expected postgres snapshot"),
        };

        assert_eq!(
            remaining_tables,
            vec![("public".to_string(), "users".to_string())]
        );
    }

    #[test]
    fn format_migration_sql_respects_breakpoints_flag() {
        let sql = vec![
            "CREATE TABLE users(id integer);".to_string(),
            "CREATE INDEX users_id_idx ON users(id);".to_string(),
        ];

        let with_breakpoints = format_migration_sql(&sql, true);
        assert_eq!(
            with_breakpoints,
            "CREATE TABLE users(id integer);\n--> statement-breakpoint\nCREATE INDEX users_id_idx ON users(id);"
        );

        let without_breakpoints = format_migration_sql(&sql, false);
        assert_eq!(
            without_breakpoints,
            "CREATE TABLE users(id integer);\n\nCREATE INDEX users_id_idx ON users(id);"
        );

        let empty = format_migration_sql(&[], false);
        assert_eq!(empty, "-- No tables to create (empty database)\n");
    }

    #[test]
    fn regenerate_sqlite_schema_applies_introspect_casing() {
        use crate::config::{Casing, IntrospectCasing};
        use crate::snapshot::parse_result_to_snapshot;
        use drizzle_migrations::parser::SchemaParser;
        use drizzle_types::Dialect as BaseDialect;

        let code = r#"
#[SQLiteTable]
pub struct AuditLogs {
    #[column(primary)]
    pub id: i64,
    pub user_name: String,
}
"#;

        let parsed = SchemaParser::parse(code);
        let snapshot =
            parse_result_to_snapshot(&parsed, BaseDialect::SQLite, Some(Casing::SnakeCase));

        let mut camel = IntrospectResult {
            schema_code: String::new(),
            table_count: 0,
            index_count: 0,
            view_count: 0,
            warnings: Vec::new(),
            snapshot: snapshot.clone(),
            snapshot_path: std::path::PathBuf::new(),
        };
        regenerate_schema_from_snapshot(
            &mut camel,
            crate::config::Dialect::Sqlite,
            Some(IntrospectCasing::Camel),
        );

        assert_eq!(
            camel.schema_code,
            "\
//! Auto-generated SQLite schema from introspection
//!
//! Schema introspected from filtered database objects

use drizzle::sqlite::prelude::*;

#[SQLiteTable(name = \"audit_logs\")]
pub struct AuditLogs {
    #[column(primary)]
    pub id: i64,
    pub userName: String,
}

#[derive(SQLiteSchema)]
pub struct Schema {
    pub auditLogs: AuditLogs,
}
"
        );

        let mut preserve = IntrospectResult {
            schema_code: String::new(),
            table_count: 0,
            index_count: 0,
            view_count: 0,
            warnings: Vec::new(),
            snapshot,
            snapshot_path: std::path::PathBuf::new(),
        };
        regenerate_schema_from_snapshot(
            &mut preserve,
            crate::config::Dialect::Sqlite,
            Some(IntrospectCasing::Preserve),
        );

        assert_eq!(
            preserve.schema_code,
            "\
//! Auto-generated SQLite schema from introspection
//!
//! Schema introspected from filtered database objects

use drizzle::sqlite::prelude::*;

#[SQLiteTable(name = \"audit_logs\")]
pub struct AuditLogs {
    #[column(primary)]
    pub id: i64,
    pub user_name: String,
}

#[derive(SQLiteSchema)]
pub struct Schema {
    pub audit_logs: AuditLogs,
}
"
        );
    }

    #[test]
    fn regenerate_postgres_schema_applies_introspect_casing() {
        use crate::config::{Casing, IntrospectCasing};
        use crate::snapshot::parse_result_to_snapshot;
        use drizzle_migrations::parser::SchemaParser;
        use drizzle_types::Dialect as BaseDialect;

        let code = r#"
#[PostgresTable]
pub struct AuditLogs {
    #[column(primary)]
    pub id: i32,
    pub user_name: String,
}
"#;

        let parsed = SchemaParser::parse(code);
        let snapshot =
            parse_result_to_snapshot(&parsed, BaseDialect::PostgreSQL, Some(Casing::SnakeCase));

        let mut camel = IntrospectResult {
            schema_code: String::new(),
            table_count: 0,
            index_count: 0,
            view_count: 0,
            warnings: Vec::new(),
            snapshot: snapshot.clone(),
            snapshot_path: std::path::PathBuf::new(),
        };
        regenerate_schema_from_snapshot(
            &mut camel,
            crate::config::Dialect::Postgresql,
            Some(IntrospectCasing::Camel),
        );

        assert_eq!(
            camel.schema_code,
            "\
//! Auto-generated PostgreSQL schema from introspection
//!
//! Schema introspected from filtered database objects

use drizzle::postgres::prelude::*;

#[PostgresTable]
pub struct AuditLogs {
    #[column(primary)]
    pub id: i32,
    pub userName: String,
}

#[derive(PostgresSchema)]
pub struct Schema {
    pub auditLogs: AuditLogs,
}
"
        );

        let mut preserve = IntrospectResult {
            schema_code: String::new(),
            table_count: 0,
            index_count: 0,
            view_count: 0,
            warnings: Vec::new(),
            snapshot,
            snapshot_path: std::path::PathBuf::new(),
        };
        regenerate_schema_from_snapshot(
            &mut preserve,
            crate::config::Dialect::Postgresql,
            Some(IntrospectCasing::Preserve),
        );

        assert_eq!(
            preserve.schema_code,
            "\
//! Auto-generated PostgreSQL schema from introspection
//!
//! Schema introspected from filtered database objects

use drizzle::postgres::prelude::*;

#[PostgresTable]
pub struct AuditLogs {
    #[column(primary)]
    pub id: i32,
    pub user_name: String,
}

#[derive(PostgresSchema)]
pub struct Schema {
    pub audit_logs: AuditLogs,
}
"
        );
    }

    #[cfg(feature = "postgres-sync")]
    #[test]
    fn postgres_sync_migrate_applies_concurrent_index_without_transaction() {
        use drizzle_migrations::{Migration, Migrations};
        use drizzle_types::Dialect;

        let creds = test_postgres_creds();
        let url = creds.connection_url();

        let mut setup_client = match postgres::Client::connect(&url, postgres::NoTls) {
            Ok(c) => c,
            Err(e) => {
                eprintln!(
                    "Skipping postgres_sync_migrate_applies_concurrent_index_without_transaction: {}",
                    e
                );
                return;
            }
        };

        let table = unique_pg_name("cli_sync_users");
        let index = format!("{}_email_idx", table);
        let migration_schema = unique_pg_name("cli_sync_mig");

        setup_client
            .batch_execute(&format!(
                "DROP TABLE IF EXISTS \"{table}\" CASCADE; \
                 CREATE TABLE \"{table}\" (id integer, email text NOT NULL); \
                 INSERT INTO \"{table}\" (id, email) VALUES (1, 'a@example.com');"
            ))
            .expect("setup table for concurrent index test");
        drop(setup_client);

        let migration_tag = format!("20260212000000_{table}");
        let migration_sql =
            format!("CREATE INDEX CONCURRENTLY \"{index}\" ON \"{table}\" (\"email\");");
        let set = Migrations::with_tracking(
            vec![Migration::new(&migration_tag, &migration_sql)],
            Dialect::PostgreSQL,
            drizzle_migrations::Tracking::POSTGRES.schema(migration_schema.clone()),
        );

        let result = run_postgres_sync_migrations(&set, &creds)
            .expect("sync migration with concurrent index should succeed");
        assert_eq!(result.applied_count, 1);

        let mut verify_client =
            postgres::Client::connect(&url, postgres::NoTls).expect("reconnect for verification");
        let exists: i64 = verify_client
            .query_one(
                "SELECT COUNT(*)::bigint FROM pg_indexes \
                 WHERE schemaname = 'public' AND tablename = $1 AND indexname = $2",
                &[&table, &index],
            )
            .expect("query pg_indexes")
            .get(0);
        assert_eq!(exists, 1, "concurrent index was not created");

        let _ = verify_client.batch_execute(&format!(
            "DROP TABLE IF EXISTS \"{table}\" CASCADE; \
             DROP SCHEMA IF EXISTS \"{migration_schema}\" CASCADE;"
        ));
    }

    #[cfg(feature = "postgres-sync")]
    #[test]
    fn postgres_sync_migrate_upgrades_legacy_tracking_table_and_applies_pending() {
        use drizzle_migrations::{Migration, Migrations};
        use drizzle_types::Dialect;

        let creds = test_postgres_creds();
        let url = creds.connection_url();
        let mut client = postgres::Client::connect(&url, postgres::NoTls)
            .expect("connect postgres for legacy tracking upgrade test");

        let applied_table = unique_pg_name("cli_sync_applied");
        let pending_table = unique_pg_name("cli_sync_pending");
        let migration_schema = unique_pg_name("cli_sync_tracking");

        client
            .batch_execute(&format!(
                "DROP TABLE IF EXISTS \"{applied_table}\" CASCADE; \
                 DROP TABLE IF EXISTS \"{pending_table}\" CASCADE; \
                 DROP SCHEMA IF EXISTS \"{migration_schema}\" CASCADE; \
                 CREATE SCHEMA \"{migration_schema}\"; \
                 CREATE TABLE \"{migration_schema}\".\"__drizzle_migrations\" (id SERIAL PRIMARY KEY, hash TEXT NOT NULL, created_at BIGINT); \
                 CREATE TABLE \"{applied_table}\" (id integer primary key);"
            ))
            .expect("setup legacy postgres tracking metadata");

        let first = Migration::new(
            &format!("20230331141203_{applied_table}"),
            &format!("CREATE TABLE \"{applied_table}\" (id integer primary key);"),
        );
        let second = Migration::new(
            &format!("20230331141204_{pending_table}"),
            &format!("CREATE TABLE \"{pending_table}\" (id integer primary key);"),
        );
        let set = Migrations::with_tracking(
            vec![first.clone(), second.clone()],
            Dialect::PostgreSQL,
            drizzle_migrations::Tracking::POSTGRES.schema(migration_schema.clone()),
        );

        client
            .execute(
                &format!(
                    "INSERT INTO \"{migration_schema}\".\"__drizzle_migrations\" (hash, created_at) VALUES ($1, $2)"
                ),
                &[&first.hash(), &first.created_at()],
            )
            .expect("insert legacy applied migration row");
        drop(client);

        let result = run_postgres_sync_migrations(&set, &creds)
            .expect("sync migration upgrade should succeed");
        assert_eq!(result.applied_count, 1);
        assert_eq!(result.applied_migrations, vec![second.hash().to_string()]);

        let mut verify_client =
            postgres::Client::connect(&url, postgres::NoTls).expect("reconnect for verification");
        let columns: Vec<String> = verify_client
            .query(
                "SELECT column_name FROM information_schema.columns WHERE table_schema = $1 AND table_name = '__drizzle_migrations' ORDER BY ordinal_position",
                &[&migration_schema],
            )
            .expect("query upgraded tracking columns")
            .into_iter()
            .map(|row| row.get(0))
            .collect();
        assert_eq!(
            columns,
            vec!["id", "hash", "created_at", "name", "applied_at"]
        );

        let rows: Vec<(String, i64, String, Option<String>)> = verify_client
            .query(
                &format!(
                    "SELECT hash, created_at, name, applied_at::text FROM \"{migration_schema}\".\"__drizzle_migrations\" ORDER BY id ASC"
                ),
                &[],
            )
            .expect("query upgraded metadata rows")
            .into_iter()
            .map(|row| (row.get(0), row.get(1), row.get(2), row.get(3)))
            .collect();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].0, first.hash());
        assert_eq!(rows[0].1, first.created_at());
        assert_eq!(rows[0].2, first.name());
        assert_eq!(rows[0].3, None);
        assert_eq!(rows[1].0, second.hash());
        assert_eq!(rows[1].1, second.created_at());
        assert_eq!(rows[1].2, second.name());
        assert!(rows[1].3.is_some());

        let pending_exists: i64 = verify_client
            .query_one(
                "SELECT COUNT(*)::bigint FROM information_schema.tables WHERE table_schema = 'public' AND table_name = $1",
                &[&pending_table],
            )
            .expect("query pending table")
            .get(0);
        assert_eq!(pending_exists, 1);

        let _ = verify_client.batch_execute(&format!(
            "DROP TABLE IF EXISTS \"{applied_table}\" CASCADE; \
             DROP TABLE IF EXISTS \"{pending_table}\" CASCADE; \
             DROP SCHEMA IF EXISTS \"{migration_schema}\" CASCADE;"
        ));
    }

    #[cfg(feature = "postgres-sync")]
    #[test]
    fn postgres_sync_migrate_upgrade_rejects_unmatched_legacy_rows() {
        use drizzle_migrations::{Migration, Migrations};
        use drizzle_types::Dialect;

        let creds = test_postgres_creds();
        let url = creds.connection_url();
        let mut client = postgres::Client::connect(&url, postgres::NoTls)
            .expect("connect postgres for unmatched legacy row test");

        let migration_schema = unique_pg_name("cli_sync_tracking_unmatched");
        client
            .batch_execute(&format!(
                "DROP SCHEMA IF EXISTS \"{migration_schema}\" CASCADE; \
                 CREATE SCHEMA \"{migration_schema}\"; \
                 CREATE TABLE \"{migration_schema}\".\"__drizzle_migrations\" (id SERIAL PRIMARY KEY, hash TEXT NOT NULL, created_at BIGINT);"
            ))
            .expect("setup unmatched legacy metadata");

        let migration = Migration::new(
            "20230331141203_cli_sync_first",
            "CREATE TABLE \"cli_sync_unmatched_target\" (id integer primary key);",
        );
        let set = Migrations::with_tracking(
            vec![migration],
            Dialect::PostgreSQL,
            drizzle_migrations::Tracking::POSTGRES.schema(migration_schema.clone()),
        );

        client
            .execute(
                &format!(
                    "INSERT INTO \"{migration_schema}\".\"__drizzle_migrations\" (hash, created_at) VALUES ($1, $2)"
                ),
                &[&"unknown_hash", &1_680_271_924_000_i64],
            )
            .expect("insert unmatched legacy row");
        drop(client);

        let err = run_postgres_sync_migrations(&set, &creds)
            .expect_err("unmatched legacy metadata should fail");
        assert!(err.to_string().contains("do not match local migrations"));

        let mut verify_client =
            postgres::Client::connect(&url, postgres::NoTls).expect("reconnect for verification");
        let columns: Vec<String> = verify_client
            .query(
                "SELECT column_name FROM information_schema.columns WHERE table_schema = $1 AND table_name = '__drizzle_migrations' ORDER BY ordinal_position",
                &[&migration_schema],
            )
            .expect("query legacy tracking columns")
            .into_iter()
            .map(|row| row.get(0))
            .collect();
        assert_eq!(columns, vec!["id", "hash", "created_at"]);

        let _ = verify_client.batch_execute(&format!(
            "DROP TABLE IF EXISTS \"cli_sync_unmatched_target\" CASCADE; \
             DROP SCHEMA IF EXISTS \"{migration_schema}\" CASCADE;"
        ));
    }

    #[cfg(feature = "tokio-postgres")]
    #[test]
    fn tokio_postgres_migrate_applies_concurrent_index_without_transaction() {
        use drizzle_migrations::{Migration, Migrations};
        use drizzle_types::Dialect;

        let creds = test_postgres_creds();
        let url = creds.connection_url();
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("create tokio runtime");

        rt.block_on(async {
            let (client, connection) = match tokio_postgres::connect(&url, tokio_postgres::NoTls).await
            {
                Ok(v) => v,
                Err(e) => {
                    eprintln!(
                        "Skipping tokio_postgres_migrate_applies_concurrent_index_without_transaction: {}",
                        e
                    );
                    return;
                }
            };

            tokio::spawn(async move {
                let _ = connection.await;
            });

            let table = unique_pg_name("cli_async_users");
            let index = format!("{}_email_idx", table);
            let migration_schema = unique_pg_name("cli_async_mig");

            client
                .batch_execute(&format!(
                    "DROP TABLE IF EXISTS \"{table}\" CASCADE; \
                     CREATE TABLE \"{table}\" (id integer, email text NOT NULL); \
                     INSERT INTO \"{table}\" (id, email) VALUES (1, 'a@example.com');"
                ))
                .await
                .expect("setup table for async concurrent index test");

            let migration_tag = format!("20260212000000_{table}");
            let migration_sql = format!(
                "CREATE INDEX CONCURRENTLY \"{index}\" ON \"{table}\" (\"email\");"
            );
            let set = Migrations::with_tracking(
                vec![Migration::new(&migration_tag, &migration_sql)],
                Dialect::PostgreSQL,
                drizzle_migrations::Tracking::POSTGRES.schema(migration_schema.clone()),
            );

            let result = run_postgres_async_inner(&set, &creds)
                .await
                .expect("async migration with concurrent index should succeed");
            assert_eq!(result.applied_count, 1);

            let exists: i64 = client
                .query_one(
                    "SELECT COUNT(*)::bigint FROM pg_indexes \
                     WHERE schemaname = 'public' AND tablename = $1 AND indexname = $2",
                    &[&table, &index],
                )
                .await
                .expect("query pg_indexes")
                .get(0);
            assert_eq!(exists, 1, "async concurrent index was not created");

            let _ = client
                .batch_execute(&format!(
                    "DROP TABLE IF EXISTS \"{table}\" CASCADE; \
                     DROP SCHEMA IF EXISTS \"{migration_schema}\" CASCADE;"
                ))
                .await;
        });
    }

    #[cfg(feature = "tokio-postgres")]
    #[test]
    fn tokio_postgres_migrate_upgrades_legacy_tracking_table_and_applies_pending() {
        use drizzle_migrations::{Migration, Migrations};
        use drizzle_types::Dialect;

        let creds = test_postgres_creds();
        let url = creds.connection_url();
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("create tokio runtime");

        rt.block_on(async {
            let (client, connection) = tokio_postgres::connect(&url, tokio_postgres::NoTls)
                .await
                .expect("connect tokio-postgres for legacy tracking upgrade test");

            tokio::spawn(async move {
                let _ = connection.await;
            });

            let applied_table = unique_pg_name("cli_async_applied");
            let pending_table = unique_pg_name("cli_async_pending");
            let migration_schema = unique_pg_name("cli_async_tracking");

            client
                .batch_execute(&format!(
                    "DROP TABLE IF EXISTS \"{applied_table}\" CASCADE; \
                     DROP TABLE IF EXISTS \"{pending_table}\" CASCADE; \
                     DROP SCHEMA IF EXISTS \"{migration_schema}\" CASCADE; \
                     CREATE SCHEMA \"{migration_schema}\"; \
                     CREATE TABLE \"{migration_schema}\".\"__drizzle_migrations\" (id SERIAL PRIMARY KEY, hash TEXT NOT NULL, created_at BIGINT); \
                     CREATE TABLE \"{applied_table}\" (id integer primary key);"
                ))
                .await
                .expect("setup legacy postgres tracking metadata");

            let first = Migration::new(
                &format!("20230331141203_{applied_table}"),
                &format!("CREATE TABLE \"{applied_table}\" (id integer primary key);"),
            );
            let second = Migration::new(
                &format!("20230331141204_{pending_table}"),
                &format!("CREATE TABLE \"{pending_table}\" (id integer primary key);"),
            );
            let set = Migrations::with_tracking(
                vec![first.clone(), second.clone()],
                Dialect::PostgreSQL,
                drizzle_migrations::Tracking::POSTGRES.schema(migration_schema.clone()),
            );

            client
                .execute(
                    &format!(
                        "INSERT INTO \"{migration_schema}\".\"__drizzle_migrations\" (hash, created_at) VALUES ($1, $2)"
                    ),
                    &[&first.hash(), &first.created_at()],
                )
                .await
                .expect("insert legacy applied migration row");

            let result = run_postgres_async_inner(&set, &creds)
                .await
                .expect("async migration upgrade should succeed");
            assert_eq!(result.applied_count, 1);
            assert_eq!(result.applied_migrations, vec![second.hash().to_string()]);

            let columns: Vec<String> = client
                .query(
                    "SELECT column_name FROM information_schema.columns WHERE table_schema = $1 AND table_name = '__drizzle_migrations' ORDER BY ordinal_position",
                    &[&migration_schema],
                )
                .await
                .expect("query upgraded tracking columns")
                .into_iter()
                .map(|row| row.get(0))
                .collect();
            assert_eq!(
                columns,
                vec!["id", "hash", "created_at", "name", "applied_at"]
            );

            let rows: Vec<(String, i64, String, Option<String>)> = client
                .query(
                    &format!(
                        "SELECT hash, created_at, name, applied_at::text FROM \"{migration_schema}\".\"__drizzle_migrations\" ORDER BY id ASC"
                    ),
                    &[],
                )
                .await
                .expect("query upgraded metadata rows")
                .into_iter()
                .map(|row| (row.get(0), row.get(1), row.get(2), row.get(3)))
                .collect();
            assert_eq!(rows.len(), 2);
            assert_eq!(rows[0].0, first.hash());
            assert_eq!(rows[0].1, first.created_at());
            assert_eq!(rows[0].2, first.name());
            assert_eq!(rows[0].3, None);
            assert_eq!(rows[1].0, second.hash());
            assert_eq!(rows[1].1, second.created_at());
            assert_eq!(rows[1].2, second.name());
            assert!(rows[1].3.is_some());

            let pending_exists: i64 = client
                .query_one(
                    "SELECT COUNT(*)::bigint FROM information_schema.tables WHERE table_schema = 'public' AND table_name = $1",
                    &[&pending_table],
                )
                .await
                .expect("query pending table")
                .get(0);
            assert_eq!(pending_exists, 1);

            let _ = client
                .batch_execute(&format!(
                    "DROP TABLE IF EXISTS \"{applied_table}\" CASCADE; \
                     DROP TABLE IF EXISTS \"{pending_table}\" CASCADE; \
                     DROP SCHEMA IF EXISTS \"{migration_schema}\" CASCADE;"
                ))
                .await;
        });
    }
}

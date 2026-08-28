//! MySQL CLI execution seams.
//!
//! Every migration lifecycle in this module uses one physical connection. That
//! matters because MySQL advisory locks are scoped to a server session, and
//! because DDL implicitly commits so the tracking journal must be durable
//! before the first migration statement is issued.

use crate::config::{Driver, MySQLCreds};
use crate::error::CliError;
use drizzle_migrations::Migrations;

#[cfg(any(feature = "mysql-sync", feature = "mysql-async"))]
use super::AppliedMigrationRecord;
use super::{MigrationPlan, MigrationResult};

#[cfg(any(feature = "mysql-sync", feature = "mysql-async"))]
const SESSION_INITIALIZATION_SQL: &str = "SET time_zone = '+00:00', sql_mode = REPLACE(@@SESSION.sql_mode, 'NO_UNSIGNED_SUBTRACTION', '')";
#[cfg(any(feature = "mysql-sync", feature = "mysql-async"))]
const LOCK_TIMEOUT_SECONDS: u32 = 30;

#[cfg(any(feature = "mysql-sync", feature = "mysql-async"))]
type MySqlRow = ::mysql_common::Row;

/// Plan MySQL migrations using the explicitly selected adapter.
///
/// The plan path takes the migration lock because checking/upgrading the
/// tracking table can perform MySQL DDL.
pub(super) fn plan_migrations(
    driver: Driver,
    creds: &MySQLCreds,
    set: &Migrations,
) -> Result<MigrationPlan, CliError> {
    let _ = (creds, set);
    match driver {
        Driver::MysqlSync => {
            #[cfg(feature = "mysql-sync")]
            {
                plan_sync(creds, set)
            }
            #[cfg(not(feature = "mysql-sync"))]
            {
                Err(missing_sync_driver())
            }
        }
        Driver::MysqlAsync => {
            #[cfg(feature = "mysql-async")]
            {
                plan_async(creds, set)
            }
            #[cfg(not(feature = "mysql-async"))]
            {
                Err(missing_async_driver())
            }
        }
        _ => Err(unexpected_driver(driver)),
    }
}

/// Run MySQL migrations using the explicitly selected adapter.
///
/// MySQL DDL can implicitly commit, so this intentionally never creates a
/// transaction. Each migration is recorded as dirty first, executed in
/// autocommit mode, and only then marked complete.
pub(super) fn run_migrations(
    driver: Driver,
    creds: &MySQLCreds,
    set: &Migrations,
    repair: bool,
) -> Result<MigrationResult, CliError> {
    let _ = (creds, set, repair);
    match driver {
        Driver::MysqlSync => {
            #[cfg(feature = "mysql-sync")]
            {
                run_sync(creds, set, repair)
            }
            #[cfg(not(feature = "mysql-sync"))]
            {
                Err(missing_sync_driver())
            }
        }
        Driver::MysqlAsync => {
            #[cfg(feature = "mysql-async")]
            {
                run_async(creds, set, repair)
            }
            #[cfg(not(feature = "mysql-async"))]
            {
                Err(missing_async_driver())
            }
        }
        _ => Err(unexpected_driver(driver)),
    }
}

/// Execute MySQL statements using the explicitly selected adapter.
///
/// Statements are deliberately executed one at a time in autocommit mode.
/// This is both how MySQL DDL behaves and how push must surface the exact
/// statement that failed.
pub(super) fn execute_statements(
    driver: Driver,
    creds: &MySQLCreds,
    statements: &[String],
) -> Result<(), CliError> {
    let _ = (creds, statements);
    match driver {
        Driver::MysqlSync => {
            #[cfg(feature = "mysql-sync")]
            {
                execute_sync_statements(creds, statements)
            }
            #[cfg(not(feature = "mysql-sync"))]
            {
                Err(missing_sync_driver())
            }
        }
        Driver::MysqlAsync => {
            #[cfg(feature = "mysql-async")]
            {
                execute_async_statements(creds, statements)
            }
            #[cfg(not(feature = "mysql-async"))]
            {
                Err(missing_async_driver())
            }
        }
        _ => Err(unexpected_driver(driver)),
    }
}

/// Seed MySQL migration metadata for `drizzle introspect --init`.
pub(super) fn init_metadata(
    driver: Driver,
    creds: &MySQLCreds,
    set: &Migrations,
) -> Result<(), CliError> {
    let _ = (creds, set);
    match driver {
        Driver::MysqlSync => {
            #[cfg(feature = "mysql-sync")]
            {
                init_sync(creds, set)
            }
            #[cfg(not(feature = "mysql-sync"))]
            {
                Err(missing_sync_driver())
            }
        }
        Driver::MysqlAsync => {
            #[cfg(feature = "mysql-async")]
            {
                init_async(creds, set)
            }
            #[cfg(not(feature = "mysql-async"))]
            {
                Err(missing_async_driver())
            }
        }
        _ => Err(unexpected_driver(driver)),
    }
}

/// Introspect a MySQL database through the explicitly selected adapter.
///
/// The complete catalog traversal, including every `SHOW CREATE VIEW`, stays
/// on one initialized connection. The query layer below owns transport-row
/// decoding; `drizzle-migrations` owns the canonical DDL assembly.
pub(super) fn introspect(
    driver: Driver,
    creds: &MySQLCreds,
) -> Result<super::IntrospectResult, CliError> {
    let _ = creds;
    match driver {
        Driver::MysqlSync => {
            #[cfg(feature = "mysql-sync")]
            {
                introspect_sync(creds)
            }
            #[cfg(not(feature = "mysql-sync"))]
            {
                Err(missing_sync_driver())
            }
        }
        Driver::MysqlAsync => {
            #[cfg(feature = "mysql-async")]
            {
                introspect_async(creds)
            }
            #[cfg(not(feature = "mysql-async"))]
            {
                Err(missing_async_driver())
            }
        }
        _ => Err(unexpected_driver(driver)),
    }
}

fn unexpected_driver(driver: Driver) -> CliError {
    CliError::Other(format!(
        "MySQL executor received non-MySQL driver '{}'",
        driver.as_str()
    ))
}

#[cfg(not(feature = "mysql-sync"))]
fn missing_sync_driver() -> CliError {
    CliError::MissingDriver {
        dialect: "MySQL",
        feature: "mysql-sync",
    }
}

#[cfg(not(feature = "mysql-async"))]
fn missing_async_driver() -> CliError {
    CliError::MissingDriver {
        dialect: "MySQL",
        feature: "mysql-async",
    }
}

#[cfg(any(feature = "mysql-sync", feature = "mysql-async"))]
fn repair_unavailable() -> CliError {
    CliError::UnsupportedForDriver {
        operation: "MySQL migration repair",
        driver: "mysql",
        hint: "MySQL repair needs a catalog-backed statement reconciler and is intentionally not implemented. Inspect the partial schema, resolve it manually, then either mark the dirty migration complete or delete its dirty tracking row before retrying.",
    }
}

#[cfg(any(feature = "mysql-sync", feature = "mysql-async"))]
fn dirty_migration_error(set: &Migrations, dirty_names: &[String]) -> CliError {
    set.interrupted_migration_error(dirty_names)
        .map(|error| CliError::MigrationError(error.to_string()))
        .unwrap_or_else(|| {
            CliError::MigrationError(
                "MySQL reported dirty migration metadata without a migration name".into(),
            )
        })
}

#[cfg(any(feature = "mysql-sync", feature = "mysql-async"))]
fn sql_string_literal(value: &str) -> String {
    value.replace('\'', "''")
}

// ============================================================================
// mysql (sync)
// ============================================================================

#[cfg(feature = "mysql-sync")]
pub(super) fn sync_options(creds: &MySQLCreds) -> Result<::mysql::Opts, CliError> {
    use crate::config::MySQLSslMode;

    match creds {
        MySQLCreds::Url(url) => ::mysql::Opts::from_url(url)
            .map_err(|error| CliError::ConnectionError(format!("Invalid MySQL URL: {error}"))),
        MySQLCreds::Host {
            host,
            port,
            user,
            password,
            database,
            ssl,
        } => {
            let ssl_opts = match ssl {
                MySQLSslMode::Disable => None,
                MySQLSslMode::Required => Some(
                    ::mysql::SslOpts::default()
                        .with_danger_skip_domain_validation(true)
                        .with_danger_accept_invalid_certs(true),
                ),
                MySQLSslMode::VerifyCa => {
                    Some(::mysql::SslOpts::default().with_danger_skip_domain_validation(true))
                }
                MySQLSslMode::VerifyIdentity => Some(::mysql::SslOpts::default()),
            };

            Ok(::mysql::Opts::from(
                ::mysql::OptsBuilder::new()
                    .ip_or_hostname(Some(host.as_ref()))
                    .tcp_port(*port)
                    .user(user.as_deref())
                    .pass(password.as_deref())
                    .db_name(Some(database.as_ref()))
                    .ssl_opts(ssl_opts),
            ))
        }
    }
}

#[cfg(feature = "mysql-sync")]
pub(super) fn connect_sync(creds: &MySQLCreds) -> Result<::mysql::Conn, CliError> {
    use ::mysql::prelude::Queryable as _;

    let mut connection = ::mysql::Conn::new(sync_options(creds)?)
        .map_err(|error| CliError::ConnectionError(error.to_string()))?;
    connection
        .query_drop(SESSION_INITIALIZATION_SQL)
        .map_err(|error| {
            CliError::ConnectionError(format!("Failed to initialize MySQL session: {error}"))
        })?;
    Ok(connection)
}

/// Execute one statement on an already-owned synchronous MySQL session.
#[cfg(feature = "mysql-sync")]
pub(super) fn execute_sync(
    connection: &mut impl ::mysql::prelude::Queryable,
    sql: &str,
) -> Result<(), CliError> {
    connection.query_drop(sql).map_err(|error| {
        CliError::MigrationError(format!("MySQL statement failed: {error}\n{sql}"))
    })
}

/// Query raw rows on an already-owned synchronous MySQL session.
///
/// Introspection must use this rather than reconnecting for each catalog
/// query, so session state and any future consistency setup remain intact.
#[cfg(feature = "mysql-sync")]
pub(super) fn query_sync_rows(
    connection: &mut impl ::mysql::prelude::Queryable,
    sql: &str,
) -> Result<Vec<MySqlRow>, CliError> {
    connection
        .query::<MySqlRow, _>(sql)
        .map_err(|error| CliError::MigrationError(format!("MySQL query failed: {error}\n{sql}")))
}

/// Query raw rows with parameters on an already-owned synchronous session.
///
/// MySQL catalog queries use `?` parameters for the selected database. Keep
/// that user-controlled value out of SQL text; only `SHOW CREATE VIEW` needs
/// a quoted identifier, and it is obtained from the catalog itself.
#[cfg(feature = "mysql-sync")]
pub(super) fn query_sync_rows_with_params(
    connection: &mut impl ::mysql::prelude::Queryable,
    sql: &str,
    params: ::mysql::Params,
) -> Result<Vec<MySqlRow>, CliError> {
    connection
        .exec::<MySqlRow, _, _>(sql, params)
        .map_err(|error| CliError::MigrationError(format!("MySQL query failed: {error}\n{sql}")))
}

#[cfg(feature = "mysql-sync")]
fn execute_sync_statements(creds: &MySQLCreds, statements: &[String]) -> Result<(), CliError> {
    let mut connection = connect_sync(creds)?;
    for statement in statements {
        let statement = statement.trim();
        if !statement.is_empty() {
            execute_sync(&mut connection, statement)?;
        }
    }
    Ok(())
}

#[cfg(feature = "mysql-sync")]
fn migration_lock_name_sync(
    connection: &mut ::mysql::Conn,
    set: &Migrations,
) -> Result<String, CliError> {
    let database = decode_selected_database(query_sync_rows(connection, "SELECT DATABASE()")?)?;
    Ok(set.mysql_advisory_lock_name(&database))
}

#[cfg(feature = "mysql-sync")]
fn plan_sync(creds: &MySQLCreds, set: &Migrations) -> Result<MigrationPlan, CliError> {
    let mut connection = connect_sync(creds)?;
    let lock_name = migration_lock_name_sync(&mut connection, set)?;
    with_migration_lock_sync(&mut connection, &lock_name, |connection| {
        ensure_tracking_table_sync(connection, set)?;
        let applied = query_applied_records_sync(connection, set)?;
        super::build_migration_plan(set, &applied)
    })
}

#[cfg(feature = "mysql-sync")]
fn run_sync(
    creds: &MySQLCreds,
    set: &Migrations,
    repair: bool,
) -> Result<MigrationResult, CliError> {
    let mut connection = connect_sync(creds)?;
    let lock_name = migration_lock_name_sync(&mut connection, set)?;
    with_migration_lock_sync(&mut connection, &lock_name, |connection| {
        run_locked_sync(connection, set, repair)
    })
}

#[cfg(feature = "mysql-sync")]
fn init_sync(creds: &MySQLCreds, set: &Migrations) -> Result<(), CliError> {
    let mut connection = connect_sync(creds)?;
    let lock_name = migration_lock_name_sync(&mut connection, set)?;
    with_migration_lock_sync(&mut connection, &lock_name, |connection| {
        ensure_tracking_table_sync(connection, set)?;
        let dirty_names = query_dirty_names_sync(connection, set)?;
        if !dirty_names.is_empty() {
            return Err(dirty_migration_error(set, &dirty_names));
        }
        let applied_names = query_applied_names_sync(connection, set)?;
        super::validate_init_metadata(&applied_names, set)?;

        if let Some(first) = set.all().first() {
            execute_sync(connection, &set.record_migration_sql(first))?;
        }
        Ok(())
    })
}

#[cfg(feature = "mysql-sync")]
fn with_migration_lock_sync<T>(
    connection: &mut ::mysql::Conn,
    lock_name: &str,
    operation: impl FnOnce(&mut ::mysql::Conn) -> Result<T, CliError>,
) -> Result<T, CliError> {
    acquire_migration_lock_sync(connection, lock_name)?;
    let result = operation(connection);
    let release = release_migration_lock_sync(connection, lock_name);
    match (result, release) {
        (Ok(value), Ok(())) => Ok(value),
        (Err(error), _) => Err(error),
        (Ok(_), Err(error)) => Err(error),
    }
}

#[cfg(feature = "mysql-sync")]
fn acquire_migration_lock_sync(
    connection: &mut ::mysql::Conn,
    lock_name: &str,
) -> Result<(), CliError> {
    let sql = format!(
        "SELECT GET_LOCK('{}', {LOCK_TIMEOUT_SECONDS})",
        sql_string_literal(lock_name)
    );
    let value = decode_lock_result(query_sync_rows(connection, &sql)?, "GET_LOCK")?;
    match value {
        Some(1) => Ok(()),
        Some(0) => Err(CliError::MigrationError(format!(
            "Timed out after {LOCK_TIMEOUT_SECONDS} seconds waiting for MySQL migration lock '{lock_name}'"
        ))),
        Some(value) => Err(CliError::MigrationError(format!(
            "MySQL returned unexpected GET_LOCK result {value} for '{lock_name}'"
        ))),
        None => Err(CliError::MigrationError(format!(
            "MySQL did not acquire migration lock '{lock_name}'"
        ))),
    }
}

#[cfg(feature = "mysql-sync")]
fn release_migration_lock_sync(
    connection: &mut ::mysql::Conn,
    lock_name: &str,
) -> Result<(), CliError> {
    let sql = format!("SELECT RELEASE_LOCK('{}')", sql_string_literal(lock_name));
    let value = decode_lock_result(query_sync_rows(connection, &sql)?, "RELEASE_LOCK")?;
    match value {
        Some(1) => Ok(()),
        Some(0) => Err(CliError::MigrationError(format!(
            "MySQL did not release migration lock '{lock_name}'"
        ))),
        Some(value) => Err(CliError::MigrationError(format!(
            "MySQL returned unexpected RELEASE_LOCK result {value} for '{lock_name}'"
        ))),
        None => Err(CliError::MigrationError(format!(
            "MySQL no longer recognizes migration lock '{lock_name}' while releasing it"
        ))),
    }
}

#[cfg(feature = "mysql-sync")]
fn ensure_tracking_table_sync(
    connection: &mut ::mysql::Conn,
    set: &Migrations,
) -> Result<(), CliError> {
    execute_sync(connection, &set.create_table_sql())?;
    let columns = decode_applied_names(query_sync_rows(connection, &column_names_sql(set))?)?;
    let has_name = columns.iter().any(|column| column == "name");
    let has_applied_at = columns.iter().any(|column| column == "applied_at");
    let applied = decode_legacy_metadata(query_sync_rows(
        connection,
        &legacy_metadata_sql(set, has_name, has_applied_at),
    )?)?;

    if !has_name {
        execute_sync(
            connection,
            &format!(
                "ALTER TABLE {} ADD COLUMN `name` TEXT NULL",
                set.table_ident_sql()
            ),
        )?;
    }
    if !has_applied_at {
        // A NULL default is intentional. Existing legacy rows are stamped
        // from created_at below; defaulting to CURRENT_TIMESTAMP would hide
        // a failed or incomplete migration-table upgrade as a new migration.
        execute_sync(
            connection,
            &format!(
                "ALTER TABLE {} ADD COLUMN `applied_at` TIMESTAMP NULL DEFAULT NULL",
                set.table_ident_sql()
            ),
        )?;
    }

    if applied.is_empty() {
        return Ok(());
    }
    let matched = drizzle_migrations::match_applied_migration_metadata(set.all(), &applied)
        .map_err(|error| CliError::MigrationError(error.to_string()))?;
    for row in &matched {
        execute_sync(connection, &set.backfill_migration_metadata_sql(row))?;
    }
    Ok(())
}

#[cfg(feature = "mysql-sync")]
fn query_applied_records_sync(
    connection: &mut ::mysql::Conn,
    set: &Migrations,
) -> Result<Vec<AppliedMigrationRecord>, CliError> {
    decode_applied_records(query_sync_rows(connection, &set.applied_records_sql())?)
}

#[cfg(feature = "mysql-sync")]
fn query_applied_names_sync(
    connection: &mut ::mysql::Conn,
    set: &Migrations,
) -> Result<Vec<String>, CliError> {
    decode_applied_names(query_sync_rows(connection, &set.applied_names_sql())?)
}

#[cfg(feature = "mysql-sync")]
fn run_locked_sync(
    connection: &mut ::mysql::Conn,
    set: &Migrations,
    repair: bool,
) -> Result<MigrationResult, CliError> {
    ensure_tracking_table_sync(connection, set)?;
    let dirty_names = query_dirty_names_sync(connection, set)?;
    if !dirty_names.is_empty() {
        if repair {
            return Err(repair_unavailable());
        }
        return Err(dirty_migration_error(set, &dirty_names));
    }

    let applied_names = query_applied_names_sync(connection, set)?;
    let pending = set.pending(&applied_names).collect::<Vec<_>>();
    let mut applied = Vec::with_capacity(pending.len());

    for migration in pending {
        execute_sync(connection, &set.record_migration_started_sql(migration))?;
        for statement in migration.statements() {
            let statement = statement.trim();
            if statement.is_empty() {
                continue;
            }
            // Do not clear the dirty row after a failed first statement. A
            // MySQL statement can still have made an implicit or partial DDL
            // effect, so the conservative journal must survive every error.
            execute_sync(connection, statement).map_err(|error| {
                CliError::MigrationError(format!(
                    "Migration '{}' failed after its dirty marker was recorded: {error}",
                    migration.tag()
                ))
            })?;
        }
        execute_sync(connection, &set.record_migration_finished_sql(migration))?;
        applied.push(migration.tag().to_owned());
    }

    Ok(MigrationResult {
        applied_count: applied.len(),
        applied_migrations: applied,
        repaired_migrations: Vec::new(),
    })
}

#[cfg(feature = "mysql-sync")]
fn query_dirty_names_sync(
    connection: &mut ::mysql::Conn,
    set: &Migrations,
) -> Result<Vec<String>, CliError> {
    decode_applied_names(query_sync_rows(connection, &set.dirty_names_sql())?)
}

#[cfg(feature = "mysql-sync")]
fn introspect_sync(creds: &MySQLCreds) -> Result<super::IntrospectResult, CliError> {
    let mut connection = connect_sync(creds)?;
    let raw = collect_introspection_sync(&mut connection)?;
    finalize_introspection(raw)
}

#[cfg(feature = "mysql-sync")]
fn collect_introspection_sync(
    connection: &mut ::mysql::Conn,
) -> Result<drizzle_migrations::mysql::introspect::RawIntrospection, CliError> {
    use drizzle_migrations::mysql::introspect::{RawIntrospection, queries};

    let database = decode_database(query_sync_rows(connection, queries::DATABASE)?)?;
    let database_name = database.name.clone();
    let mut raw = RawIntrospection {
        database,
        tables: decode_tables(query_sync_rows_with_params(
            connection,
            queries::TABLES,
            sync_database_params(&database_name),
        )?)?,
        columns: decode_columns(query_sync_rows_with_params(
            connection,
            queries::COLUMNS,
            sync_database_params(&database_name),
        )?)?,
        indexes: decode_indexes(query_sync_rows_with_params(
            connection,
            queries::INDEXES,
            sync_database_params(&database_name),
        )?)?,
        primary_keys: decode_primary_keys(query_sync_rows_with_params(
            connection,
            queries::PRIMARY_KEYS,
            sync_database_params(&database_name),
        )?)?,
        foreign_keys: decode_foreign_keys(query_sync_rows_with_params(
            connection,
            queries::FOREIGN_KEYS,
            sync_database_params(&database_name),
        )?)?,
        checks: decode_checks(query_sync_rows_with_params(
            connection,
            queries::CHECKS,
            sync_database_params(&database_name),
        )?)?,
        views: decode_views(query_sync_rows_with_params(
            connection,
            queries::VIEWS,
            sync_database_params(&database_name),
        )?)?,
    };

    enrich_views_sync(connection, &mut raw)?;
    Ok(raw)
}

#[cfg(feature = "mysql-sync")]
fn sync_database_params(database: &str) -> ::mysql::Params {
    ::mysql::Params::Positional(vec![::mysql::Value::from(database.to_owned())])
}

#[cfg(feature = "mysql-sync")]
fn enrich_views_sync(
    connection: &mut ::mysql::Conn,
    raw: &mut drizzle_migrations::mysql::introspect::RawIntrospection,
) -> Result<(), CliError> {
    for view in &mut raw.views {
        let sql = show_create_view_sql(&view.database, &view.name);
        let rows = query_sync_rows(connection, &sql)?;
        let create_sql = decode_show_create_view(rows, &view.name)?;
        drizzle_migrations::mysql::introspect::apply_show_create_view(view, &create_sql);
    }
    Ok(())
}

// ============================================================================
// mysql_async
// ============================================================================

#[cfg(feature = "mysql-async")]
pub(super) fn async_options(creds: &MySQLCreds) -> Result<::mysql_async::Opts, CliError> {
    use crate::config::MySQLSslMode;

    match creds {
        MySQLCreds::Url(url) => ::mysql_async::Opts::from_url(url)
            .map_err(|error| CliError::ConnectionError(format!("Invalid MySQL URL: {error}"))),
        MySQLCreds::Host {
            host,
            port,
            user,
            password,
            database,
            ssl,
        } => {
            let ssl_opts = match ssl {
                MySQLSslMode::Disable => None,
                MySQLSslMode::Required => Some(
                    ::mysql_async::SslOpts::default()
                        .with_danger_skip_domain_validation(true)
                        .with_danger_accept_invalid_certs(true),
                ),
                MySQLSslMode::VerifyCa => {
                    Some(::mysql_async::SslOpts::default().with_danger_skip_domain_validation(true))
                }
                MySQLSslMode::VerifyIdentity => Some(::mysql_async::SslOpts::default()),
            };

            Ok(::mysql_async::Opts::from(
                ::mysql_async::OptsBuilder::default()
                    .ip_or_hostname(host.as_ref())
                    .tcp_port(*port)
                    .user(user.as_deref())
                    .pass(password.as_deref())
                    .db_name(Some(database.as_ref()))
                    .ssl_opts(ssl_opts),
            ))
        }
    }
}

#[cfg(feature = "mysql-async")]
pub(super) async fn connect_async(creds: &MySQLCreds) -> Result<::mysql_async::Conn, CliError> {
    use ::mysql_async::prelude::Queryable as _;

    let mut connection = ::mysql_async::Conn::new(async_options(creds)?)
        .await
        .map_err(|error| CliError::ConnectionError(error.to_string()))?;
    connection
        .query_drop(SESSION_INITIALIZATION_SQL)
        .await
        .map_err(|error| {
            CliError::ConnectionError(format!("Failed to initialize MySQL session: {error}"))
        })?;
    Ok(connection)
}

/// Execute one statement on an already-owned asynchronous MySQL session.
#[cfg(feature = "mysql-async")]
pub(super) async fn execute_async<C>(connection: &mut C, sql: &str) -> Result<(), CliError>
where
    C: ::mysql_async::prelude::Queryable + ?Sized,
{
    connection.query_drop(sql).await.map_err(|error| {
        CliError::MigrationError(format!("MySQL statement failed: {error}\n{sql}"))
    })
}

/// Query raw rows on an already-owned asynchronous MySQL session.
///
/// The caller owns the connection for its full catalog traversal. In
/// particular, callers must not use a pool checkout per query while an
/// advisory lock or a consistent introspection session is in progress.
#[cfg(feature = "mysql-async")]
pub(super) async fn query_async_rows<C>(
    connection: &mut C,
    sql: &str,
) -> Result<Vec<MySqlRow>, CliError>
where
    C: ::mysql_async::prelude::Queryable + ?Sized,
{
    connection
        .query::<MySqlRow, _>(sql)
        .await
        .map_err(|error| CliError::MigrationError(format!("MySQL query failed: {error}\n{sql}")))
}

/// Query raw rows with parameters on an already-owned asynchronous session.
#[cfg(feature = "mysql-async")]
pub(super) async fn query_async_rows_with_params<C>(
    connection: &mut C,
    sql: &str,
    params: ::mysql_async::Params,
) -> Result<Vec<MySqlRow>, CliError>
where
    C: ::mysql_async::prelude::Queryable + ?Sized,
{
    connection
        .exec::<MySqlRow, _, _>(sql, params)
        .await
        .map_err(|error| CliError::MigrationError(format!("MySQL query failed: {error}\n{sql}")))
}

#[cfg(feature = "mysql-async")]
fn new_async_runtime() -> Result<tokio::runtime::Runtime, CliError> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|error| CliError::Other(format!("Failed to create async runtime: {error}")))
}

#[cfg(feature = "mysql-async")]
fn execute_async_statements(creds: &MySQLCreds, statements: &[String]) -> Result<(), CliError> {
    new_async_runtime()?.block_on(async {
        let mut connection = connect_async(creds).await?;
        for statement in statements {
            let statement = statement.trim();
            if !statement.is_empty() {
                execute_async(&mut connection, statement).await?;
            }
        }
        Ok(())
    })
}

#[cfg(feature = "mysql-async")]
async fn migration_lock_name_async(
    connection: &mut ::mysql_async::Conn,
    set: &Migrations,
) -> Result<String, CliError> {
    let database =
        decode_selected_database(query_async_rows(connection, "SELECT DATABASE()").await?)?;
    Ok(set.mysql_advisory_lock_name(&database))
}

#[cfg(feature = "mysql-async")]
fn plan_async(creds: &MySQLCreds, set: &Migrations) -> Result<MigrationPlan, CliError> {
    new_async_runtime()?.block_on(plan_async_inner(creds, set))
}

#[cfg(feature = "mysql-async")]
async fn plan_async_inner(creds: &MySQLCreds, set: &Migrations) -> Result<MigrationPlan, CliError> {
    let mut connection = connect_async(creds).await?;
    let lock_name = migration_lock_name_async(&mut connection, set).await?;
    acquire_migration_lock_async(&mut connection, &lock_name).await?;
    let result = async {
        ensure_tracking_table_async(&mut connection, set).await?;
        let applied = query_applied_records_async(&mut connection, set).await?;
        super::build_migration_plan(set, &applied)
    }
    .await;
    let release = release_migration_lock_async(&mut connection, &lock_name).await;
    finish_locked_result(result, release)
}

#[cfg(feature = "mysql-async")]
fn run_async(
    creds: &MySQLCreds,
    set: &Migrations,
    repair: bool,
) -> Result<MigrationResult, CliError> {
    new_async_runtime()?.block_on(run_async_inner(creds, set, repair))
}

#[cfg(feature = "mysql-async")]
async fn run_async_inner(
    creds: &MySQLCreds,
    set: &Migrations,
    repair: bool,
) -> Result<MigrationResult, CliError> {
    // This direct connection remains checked out from connection creation
    // through GET_LOCK, tracking-table work, every statement, and
    // RELEASE_LOCK. Do not replace it with per-query pool checkouts.
    let mut connection = connect_async(creds).await?;
    let lock_name = migration_lock_name_async(&mut connection, set).await?;
    acquire_migration_lock_async(&mut connection, &lock_name).await?;
    let result = run_locked_async(&mut connection, set, repair).await;
    let release = release_migration_lock_async(&mut connection, &lock_name).await;
    finish_locked_result(result, release)
}

#[cfg(feature = "mysql-async")]
fn init_async(creds: &MySQLCreds, set: &Migrations) -> Result<(), CliError> {
    new_async_runtime()?.block_on(init_async_inner(creds, set))
}

#[cfg(feature = "mysql-async")]
async fn init_async_inner(creds: &MySQLCreds, set: &Migrations) -> Result<(), CliError> {
    let mut connection = connect_async(creds).await?;
    let lock_name = migration_lock_name_async(&mut connection, set).await?;
    acquire_migration_lock_async(&mut connection, &lock_name).await?;
    let result = async {
        ensure_tracking_table_async(&mut connection, set).await?;
        let dirty_names = query_dirty_names_async(&mut connection, set).await?;
        if !dirty_names.is_empty() {
            return Err(dirty_migration_error(set, &dirty_names));
        }
        let applied_names = query_applied_names_async(&mut connection, set).await?;
        super::validate_init_metadata(&applied_names, set)?;
        if let Some(first) = set.all().first() {
            execute_async(&mut connection, &set.record_migration_sql(first)).await?;
        }
        Ok(())
    }
    .await;
    let release = release_migration_lock_async(&mut connection, &lock_name).await;
    finish_locked_result(result, release)
}

#[cfg(feature = "mysql-async")]
fn finish_locked_result<T>(
    result: Result<T, CliError>,
    release: Result<(), CliError>,
) -> Result<T, CliError> {
    match (result, release) {
        (Ok(value), Ok(())) => Ok(value),
        (Err(error), _) => Err(error),
        (Ok(_), Err(error)) => Err(error),
    }
}

#[cfg(feature = "mysql-async")]
async fn acquire_migration_lock_async(
    connection: &mut ::mysql_async::Conn,
    lock_name: &str,
) -> Result<(), CliError> {
    let sql = format!(
        "SELECT GET_LOCK('{}', {LOCK_TIMEOUT_SECONDS})",
        sql_string_literal(lock_name)
    );
    let value = decode_lock_result(query_async_rows(connection, &sql).await?, "GET_LOCK")?;
    match value {
        Some(1) => Ok(()),
        Some(0) => Err(CliError::MigrationError(format!(
            "Timed out after {LOCK_TIMEOUT_SECONDS} seconds waiting for MySQL migration lock '{lock_name}'"
        ))),
        Some(value) => Err(CliError::MigrationError(format!(
            "MySQL returned unexpected GET_LOCK result {value} for '{lock_name}'"
        ))),
        None => Err(CliError::MigrationError(format!(
            "MySQL did not acquire migration lock '{lock_name}'"
        ))),
    }
}

#[cfg(feature = "mysql-async")]
async fn release_migration_lock_async(
    connection: &mut ::mysql_async::Conn,
    lock_name: &str,
) -> Result<(), CliError> {
    let sql = format!("SELECT RELEASE_LOCK('{}')", sql_string_literal(lock_name));
    let value = decode_lock_result(query_async_rows(connection, &sql).await?, "RELEASE_LOCK")?;
    match value {
        Some(1) => Ok(()),
        Some(0) => Err(CliError::MigrationError(format!(
            "MySQL did not release migration lock '{lock_name}'"
        ))),
        Some(value) => Err(CliError::MigrationError(format!(
            "MySQL returned unexpected RELEASE_LOCK result {value} for '{lock_name}'"
        ))),
        None => Err(CliError::MigrationError(format!(
            "MySQL no longer recognizes migration lock '{lock_name}' while releasing it"
        ))),
    }
}

#[cfg(feature = "mysql-async")]
async fn ensure_tracking_table_async(
    connection: &mut ::mysql_async::Conn,
    set: &Migrations,
) -> Result<(), CliError> {
    execute_async(connection, &set.create_table_sql()).await?;
    let columns = query_column_names_async(connection, set).await?;
    let has_name = columns.iter().any(|column| column == "name");
    let has_applied_at = columns.iter().any(|column| column == "applied_at");

    let metadata_sql = legacy_metadata_sql(set, has_name, has_applied_at);
    let applied = decode_legacy_metadata(query_async_rows(connection, &metadata_sql).await?)?;

    if !has_name {
        execute_async(
            connection,
            &format!(
                "ALTER TABLE {} ADD COLUMN `name` TEXT NULL",
                set.table_ident_sql()
            ),
        )
        .await?;
    }
    if !has_applied_at {
        // A NULL default is intentional. Existing legacy rows are stamped
        // from created_at below; defaulting to CURRENT_TIMESTAMP would hide
        // a failed or incomplete migration-table upgrade as a new migration.
        execute_async(
            connection,
            &format!(
                "ALTER TABLE {} ADD COLUMN `applied_at` TIMESTAMP NULL DEFAULT NULL",
                set.table_ident_sql()
            ),
        )
        .await?;
    }

    backfill_legacy_metadata_async(connection, set, applied).await
}

#[cfg(feature = "mysql-async")]
async fn query_column_names_async(
    connection: &mut ::mysql_async::Conn,
    set: &Migrations,
) -> Result<Vec<String>, CliError> {
    decode_applied_names(query_async_rows(connection, &column_names_sql(set)).await?)
}

#[cfg(feature = "mysql-async")]
async fn backfill_legacy_metadata_async(
    connection: &mut ::mysql_async::Conn,
    set: &Migrations,
    applied: Vec<drizzle_migrations::AppliedMigrationMetadata>,
) -> Result<(), CliError> {
    if applied.is_empty() {
        return Ok(());
    }
    let matched = drizzle_migrations::match_applied_migration_metadata(set.all(), &applied)
        .map_err(|error| CliError::MigrationError(error.to_string()))?;
    for row in &matched {
        execute_async(connection, &set.backfill_migration_metadata_sql(row)).await?;
    }
    Ok(())
}

#[cfg(feature = "mysql-async")]
async fn query_applied_records_async(
    connection: &mut ::mysql_async::Conn,
    set: &Migrations,
) -> Result<Vec<AppliedMigrationRecord>, CliError> {
    decode_applied_records(query_async_rows(connection, &set.applied_records_sql()).await?)
}

#[cfg(feature = "mysql-async")]
async fn query_applied_names_async(
    connection: &mut ::mysql_async::Conn,
    set: &Migrations,
) -> Result<Vec<String>, CliError> {
    decode_applied_names(query_async_rows(connection, &set.applied_names_sql()).await?)
}

#[cfg(feature = "mysql-async")]
async fn query_dirty_names_async(
    connection: &mut ::mysql_async::Conn,
    set: &Migrations,
) -> Result<Vec<String>, CliError> {
    decode_applied_names(query_async_rows(connection, &set.dirty_names_sql()).await?)
}

#[cfg(feature = "mysql-async")]
fn introspect_async(creds: &MySQLCreds) -> Result<super::IntrospectResult, CliError> {
    new_async_runtime()?.block_on(introspect_async_inner(creds))
}

#[cfg(feature = "mysql-async")]
async fn introspect_async_inner(creds: &MySQLCreds) -> Result<super::IntrospectResult, CliError> {
    let mut connection = connect_async(creds).await?;
    let raw = collect_introspection_async(&mut connection).await?;
    finalize_introspection(raw)
}

#[cfg(feature = "mysql-async")]
async fn collect_introspection_async(
    connection: &mut ::mysql_async::Conn,
) -> Result<drizzle_migrations::mysql::introspect::RawIntrospection, CliError> {
    use drizzle_migrations::mysql::introspect::{RawIntrospection, queries};

    let database = decode_database(query_async_rows(connection, queries::DATABASE).await?)?;
    let database_name = database.name.clone();
    let mut raw = RawIntrospection {
        database,
        tables: decode_tables(
            query_async_rows_with_params(
                connection,
                queries::TABLES,
                async_database_params(&database_name),
            )
            .await?,
        )?,
        columns: decode_columns(
            query_async_rows_with_params(
                connection,
                queries::COLUMNS,
                async_database_params(&database_name),
            )
            .await?,
        )?,
        indexes: decode_indexes(
            query_async_rows_with_params(
                connection,
                queries::INDEXES,
                async_database_params(&database_name),
            )
            .await?,
        )?,
        primary_keys: decode_primary_keys(
            query_async_rows_with_params(
                connection,
                queries::PRIMARY_KEYS,
                async_database_params(&database_name),
            )
            .await?,
        )?,
        foreign_keys: decode_foreign_keys(
            query_async_rows_with_params(
                connection,
                queries::FOREIGN_KEYS,
                async_database_params(&database_name),
            )
            .await?,
        )?,
        checks: decode_checks(
            query_async_rows_with_params(
                connection,
                queries::CHECKS,
                async_database_params(&database_name),
            )
            .await?,
        )?,
        views: decode_views(
            query_async_rows_with_params(
                connection,
                queries::VIEWS,
                async_database_params(&database_name),
            )
            .await?,
        )?,
    };

    enrich_views_async(connection, &mut raw).await?;
    Ok(raw)
}

#[cfg(feature = "mysql-async")]
fn async_database_params(database: &str) -> ::mysql_async::Params {
    ::mysql_async::Params::Positional(vec![::mysql_async::Value::from(database.to_owned())])
}

#[cfg(feature = "mysql-async")]
async fn enrich_views_async(
    connection: &mut ::mysql_async::Conn,
    raw: &mut drizzle_migrations::mysql::introspect::RawIntrospection,
) -> Result<(), CliError> {
    for view in &mut raw.views {
        let sql = show_create_view_sql(&view.database, &view.name);
        let rows = query_async_rows(connection, &sql).await?;
        let create_sql = decode_show_create_view(rows, &view.name)?;
        drizzle_migrations::mysql::introspect::apply_show_create_view(view, &create_sql);
    }
    Ok(())
}

#[cfg(feature = "mysql-async")]
async fn run_locked_async(
    connection: &mut ::mysql_async::Conn,
    set: &Migrations,
    repair: bool,
) -> Result<MigrationResult, CliError> {
    ensure_tracking_table_async(connection, set).await?;
    let dirty_names = query_dirty_names_async(connection, set).await?;
    if !dirty_names.is_empty() {
        if repair {
            return Err(repair_unavailable());
        }
        return Err(dirty_migration_error(set, &dirty_names));
    }

    let applied_names = query_applied_names_async(connection, set).await?;
    let pending = set.pending(&applied_names).collect::<Vec<_>>();
    let mut applied = Vec::with_capacity(pending.len());

    for migration in pending {
        execute_async(connection, &set.record_migration_started_sql(migration)).await?;
        for statement in migration.statements() {
            let statement = statement.trim();
            if statement.is_empty() {
                continue;
            }
            // Keep the dirty marker even when this is the first failing
            // statement: MySQL can implicitly commit or partly apply DDL.
            execute_async(connection, statement)
                .await
                .map_err(|error| {
                    CliError::MigrationError(format!(
                        "Migration '{}' failed after its dirty marker was recorded: {error}",
                        migration.tag()
                    ))
                })?;
        }
        execute_async(connection, &set.record_migration_finished_sql(migration)).await?;
        applied.push(migration.tag().to_owned());
    }

    Ok(MigrationResult {
        applied_count: applied.len(),
        applied_migrations: applied,
        repaired_migrations: Vec::new(),
    })
}

// ============================================================================
// Shared MySQL row / tracking-table helpers
// ============================================================================

#[cfg(any(feature = "mysql-sync", feature = "mysql-async"))]
fn column_names_sql(set: &Migrations) -> String {
    format!(
        "SELECT column_name FROM information_schema.columns WHERE table_schema = DATABASE() AND table_name = '{}'",
        sql_string_literal(set.table_name())
    )
}

#[cfg(any(feature = "mysql-sync", feature = "mysql-async"))]
fn legacy_metadata_sql(set: &Migrations, has_name: bool, has_applied_at: bool) -> String {
    let filter = if has_name && has_applied_at {
        " WHERE `name` IS NULL"
    } else {
        ""
    };
    format!(
        "SELECT CAST(id AS SIGNED), `hash`, `created_at` FROM {}{filter} ORDER BY id ASC",
        set.table_ident_sql()
    )
}

#[cfg(any(feature = "mysql-sync", feature = "mysql-async"))]
fn row_value<T>(row: &MySqlRow, index: usize, field: &str) -> Result<T, CliError>
where
    T: ::mysql_common::prelude::FromValue,
{
    match row.get_opt::<T, _>(index) {
        Some(Ok(value)) => Ok(value),
        Some(Err(error)) => Err(CliError::MigrationError(format!(
            "MySQL returned an invalid value for {field}: {error}"
        ))),
        None => Err(CliError::MigrationError(format!(
            "MySQL result is missing required column {field}"
        ))),
    }
}

#[cfg(any(feature = "mysql-sync", feature = "mysql-async"))]
fn decode_lock_result(rows: Vec<MySqlRow>, operation: &str) -> Result<Option<i64>, CliError> {
    let mut rows = rows.into_iter();
    let row = rows.next().ok_or_else(|| {
        CliError::MigrationError(format!("MySQL {operation} returned no result row"))
    })?;
    if rows.next().is_some() {
        return Err(CliError::MigrationError(format!(
            "MySQL {operation} returned multiple result rows"
        )));
    }
    row_value(&row, 0, operation)
}

#[cfg(any(feature = "mysql-sync", feature = "mysql-async"))]
fn optional_string(row: &MySqlRow, index: usize, field: &str) -> Result<Option<String>, CliError> {
    row_value(row, index, field)
}

#[cfg(any(feature = "mysql-sync", feature = "mysql-async"))]
fn required_string(row: &MySqlRow, index: usize, field: &str) -> Result<String, CliError> {
    optional_string(row, index, field)?.ok_or_else(|| {
        CliError::MigrationError(format!("MySQL returned NULL for required column {field}"))
    })
}

#[cfg(any(feature = "mysql-sync", feature = "mysql-async"))]
fn required_u32(row: &MySqlRow, index: usize, field: &str) -> Result<u32, CliError> {
    let value = row_value::<u64>(row, index, field)?;
    u32::try_from(value).map_err(|_| {
        CliError::MigrationError(format!(
            "MySQL value {value} for {field} does not fit in a u32"
        ))
    })
}

#[cfg(any(feature = "mysql-sync", feature = "mysql-async"))]
fn catalog_boolean(value: &str, field: &str) -> Result<bool, CliError> {
    match value.trim().to_ascii_lowercase().as_str() {
        "1" | "yes" | "true" => Ok(true),
        "0" | "no" | "false" => Ok(false),
        _ => Err(CliError::MigrationError(format!(
            "MySQL returned unsupported boolean value '{value}' for {field}"
        ))),
    }
}

#[cfg(any(feature = "mysql-sync", feature = "mysql-async"))]
fn required_catalog_boolean(row: &MySqlRow, index: usize, field: &str) -> Result<bool, CliError> {
    catalog_boolean(&required_string(row, index, field)?, field)
}

#[cfg(any(feature = "mysql-sync", feature = "mysql-async"))]
fn optional_catalog_boolean(
    row: &MySqlRow,
    index: usize,
    field: &str,
) -> Result<Option<bool>, CliError> {
    optional_string(row, index, field)?
        .map(|value| catalog_boolean(&value, field))
        .transpose()
}

#[cfg(any(feature = "mysql-sync", feature = "mysql-async"))]
fn decode_applied_names(rows: Vec<MySqlRow>) -> Result<Vec<String>, CliError> {
    rows.iter()
        .map(|row| required_string(row, 0, "name"))
        .collect()
}

#[cfg(any(feature = "mysql-sync", feature = "mysql-async"))]
fn decode_applied_records(rows: Vec<MySqlRow>) -> Result<Vec<AppliedMigrationRecord>, CliError> {
    rows.iter()
        .map(|row| {
            Ok(AppliedMigrationRecord {
                hash: required_string(row, 0, "migration hash")?,
                name: required_string(row, 1, "migration name")?,
                dirty: row_value::<u64>(row, 2, "dirty marker")? != 0,
            })
        })
        .collect()
}

#[cfg(any(feature = "mysql-sync", feature = "mysql-async"))]
fn decode_legacy_metadata(
    rows: Vec<MySqlRow>,
) -> Result<Vec<drizzle_migrations::AppliedMigrationMetadata>, CliError> {
    rows.iter()
        .map(|row| {
            Ok(drizzle_migrations::AppliedMigrationMetadata {
                id: row_value(row, 0, "migration id")?,
                hash: required_string(row, 1, "migration hash")?,
                created_at: row_value(row, 2, "migration created_at")?,
            })
        })
        .collect()
}

#[cfg(any(feature = "mysql-sync", feature = "mysql-async"))]
fn decode_database(
    rows: Vec<MySqlRow>,
) -> Result<drizzle_migrations::mysql::introspect::RawDatabaseInfo, CliError> {
    let mut rows = rows.into_iter();
    let row = rows.next().ok_or_else(|| {
        CliError::ConnectionError("MySQL connection has no selected database".into())
    })?;
    if rows.next().is_some() {
        return Err(CliError::MigrationError(
            "MySQL catalog query for the selected database returned multiple rows".into(),
        ));
    }
    Ok(drizzle_migrations::mysql::introspect::RawDatabaseInfo {
        name: required_string(&row, 0, "schema name")?,
        default_engine: optional_string(&row, 1, "default storage engine")?,
        default_charset: optional_string(&row, 2, "default character set")?,
        default_collation: optional_string(&row, 3, "default collation")?,
    })
}

#[cfg(any(feature = "mysql-sync", feature = "mysql-async"))]
fn decode_selected_database(rows: Vec<MySqlRow>) -> Result<String, CliError> {
    let mut rows = rows.into_iter();
    let row = rows.next().ok_or_else(|| {
        CliError::ConnectionError("MySQL connection has no selected database".into())
    })?;
    if rows.next().is_some() {
        return Err(CliError::MigrationError(
            "MySQL selected-database query returned multiple rows".into(),
        ));
    }
    required_string(&row, 0, "selected database")
}

#[cfg(any(feature = "mysql-sync", feature = "mysql-async"))]
fn decode_tables(
    rows: Vec<MySqlRow>,
) -> Result<Vec<drizzle_migrations::mysql::introspect::RawTableInfo>, CliError> {
    rows.iter()
        .map(|row| {
            Ok(drizzle_migrations::mysql::introspect::RawTableInfo {
                database: required_string(row, 0, "TABLES.TABLE_SCHEMA")?,
                name: required_string(row, 1, "TABLES.TABLE_NAME")?,
                engine: optional_string(row, 2, "TABLES.ENGINE")?,
                charset: optional_string(row, 3, "TABLES.CHARACTER_SET_NAME")?,
                collation: optional_string(row, 4, "TABLES.TABLE_COLLATION")?,
                comment: optional_string(row, 5, "TABLES.TABLE_COMMENT")?,
            })
        })
        .collect()
}

#[cfg(any(feature = "mysql-sync", feature = "mysql-async"))]
fn decode_columns(
    rows: Vec<MySqlRow>,
) -> Result<Vec<drizzle_migrations::mysql::introspect::RawColumnInfo>, CliError> {
    rows.iter()
        .map(|row| {
            Ok(drizzle_migrations::mysql::introspect::RawColumnInfo {
                database: required_string(row, 0, "COLUMNS.TABLE_SCHEMA")?,
                table: required_string(row, 1, "COLUMNS.TABLE_NAME")?,
                name: required_string(row, 2, "COLUMNS.COLUMN_NAME")?,
                column_type: required_string(row, 3, "COLUMNS.COLUMN_TYPE")?,
                nullable: required_catalog_boolean(row, 4, "COLUMNS.IS_NULLABLE")?,
                default_value: optional_string(row, 5, "COLUMNS.COLUMN_DEFAULT")?,
                extra: required_string(row, 6, "COLUMNS.EXTRA")?,
                generation_expression: optional_string(row, 7, "COLUMNS.GENERATION_EXPRESSION")?,
                charset: optional_string(row, 8, "COLUMNS.CHARACTER_SET_NAME")?,
                collation: optional_string(row, 9, "COLUMNS.COLLATION_NAME")?,
                comment: optional_string(row, 10, "COLUMNS.COLUMN_COMMENT")?,
                ordinal_position: required_u32(row, 11, "COLUMNS.ORDINAL_POSITION")?,
            })
        })
        .collect()
}

#[cfg(any(feature = "mysql-sync", feature = "mysql-async"))]
fn decode_indexes(
    rows: Vec<MySqlRow>,
) -> Result<Vec<drizzle_migrations::mysql::introspect::RawIndexPart>, CliError> {
    rows.iter()
        .map(|row| {
            Ok(drizzle_migrations::mysql::introspect::RawIndexPart {
                database: required_string(row, 0, "STATISTICS.TABLE_SCHEMA")?,
                table: required_string(row, 1, "STATISTICS.TABLE_NAME")?,
                name: required_string(row, 2, "STATISTICS.INDEX_NAME")?,
                non_unique: row_value::<u64>(row, 3, "STATISTICS.NON_UNIQUE")? != 0,
                sequence: required_u32(row, 4, "STATISTICS.SEQ_IN_INDEX")?,
                column_name: optional_string(row, 5, "STATISTICS.COLUMN_NAME")?,
                expression: optional_string(row, 6, "STATISTICS.EXPRESSION")?,
                prefix_length: row_value::<Option<u64>>(row, 7, "STATISTICS.SUB_PART")?
                    .map(|value| {
                        u32::try_from(value).map_err(|_| {
                            CliError::MigrationError(format!(
                                "MySQL value {value} for STATISTICS.SUB_PART does not fit in a u32"
                            ))
                        })
                    })
                    .transpose()?,
                collation: optional_string(row, 8, "STATISTICS.COLLATION")?,
                index_type: optional_string(row, 9, "STATISTICS.INDEX_TYPE")?,
                comment: optional_string(row, 10, "STATISTICS.INDEX_COMMENT")?,
                visible: optional_catalog_boolean(row, 11, "STATISTICS.IS_VISIBLE")?,
            })
        })
        .collect()
}

#[cfg(any(feature = "mysql-sync", feature = "mysql-async"))]
fn decode_primary_keys(
    rows: Vec<MySqlRow>,
) -> Result<Vec<drizzle_migrations::mysql::introspect::RawPrimaryKeyPart>, CliError> {
    rows.iter()
        .map(|row| {
            Ok(drizzle_migrations::mysql::introspect::RawPrimaryKeyPart {
                database: required_string(row, 0, "PRIMARY_KEYS.TABLE_SCHEMA")?,
                table: required_string(row, 1, "PRIMARY_KEYS.TABLE_NAME")?,
                constraint_name: required_string(row, 2, "PRIMARY_KEYS.CONSTRAINT_NAME")?,
                column: required_string(row, 3, "PRIMARY_KEYS.COLUMN_NAME")?,
                ordinal_position: required_u32(row, 4, "PRIMARY_KEYS.ORDINAL_POSITION")?,
            })
        })
        .collect()
}

#[cfg(any(feature = "mysql-sync", feature = "mysql-async"))]
fn decode_foreign_keys(
    rows: Vec<MySqlRow>,
) -> Result<Vec<drizzle_migrations::mysql::introspect::RawForeignKeyPart>, CliError> {
    rows.iter()
        .map(|row| {
            Ok(drizzle_migrations::mysql::introspect::RawForeignKeyPart {
                database: required_string(row, 0, "FOREIGN_KEYS.TABLE_SCHEMA")?,
                table: required_string(row, 1, "FOREIGN_KEYS.TABLE_NAME")?,
                name: required_string(row, 2, "FOREIGN_KEYS.CONSTRAINT_NAME")?,
                column: required_string(row, 3, "FOREIGN_KEYS.COLUMN_NAME")?,
                ordinal_position: required_u32(row, 4, "FOREIGN_KEYS.ORDINAL_POSITION")?,
                foreign_database: required_string(row, 5, "FOREIGN_KEYS.REFERENCED_TABLE_SCHEMA")?,
                foreign_table: required_string(row, 6, "FOREIGN_KEYS.REFERENCED_TABLE_NAME")?,
                foreign_column: required_string(row, 7, "FOREIGN_KEYS.REFERENCED_COLUMN_NAME")?,
                on_update: required_string(row, 8, "FOREIGN_KEYS.UPDATE_RULE")?,
                on_delete: required_string(row, 9, "FOREIGN_KEYS.DELETE_RULE")?,
            })
        })
        .collect()
}

#[cfg(any(feature = "mysql-sync", feature = "mysql-async"))]
fn decode_checks(
    rows: Vec<MySqlRow>,
) -> Result<Vec<drizzle_migrations::mysql::introspect::RawCheckInfo>, CliError> {
    rows.iter()
        .map(|row| {
            Ok(drizzle_migrations::mysql::introspect::RawCheckInfo {
                database: required_string(row, 0, "CHECKS.TABLE_SCHEMA")?,
                table: required_string(row, 1, "CHECKS.TABLE_NAME")?,
                name: required_string(row, 2, "CHECKS.CONSTRAINT_NAME")?,
                expression: required_string(row, 3, "CHECKS.CHECK_CLAUSE")?,
                enforced: optional_catalog_boolean(row, 4, "CHECKS.ENFORCED")?,
            })
        })
        .collect()
}

#[cfg(any(feature = "mysql-sync", feature = "mysql-async"))]
fn decode_views(
    rows: Vec<MySqlRow>,
) -> Result<Vec<drizzle_migrations::mysql::introspect::RawViewInfo>, CliError> {
    rows.iter()
        .map(|row| {
            Ok(drizzle_migrations::mysql::introspect::RawViewInfo {
                database: required_string(row, 0, "VIEWS.TABLE_SCHEMA")?,
                name: required_string(row, 1, "VIEWS.TABLE_NAME")?,
                definition: required_string(row, 2, "VIEWS.VIEW_DEFINITION")?,
                algorithm: None,
                definer: optional_string(row, 3, "VIEWS.DEFINER")?,
                sql_security: optional_string(row, 4, "VIEWS.SECURITY_TYPE")?,
                check_option: optional_string(row, 5, "VIEWS.CHECK_OPTION")?,
                charset: optional_string(row, 6, "VIEWS.CHARACTER_SET_CLIENT")?,
                collation: optional_string(row, 7, "VIEWS.COLLATION_CONNECTION")?,
            })
        })
        .collect()
}

#[cfg(any(feature = "mysql-sync", feature = "mysql-async"))]
fn show_create_view_sql(database: &str, view: &str) -> String {
    format!(
        "SHOW CREATE VIEW `{}`.`{}`",
        database.replace('`', "``"),
        view.replace('`', "``")
    )
}

#[cfg(any(feature = "mysql-sync", feature = "mysql-async"))]
fn decode_show_create_view(rows: Vec<MySqlRow>, view: &str) -> Result<String, CliError> {
    let mut rows = rows.into_iter();
    let row = rows.next().ok_or_else(|| {
        CliError::MigrationError(format!("SHOW CREATE VIEW returned no row for `{view}`"))
    })?;
    if rows.next().is_some() {
        return Err(CliError::MigrationError(format!(
            "SHOW CREATE VIEW returned multiple rows for `{view}`"
        )));
    }
    required_string(&row, 1, "SHOW CREATE VIEW statement")
}

#[cfg(any(feature = "mysql-sync", feature = "mysql-async"))]
fn finalize_introspection(
    raw: drizzle_migrations::mysql::introspect::RawIntrospection,
) -> Result<super::IntrospectResult, CliError> {
    use drizzle_migrations::mysql::codegen::{CodegenOptions, FieldCasing, generate_rust_schema};
    use drizzle_migrations::mysql::introspect::{IntrospectionResult, assemble_ddl};
    use drizzle_migrations::schema::Snapshot;

    let catalog_defaults = raw.database.catalog_defaults();
    let ddl = assemble_ddl(raw)
        .map_err(|error| CliError::MigrationError(format!("Invalid MySQL catalog: {error}")))?;
    let generated = generate_rust_schema(
        &ddl,
        &CodegenOptions {
            module_doc: Some("Schema introspected from MySQL".into()),
            include_schema: true,
            schema_name: "Schema".into(),
            use_pub: true,
            field_casing: FieldCasing::default(),
        },
    )
    .map_err(|error| {
        CliError::MigrationError(format!(
            "Cannot generate a lossless Rust schema from the MySQL catalog: {error}"
        ))
    })?;
    let snapshot = IntrospectionResult { ddl: ddl.clone() }.to_snapshot();

    Ok(super::IntrospectResult {
        schema_code: generated.code,
        table_count: ddl.tables.list().len(),
        index_count: ddl.indexes.list().len(),
        view_count: ddl.views.list().len(),
        warnings: generated.warnings,
        mysql_catalog_defaults: Some(catalog_defaults),
        snapshot: Snapshot::MySQL(snapshot),
        snapshot_path: std::path::PathBuf::new(),
    })
}

//! The error type shared by every Drizzle crate and driver.

use crate::prelude::{Box, String, Vec, format};
use compact_str::CompactString;
use thiserror::Error;

const MAX_CONTEXT_PARAMS: usize = 32;
const MAX_CONTEXT_PARAM_CHARS: usize = 128;

/// The SQL and parameters of a failed query.
///
/// Attach it with [`ResultExt::with_query`]. The error message and the
/// `Debug` output show the SQL and how many parameters were bound, never
/// their values: they often hold personal data or secrets, and errors end up
/// in logs (`.unwrap()` prints `Debug`). The values stay available in
/// [`params`](Self::params).
#[derive(Clone)]
pub struct QueryContext {
    /// Rendered SQL statement.
    pub sql: CompactString,
    /// Debug-rendered parameter values, truncated to keep errors bounded.
    pub params: Box<[CompactString]>,
    /// Total number of parameters, including any omitted from `params`.
    pub param_count: usize,
}

impl QueryContext {
    /// Captures `sql` and the `Debug` form of each parameter.
    ///
    /// At most 32 parameters are kept, each cut to 128 characters, so the
    /// error stays small.
    pub fn new<V: core::fmt::Debug>(sql: &str, params: &[&V]) -> Self {
        let rendered = params
            .iter()
            .take(MAX_CONTEXT_PARAMS)
            .map(|param| truncate_param(format!("{param:?}")))
            .collect::<Vec<_>>()
            .into_boxed_slice();

        Self {
            sql: sql.into(),
            params: rendered,
            param_count: params.len(),
        }
    }
}

impl core::fmt::Debug for QueryContext {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("QueryContext")
            .field("sql", &self.sql)
            .field("param_count", &self.param_count)
            .finish_non_exhaustive()
    }
}

fn truncate_param(mut value: String) -> CompactString {
    if value.chars().count() <= MAX_CONTEXT_PARAM_CHARS {
        return value.into();
    }

    let mut truncated = String::new();
    for ch in value.drain(..).take(MAX_CONTEXT_PARAM_CHARS) {
        truncated.push(ch);
    }
    truncated.push_str("...");
    truncated.into()
}

/// Every error a Drizzle query, conversion or migration can return.
///
/// Driver errors are wrapped in a variant for that driver when its feature
/// is enabled. [`DrizzleError::QueryFailed`] adds the failing SQL and
/// parameters to another error.
#[derive(Debug, Error)]
pub enum DrizzleError {
    /// The statement failed to execute.
    #[error("Execution error: {0}")]
    ExecutionError(compact_str::CompactString),

    /// The statement failed to prepare.
    #[error("Prepare error: {0}")]
    PrepareError(compact_str::CompactString),

    /// A query that returns exactly one row, such as an aggregate with no
    /// GROUP BY, returned none.
    #[error("No rows found")]
    NotFound,

    /// A transaction could not begin, commit or roll back.
    #[error("Transaction error: {0}")]
    TransactionError(compact_str::CompactString),

    /// A row could not be mapped to the target type.
    #[error("Mapping error: {0}")]
    Mapping(compact_str::CompactString),

    /// The statement is invalid.
    #[error("Statement error: {0}")]
    Statement(compact_str::CompactString),

    /// The query is invalid.
    #[error("Query error: {0}")]
    Query(CompactString),

    /// Another error, with the SQL and parameters of the query that caused
    /// it. The message shows the parameter count, not the values; see
    /// [`QueryContext`].
    #[error("{source}\n  sql: {sql}\n  params: {count} bound", sql = .ctx.sql, count = .ctx.param_count)]
    QueryFailed {
        /// Captured SQL and parameter context.
        ctx: Box<QueryContext>,
        /// Original error.
        #[source]
        source: Box<DrizzleError>,
    },

    /// Parameters could not be bound: missing, duplicate or unexpected
    /// bindings, or a value that could not be converted.
    #[error("Parameter conversion error: {0}")]
    ParameterError(compact_str::CompactString),

    /// An integer did not fit the target type.
    #[error("Integer conversion error: {0}")]
    TryFromInt(#[from] core::num::TryFromIntError),

    /// Text could not be parsed as an integer.
    #[error("Parse int error: {0}")]
    ParseInt(#[from] core::num::ParseIntError),

    /// Text could not be parsed as a float.
    #[error("Parse float error: {0}")]
    ParseFloat(#[from] core::num::ParseFloatError),

    /// Text could not be parsed as a boolean.
    #[error("Parse bool error: {0}")]
    ParseBool(#[from] core::str::ParseBoolError),

    /// A value could not be converted to the requested type.
    #[error("Type conversion error: {0}")]
    ConversionError(compact_str::CompactString),

    /// The schema is invalid (for example a cycle in table dependencies).
    #[error("Schema error: {0}")]
    Schema(compact_str::CompactString),

    /// A dirty migration cannot be repaired without violating its execution
    /// safety contract.
    #[error("Migration `{tag}` cannot be repaired safely: {reason}")]
    UnsafeMigrationRepair {
        /// Migration tag recorded in the tracking table.
        tag: CompactString,
        /// Safety requirement that prevents automatic repair.
        reason: CompactString,
    },

    /// The selected adapter cannot provide a migration's required execution
    /// semantics.
    #[error("{adapter} cannot execute this migration: {requirement}")]
    UnsupportedMigrationExecution {
        /// Runtime adapter that cannot provide the required semantics.
        adapter: CompactString,
        /// Migration execution capability the adapter does not provide.
        requirement: CompactString,
    },

    /// Any other error.
    #[error("Database error: {0}")]
    Other(compact_str::CompactString),

    /// Error returned by a wire driver whose concrete type is intentionally
    /// kept out of drizzle-core's public dependency graph.
    #[cfg(feature = "driver-error")]
    #[error("{driver} error: {source}")]
    Driver {
        /// Stable adapter name used in diagnostics.
        driver: CompactString,
        /// Original driver error, retained as the error source.
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
        /// What kind of failure the driver reported; see
        /// [`DrizzleError::kind`].
        kind: ErrorKind,
        /// The constraint the failure names, if any; see
        /// [`DrizzleError::constraint`].
        constraint: Option<CompactString>,
    },

    /// Error from a higher-level subsystem kept as a typed source.
    #[cfg(feature = "driver-error")]
    #[error("{context}: {source}")]
    External {
        /// Operation or subsystem that failed.
        context: CompactString,
        /// Original error, retained as the error source.
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    /// An error from `rusqlite`.
    #[cfg(feature = "rusqlite")]
    #[error("Rusqlite error: {0}")]
    Rusqlite(#[from] rusqlite::Error),

    /// An error from `turso`.
    #[cfg(feature = "turso")]
    #[error("Turso error: {0}")]
    Turso(#[from] turso::Error),

    /// An error from `libsql`.
    #[cfg(feature = "libsql")]
    #[error("LibSQL error: {0}")]
    LibSQL(#[from] libsql::Error),

    /// An error from `tokio-postgres`. The message is the server's, with its
    /// SQLSTATE; the full `DbError` (detail, constraint, ...) is the source.
    #[cfg(feature = "tokio-postgres")]
    #[error("Postgres error: {}", postgres_message(.0))]
    Postgres(#[from] tokio_postgres::Error),

    /// An error from `postgres`. The message is the server's, with its
    /// SQLSTATE; the full `DbError` (detail, constraint, ...) is the source.
    #[cfg(all(feature = "postgres-sync", not(feature = "tokio-postgres")))]
    #[error("Postgres error: {}", postgres_message(.0))]
    Postgres(#[from] postgres::Error),

    /// A UUID could not be parsed.
    #[cfg(feature = "uuid")]
    #[error("UUID error: {0}")]
    UuidError(#[from] uuid::Error),

    /// A JSON value could not be serialized or deserialized.
    #[cfg(feature = "serde")]
    #[error("JSON error: {0}")]
    JsonError(#[from] serde_json::Error),

    /// Never constructed; lets `?` work on infallible conversions.
    #[error("Infallible conversion error")]
    Infallible(#[from] core::convert::Infallible),
}

impl DrizzleError {
    /// Wraps a concrete wire-driver error without exposing its type in the
    /// public error enum.
    #[cfg(feature = "driver-error")]
    pub fn driver(
        driver: impl Into<CompactString>,
        source: impl std::error::Error + Send + Sync + 'static,
    ) -> Self {
        Self::Driver {
            driver: driver.into(),
            source: Box::new(source),
            kind: ErrorKind::Other,
            constraint: None,
        }
    }

    /// Records the [`ErrorKind`] and constraint of a [`Driver`](Self::Driver)
    /// error, for adapters that read them from the driver's error. Other
    /// errors are returned unchanged.
    #[cfg(feature = "driver-error")]
    #[doc(hidden)]
    #[must_use]
    pub fn classified(mut self, kind: ErrorKind, constraint: Option<&str>) -> Self {
        if let Self::Driver {
            kind: stored,
            constraint: stored_constraint,
            ..
        } = &mut self
        {
            *stored = kind;
            *stored_constraint = constraint.map(Into::into);
        }
        self
    }

    /// Wraps an external subsystem error while preserving its source chain.
    #[cfg(feature = "driver-error")]
    pub fn external(
        context: impl Into<CompactString>,
        source: impl std::error::Error + Send + Sync + 'static,
    ) -> Self {
        Self::External {
            context: context.into(),
            source: Box::new(source),
        }
    }
}

/// What kind of failure an error is, named the same on every driver.
///
/// [`DrizzleError::kind`] reads it from the driver's own error: the
/// SQLSTATE on PostgreSQL, the extended result code (or SQLite's message,
/// where the driver reports only text) on SQLite, libsql, turso, D1 and
/// Durable Objects, and the server error number on MySQL.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ErrorKind {
    /// A `UNIQUE` or `PRIMARY KEY` constraint rejected a duplicate value.
    UniqueViolation,
    /// A `FOREIGN KEY` constraint rejected a row: its parent is missing, or
    /// a child still points at the row being removed.
    ForeignKeyViolation,
    /// A `NOT NULL` column got no value.
    NotNullViolation,
    /// A `CHECK` constraint rejected a row.
    CheckViolation,
    /// A serializable transaction conflicted with a concurrent one. Run the
    /// transaction again.
    SerializationFailure,
    /// The database broke a deadlock by failing this statement. Run the
    /// transaction again.
    Deadlock,
    /// The database or a row is locked by another connection, and waiting
    /// for it failed (SQLite `SQLITE_BUSY`, a lock timeout). Try again.
    Busy,
    /// A query that returns exactly one row returned none.
    NotFound,
    /// Any other failure.
    Other,
}

impl ErrorKind {
    /// Whether a constraint rejected the statement.
    #[must_use]
    pub const fn is_constraint_violation(self) -> bool {
        matches!(
            self,
            Self::UniqueViolation
                | Self::ForeignKeyViolation
                | Self::NotNullViolation
                | Self::CheckViolation
        )
    }

    /// Whether running the statement or transaction again can succeed:
    /// [`SerializationFailure`](Self::SerializationFailure),
    /// [`Deadlock`](Self::Deadlock) and [`Busy`](Self::Busy).
    #[must_use]
    pub const fn is_retryable(self) -> bool {
        matches!(
            self,
            Self::SerializationFailure | Self::Deadlock | Self::Busy
        )
    }
}

impl DrizzleError {
    /// What kind of failure this is, the same on every driver.
    ///
    /// It looks through [`QueryFailed`](Self::QueryFailed) to the driver's
    /// error.
    ///
    /// # Examples
    ///
    /// ```
    /// use drizzle_core::error::{DrizzleError, ErrorKind};
    ///
    /// let error = DrizzleError::Other("UNIQUE constraint failed: users.email".into());
    /// assert_eq!(error.kind(), ErrorKind::UniqueViolation);
    /// assert!(!error.is_retryable());
    /// ```
    #[must_use]
    pub fn kind(&self) -> ErrorKind {
        match self {
            Self::QueryFailed { source, .. } => source.kind(),
            Self::NotFound => ErrorKind::NotFound,
            Self::Other(message) | Self::ExecutionError(message) => message_kind(message),
            #[cfg(feature = "driver-error")]
            Self::Driver { kind, .. } => *kind,
            #[cfg(feature = "rusqlite")]
            Self::Rusqlite(error) => rusqlite_kind(error),
            #[cfg(feature = "turso")]
            Self::Turso(error) => turso_kind(error),
            #[cfg(feature = "libsql")]
            Self::LibSQL(libsql::Error::SqliteFailure(code, message)) => {
                sqlite_code_kind(*code).unwrap_or_else(|| sqlite_message_kind(message))
            }
            #[cfg(any(feature = "tokio-postgres", feature = "postgres-sync"))]
            Self::Postgres(error) => error
                .as_db_error()
                .map_or(ErrorKind::Other, |db| sqlstate_kind(db.code().code())),
            _ => ErrorKind::Other,
        }
    }

    /// The name of the constraint the failure names, when the database
    /// reports one: the constraint on PostgreSQL and MySQL, a named `CHECK`
    /// on SQLite (SQLite names only the columns of a `UNIQUE` failure).
    ///
    /// It looks through [`QueryFailed`](Self::QueryFailed).
    #[must_use]
    pub fn constraint(&self) -> Option<&str> {
        match self {
            Self::QueryFailed { source, .. } => source.constraint(),
            Self::Other(message) | Self::ExecutionError(message) => message_constraint(message),
            #[cfg(feature = "driver-error")]
            Self::Driver { constraint, .. } => constraint.as_deref(),
            #[cfg(feature = "rusqlite")]
            Self::Rusqlite(rusqlite::Error::SqliteFailure(_, Some(message))) => {
                sqlite_check_name(message)
            }
            #[cfg(feature = "turso")]
            Self::Turso(turso::Error::Constraint(message)) => sqlite_check_name(message),
            #[cfg(feature = "libsql")]
            Self::LibSQL(libsql::Error::SqliteFailure(_, message)) => sqlite_check_name(message),
            #[cfg(any(feature = "tokio-postgres", feature = "postgres-sync"))]
            Self::Postgres(error) => error.as_db_error().and_then(|db| db.constraint()),
            _ => None,
        }
    }

    /// Whether running the statement or transaction again can succeed; see
    /// [`ErrorKind::is_retryable`].
    #[must_use]
    pub fn is_retryable(&self) -> bool {
        self.kind().is_retryable()
    }

    /// The SQL and parameters of the query that failed, when drizzle ran it.
    #[must_use]
    pub fn query(&self) -> Option<&QueryContext> {
        match self {
            Self::QueryFailed { ctx, .. } => Some(ctx),
            _ => None,
        }
    }
}

/// SQLite primary result codes, and the extended codes of constraint
/// failures. An extended code keeps its primary code in its low byte.
const SQLITE_BUSY: i32 = 5;
const SQLITE_LOCKED: i32 = 6;
const SQLITE_CONSTRAINT_CHECK: i32 = 275;
const SQLITE_CONSTRAINT_FOREIGNKEY: i32 = 787;
const SQLITE_CONSTRAINT_NOTNULL: i32 = 1299;
const SQLITE_CONSTRAINT_PRIMARYKEY: i32 = 1555;
const SQLITE_CONSTRAINT_UNIQUE: i32 = 2067;

/// The kind of an SQLite (extended) result code, or `None` when the code
/// alone does not say.
#[cfg_attr(not(any(feature = "rusqlite", feature = "libsql")), allow(dead_code))]
const fn sqlite_code_kind(code: i32) -> Option<ErrorKind> {
    match code {
        SQLITE_CONSTRAINT_UNIQUE | SQLITE_CONSTRAINT_PRIMARYKEY => Some(ErrorKind::UniqueViolation),
        SQLITE_CONSTRAINT_FOREIGNKEY => Some(ErrorKind::ForeignKeyViolation),
        SQLITE_CONSTRAINT_NOTNULL => Some(ErrorKind::NotNullViolation),
        SQLITE_CONSTRAINT_CHECK => Some(ErrorKind::CheckViolation),
        _ if matches!(code & 0xff, SQLITE_BUSY | SQLITE_LOCKED) => Some(ErrorKind::Busy),
        _ => None,
    }
}

#[cfg(feature = "rusqlite")]
fn rusqlite_kind(error: &rusqlite::Error) -> ErrorKind {
    match error {
        rusqlite::Error::SqliteFailure(failure, message) => sqlite_code_kind(failure.extended_code)
            .unwrap_or_else(|| {
                message
                    .as_deref()
                    .map_or(ErrorKind::Other, sqlite_message_kind)
            }),
        _ => ErrorKind::Other,
    }
}

#[cfg(feature = "turso")]
fn turso_kind(error: &turso::Error) -> ErrorKind {
    match error {
        turso::Error::Constraint(message) | turso::Error::Error(message) => {
            sqlite_message_kind(message)
        }
        turso::Error::Busy(_) | turso::Error::BusySnapshot(_) => ErrorKind::Busy,
        _ => ErrorKind::Other,
    }
}

/// The kind named by an error message: SQLite's constraint and lock text
/// (D1, Durable Objects and turso report only text), else a PostgreSQL
/// SQLSTATE written as `SQLState: 23505` (the AWS Data API).
fn message_kind(message: &str) -> ErrorKind {
    match sqlite_message_kind(message) {
        ErrorKind::Other => message
            .split_once("SQLState: ")
            .and_then(|(_, rest)| rest.get(..5))
            .map_or(ErrorKind::Other, sqlstate_kind),
        kind => kind,
    }
}

/// The constraint an error message names: a named SQLite `CHECK`, or a
/// PostgreSQL message's `constraint "name"`.
fn message_constraint(message: &str) -> Option<&str> {
    sqlite_check_name(message).or_else(|| {
        message
            .split_once("constraint \"")
            .and_then(|(_, rest)| rest.split_once('"'))
            .map(|(name, _)| name)
    })
}

/// The kind named by SQLite's error message text, which is stable across
/// SQLite versions and drivers.
fn sqlite_message_kind(message: &str) -> ErrorKind {
    if message.contains("UNIQUE constraint failed") {
        ErrorKind::UniqueViolation
    } else if message.contains("FOREIGN KEY constraint failed") {
        ErrorKind::ForeignKeyViolation
    } else if message.contains("NOT NULL constraint failed") {
        ErrorKind::NotNullViolation
    } else if message.contains("CHECK constraint failed") {
        ErrorKind::CheckViolation
    } else if message.contains("database is locked")
        || message.contains("database table is locked")
        || message.contains("SQLITE_BUSY")
    {
        ErrorKind::Busy
    } else {
        ErrorKind::Other
    }
}

/// The constraint named by SQLite's `CHECK constraint failed: <name>`.
fn sqlite_check_name(message: &str) -> Option<&str> {
    let (_, rest) = message.split_once("CHECK constraint failed: ")?;
    let name = rest
        .split([':', '\n'])
        .next()
        .unwrap_or(rest)
        .trim();
    (!name.is_empty()).then_some(name)
}

/// The kind of a PostgreSQL SQLSTATE.
fn sqlstate_kind(code: &str) -> ErrorKind {
    match code {
        "23505" => ErrorKind::UniqueViolation,
        "23503" => ErrorKind::ForeignKeyViolation,
        "23502" => ErrorKind::NotNullViolation,
        "23514" => ErrorKind::CheckViolation,
        "40001" => ErrorKind::SerializationFailure,
        "40P01" => ErrorKind::Deadlock,
        "55P03" => ErrorKind::Busy,
        _ => ErrorKind::Other,
    }
}

/// The server's message and SQLSTATE for a Postgres error. The driver's own
/// `Display` is only "db error". `DbError`'s `Display` adds the DETAIL line,
/// which for a constraint violation holds the row's values
/// (`Key (email)=(...) already exists.`), so it is left out.
#[cfg(feature = "tokio-postgres")]
fn postgres_message(error: &tokio_postgres::Error) -> String {
    error.as_db_error().map_or_else(
        || format!("{error}"),
        |db| format!("{} ({})", db.message(), db.code().code()),
    )
}

#[cfg(all(feature = "postgres-sync", not(feature = "tokio-postgres")))]
fn postgres_message(error: &postgres::Error) -> String {
    error.as_db_error().map_or_else(
        || format!("{error}"),
        |db| format!("{} ({})", db.message(), db.code().code()),
    )
}

/// `Result` with [`DrizzleError`] as the error type.
pub type Result<T> = core::result::Result<T, DrizzleError>;

/// Adds the failing query's SQL and parameters to an error.
///
/// # Examples
///
/// ```
/// use drizzle_core::error::{DrizzleError, QueryContext, ResultExt};
///
/// let result: Result<(), DrizzleError> = Err(DrizzleError::NotFound);
/// let error = result
///     .with_query(|| QueryContext::new("SELECT * FROM users WHERE id = ?", &[&7]))
///     .unwrap_err();
///
/// assert_eq!(
///     error.to_string(),
///     "No rows found\n  sql: SELECT * FROM users WHERE id = ?\n  params: 1 bound",
/// );
/// ```
pub trait ResultExt<T> {
    /// On error, wraps it in [`DrizzleError::QueryFailed`] with the context
    /// from `ctx`. `ctx` only runs on the error path, and an error that
    /// already has context keeps it.
    fn with_query<F>(self, ctx: F) -> Result<T>
    where
        F: FnOnce() -> QueryContext;
}

impl<T, E> ResultExt<T> for core::result::Result<T, E>
where
    E: Into<DrizzleError>,
{
    fn with_query<F>(self, ctx: F) -> Result<T>
    where
        F: FnOnce() -> QueryContext,
    {
        self.map_err(|error| {
            let source = error.into();
            match source {
                DrizzleError::QueryFailed { .. } => source,
                other => DrizzleError::QueryFailed {
                    ctx: Box::new(ctx()),
                    source: Box::new(other),
                },
            }
        })
    }
}

#[cfg(test)]
mod display_tests {
    use super::*;
    use crate::prelude::ToString;

    // Parameter values are often personal data or secrets; the message that
    // reaches logs shows only how many were bound.
    #[test]
    fn query_failed_display_omits_parameter_values() {
        let error = Err::<(), _>(DrizzleError::NotFound)
            .with_query(|| {
                QueryContext::new(
                    "SELECT * FROM users WHERE email = $1",
                    &[&"ada@example.com"],
                )
            })
            .unwrap_err();

        let message = error.to_string();
        assert_eq!(
            message,
            "No rows found\n  sql: SELECT * FROM users WHERE email = $1\n  params: 1 bound"
        );
        // `.unwrap()` prints `Debug`.
        let debug = format!("{error:?}");
        assert!(!debug.contains("ada@example.com"), "{debug}");
        let DrizzleError::QueryFailed { ctx, .. } = error else {
            panic!("expected QueryFailed");
        };
        assert_eq!(ctx.params.as_ref(), ["\"ada@example.com\""]);
    }
}

#[cfg(all(test, feature = "driver-error"))]
mod tests {
    use super::*;
    use std::error::Error as _;

    #[test]
    fn external_errors_keep_their_source() {
        let error =
            DrizzleError::external("schema diff", std::io::Error::other("invalid snapshot"));

        assert_eq!(error.to_string(), "schema diff: invalid snapshot");
        assert_eq!(
            error.source().map(ToString::to_string),
            Some("invalid snapshot".to_string())
        );
    }
}

#[cfg(test)]
mod kind_tests {
    use super::*;

    #[test]
    fn sqlite_messages_name_their_kind() {
        // D1 and Durable Objects report SQLite's text with their own prefix
        // and suffix.
        let cases = [
            (
                "D1_ERROR: UNIQUE constraint failed: users.email: SQLITE_CONSTRAINT",
                ErrorKind::UniqueViolation,
            ),
            (
                "FOREIGN KEY constraint failed: SQLITE_CONSTRAINT",
                ErrorKind::ForeignKeyViolation,
            ),
            (
                "NOT NULL constraint failed: users.name",
                ErrorKind::NotNullViolation,
            ),
            (
                "CHECK constraint failed: users_age_check",
                ErrorKind::CheckViolation,
            ),
            ("database is locked", ErrorKind::Busy),
            ("no such table: users", ErrorKind::Other),
        ];
        for (message, kind) in cases {
            assert_eq!(
                DrizzleError::Other(message.into()).kind(),
                kind,
                "{message}"
            );
        }
    }

    #[test]
    fn a_named_sqlite_check_is_the_constraint() {
        let error = DrizzleError::Other(
            "D1_ERROR: CHECK constraint failed: positive_age: SQLITE_CONSTRAINT".into(),
        );
        assert_eq!(error.constraint(), Some("positive_age"));
        // SQLite names only the columns of a UNIQUE failure.
        let error = DrizzleError::Other("UNIQUE constraint failed: users.email".into());
        assert_eq!(error.constraint(), None);
    }

    #[test]
    fn aws_data_api_messages_carry_the_sqlstate() {
        let error = DrizzleError::Other(
            "aws ExecuteStatement: ERROR: duplicate key value violates unique constraint \
             \"users_email_key\"; SQLState: 23505"
                .into(),
        );
        assert_eq!(error.kind(), ErrorKind::UniqueViolation);
        assert_eq!(error.constraint(), Some("users_email_key"));

        let serialization =
            DrizzleError::Other("aws ExecuteStatement: ...; SQLState: 40001".into());
        assert_eq!(serialization.kind(), ErrorKind::SerializationFailure);
        assert!(serialization.is_retryable());
        let deadlock = DrizzleError::Other("aws ExecuteStatement: ...; SQLState: 40P01".into());
        assert_eq!(deadlock.kind(), ErrorKind::Deadlock);
        assert!(deadlock.is_retryable());
    }

    #[test]
    fn the_query_context_is_looked_through() {
        let error: Result<()> = Err(DrizzleError::Other(
            "CHECK constraint failed: positive_age".into(),
        ));
        let error = error
            .with_query(|| QueryContext::new("INSERT INTO users VALUES (?)", &[&-1]))
            .unwrap_err();
        assert_eq!(error.kind(), ErrorKind::CheckViolation);
        assert_eq!(error.constraint(), Some("positive_age"));
        assert_eq!(
            error.query().map(|query| query.sql.as_str()),
            Some("INSERT INTO users VALUES (?)")
        );
        assert!(DrizzleError::NotFound.query().is_none());
    }

    #[test]
    fn kinds_say_whether_they_are_constraints_or_retryable() {
        assert_eq!(DrizzleError::NotFound.kind(), ErrorKind::NotFound);
        for kind in [
            ErrorKind::UniqueViolation,
            ErrorKind::ForeignKeyViolation,
            ErrorKind::NotNullViolation,
            ErrorKind::CheckViolation,
        ] {
            assert!(
                kind.is_constraint_violation() && !kind.is_retryable(),
                "{kind:?}"
            );
        }
        for kind in [
            ErrorKind::SerializationFailure,
            ErrorKind::Deadlock,
            ErrorKind::Busy,
        ] {
            assert!(
                kind.is_retryable() && !kind.is_constraint_violation(),
                "{kind:?}"
            );
        }
        for kind in [ErrorKind::NotFound, ErrorKind::Other] {
            assert!(
                !kind.is_retryable() && !kind.is_constraint_violation(),
                "{kind:?}"
            );
        }
    }

    #[cfg(feature = "rusqlite")]
    #[test]
    fn rusqlite_extended_codes_name_their_kind() {
        let failure = |code: i32, message: &str| {
            DrizzleError::Rusqlite(rusqlite::Error::SqliteFailure(
                rusqlite::ffi::Error::new(code),
                Some(message.into()),
            ))
        };
        assert_eq!(
            failure(1555, "UNIQUE constraint failed: users.id").kind(),
            ErrorKind::UniqueViolation
        );
        assert_eq!(
            failure(787, "FOREIGN KEY constraint failed").kind(),
            ErrorKind::ForeignKeyViolation
        );
        let check = failure(275, "CHECK constraint failed: positive_age");
        assert_eq!(check.kind(), ErrorKind::CheckViolation);
        assert_eq!(check.constraint(), Some("positive_age"));
        assert_eq!(failure(5, "database is locked").kind(), ErrorKind::Busy);
        assert_eq!(failure(517, "database is locked").kind(), ErrorKind::Busy);
    }

    #[cfg(feature = "turso")]
    #[test]
    fn turso_errors_name_their_kind() {
        assert_eq!(
            DrizzleError::Turso(turso::Error::Busy("database is locked".into())).kind(),
            ErrorKind::Busy
        );
        assert_eq!(
            DrizzleError::Turso(turso::Error::Constraint(
                "FOREIGN KEY constraint failed".into()
            ))
            .kind(),
            ErrorKind::ForeignKeyViolation
        );
    }

    #[cfg(feature = "libsql")]
    #[test]
    fn libsql_codes_name_their_kind() {
        assert_eq!(
            DrizzleError::LibSQL(libsql::Error::SqliteFailure(
                1299,
                "NOT NULL constraint failed: users.name".into()
            ))
            .kind(),
            ErrorKind::NotNullViolation
        );
    }

    #[cfg(feature = "driver-error")]
    #[test]
    fn driver_errors_keep_the_kind_their_adapter_read() {
        let error = DrizzleError::driver("MySQL", std::io::Error::other("Deadlock found"))
            .classified(ErrorKind::Deadlock, None);
        assert_eq!(error.kind(), ErrorKind::Deadlock);
        assert!(error.is_retryable());

        let error = DrizzleError::driver("MySQL", std::io::Error::other("Duplicate entry"))
            .classified(ErrorKind::UniqueViolation, Some("users_email_unique"));
        assert_eq!(error.constraint(), Some("users_email_unique"));

        let unchanged = DrizzleError::NotFound.classified(ErrorKind::Deadlock, None);
        assert_eq!(unchanged.kind(), ErrorKind::NotFound);
    }
}

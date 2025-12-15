//! Configuration builders with typestate pattern
//!
//! This module provides the `ConfigBuilder` type and driver-specific builder aliases
//! for type-safe configuration construction.

use std::marker::PhantomData;
use std::path::PathBuf;

use crate::schema::Schema;

use super::Config;
use super::credentials::{
    LibsqlCredentials, NoCredentials, PostgresCredentials, SqliteCredentials, TursoCredentials,
};
use super::markers::{
    DialectMarker, MysqlDialect, NoConnection, NoDialect, NoSchema, OutNotSet, OutSet,
    PostgresDialect, SqliteDialect,
};

#[cfg(feature = "rusqlite")]
use super::markers::RusqliteConnection;

#[cfg(feature = "libsql")]
use super::markers::LibsqlConnection;

#[cfg(feature = "turso")]
use super::markers::TursoConnection;

#[cfg(feature = "tokio-postgres")]
use super::markers::TokioPostgresConnection;

#[cfg(feature = "postgres-sync")]
use super::markers::PostgresSyncConnection;

// =============================================================================
// ConfigBuilder
// =============================================================================

/// Builder for creating a `Config` with progressive type refinement.
///
/// Uses typestate pattern to enforce:
/// - `schema()` can only be called once (when S = NoSchema)
/// - `out()` can only be called once (transitions OutNotSet -> OutSet)
/// - `build()` is only available when schema is set
///
/// # Driver-Specific Builders
///
/// For a cleaner API, use the driver-specific builder types which take required
/// credentials as constructor parameters:
///
/// - [`RusqliteConfigBuilder`] - rusqlite (file-based SQLite)
/// - [`LibsqlConfigBuilder`] - libsql (embedded replica SQLite)
/// - [`TursoConfigBuilder`] - turso (edge SQLite)
/// - [`TokioPostgresConfigBuilder`] - tokio-postgres (async PostgreSQL)
/// - [`PostgresSyncConfigBuilder`] - postgres-sync (sync PostgreSQL)
///
/// # Example
///
/// ```ignore
/// use drizzle_migrations::TokioPostgresConfigBuilder;
/// use my_app::schema::AppSchema;
///
/// let config = TokioPostgresConfigBuilder::new("localhost", 5432, "user", "pass", "mydb")
///     .schema::<AppSchema>()
///     .out("./drizzle")
///     .build();
/// ```
#[derive(Clone, Debug)]
pub struct ConfigBuilder<S, D, C, Creds, Out = OutNotSet> {
    pub(crate) out: PathBuf,
    pub(crate) breakpoints: bool,
    pub(crate) credentials: Creds,
    pub(crate) _schema: PhantomData<S>,
    pub(crate) _dialect: PhantomData<D>,
    pub(crate) _connection: PhantomData<C>,
    pub(crate) _out: PhantomData<Out>,
}

/// Initial state: no schema, no dialect, no connection, output not set
impl ConfigBuilder<NoSchema, NoDialect, NoConnection, NoCredentials, OutNotSet> {
    /// Create a new builder (prefer driver-specific builders for a cleaner API)
    pub fn new() -> Self {
        Self {
            out: PathBuf::from("./drizzle"),
            breakpoints: true,
            credentials: NoCredentials,
            _schema: PhantomData,
            _dialect: PhantomData,
            _connection: PhantomData,
            _out: PhantomData,
        }
    }
}

impl Default for ConfigBuilder<NoSchema, NoDialect, NoConnection, NoCredentials, OutNotSet> {
    fn default() -> Self {
        Self::new()
    }
}

// Static entry point
impl Config<NoSchema, NoDialect, NoConnection, NoCredentials> {
    /// Create a new config builder (prefer driver-specific builders for a cleaner API)
    pub fn builder() -> ConfigBuilder<NoSchema, NoDialect, NoConnection, NoCredentials, OutNotSet> {
        <ConfigBuilder<NoSchema, NoDialect, NoConnection, NoCredentials, OutNotSet>>::new()
    }
}

/// Set schema type - only available when S = NoSchema (not yet set)
impl<D, C, Creds, Out> ConfigBuilder<NoSchema, D, C, Creds, Out> {
    /// Set the schema type
    pub fn schema<S: Schema + Default>(self) -> ConfigBuilder<S, D, C, Creds, Out> {
        ConfigBuilder {
            out: self.out,
            breakpoints: self.breakpoints,
            credentials: self.credentials,
            _schema: PhantomData,
            _dialect: PhantomData,
            _connection: PhantomData,
            _out: PhantomData,
        }
    }
}

/// Set dialect - only available when D = NoDialect (not yet set)
impl<S, C, Creds, Out> ConfigBuilder<S, NoDialect, C, Creds, Out> {
    /// Use SQLite dialect
    pub fn sqlite(self) -> ConfigBuilder<S, SqliteDialect, C, Creds, Out> {
        ConfigBuilder {
            out: self.out,
            breakpoints: self.breakpoints,
            credentials: self.credentials,
            _schema: PhantomData,
            _dialect: PhantomData,
            _connection: PhantomData,
            _out: PhantomData,
        }
    }

    /// Use PostgreSQL dialect
    pub fn postgres(self) -> ConfigBuilder<S, PostgresDialect, C, Creds, Out> {
        ConfigBuilder {
            out: self.out,
            breakpoints: self.breakpoints,
            credentials: self.credentials,
            _schema: PhantomData,
            _dialect: PhantomData,
            _connection: PhantomData,
            _out: PhantomData,
        }
    }

    /// Use MySQL dialect  
    pub fn mysql(self) -> ConfigBuilder<S, MysqlDialect, C, Creds, Out> {
        ConfigBuilder {
            out: self.out,
            breakpoints: self.breakpoints,
            credentials: self.credentials,
            _schema: PhantomData,
            _dialect: PhantomData,
            _connection: PhantomData,
            _out: PhantomData,
        }
    }
}

// =============================================================================
// Connection Type Setters (with driver-specific credentials)
// Only available when C = NoConnection (not yet set)
// =============================================================================

// Rusqlite: takes path directly
#[cfg(feature = "rusqlite")]
impl<S, Creds, Out> ConfigBuilder<S, SqliteDialect, NoConnection, Creds, Out> {
    /// Use rusqlite as the connection driver with a database path
    pub fn rusqlite(
        self,
        path: impl Into<String>,
    ) -> ConfigBuilder<S, SqliteDialect, RusqliteConnection, SqliteCredentials, Out> {
        ConfigBuilder {
            out: self.out,
            breakpoints: self.breakpoints,
            credentials: SqliteCredentials::new(path),
            _schema: PhantomData,
            _dialect: PhantomData,
            _connection: PhantomData,
            _out: PhantomData,
        }
    }

    /// Use rusqlite with an in-memory database
    pub fn rusqlite_in_memory(
        self,
    ) -> ConfigBuilder<S, SqliteDialect, RusqliteConnection, SqliteCredentials, Out> {
        ConfigBuilder {
            out: self.out,
            breakpoints: self.breakpoints,
            credentials: SqliteCredentials::in_memory(),
            _schema: PhantomData,
            _dialect: PhantomData,
            _connection: PhantomData,
            _out: PhantomData,
        }
    }
}

// LibSQL: takes path and optional sync config
#[cfg(feature = "libsql")]
impl<S, Creds, Out> ConfigBuilder<S, SqliteDialect, NoConnection, Creds, Out> {
    /// Use libsql with a local database file
    pub fn libsql_local(
        self,
        path: impl Into<String>,
    ) -> ConfigBuilder<S, SqliteDialect, LibsqlConnection, LibsqlCredentials, Out> {
        ConfigBuilder {
            out: self.out,
            breakpoints: self.breakpoints,
            credentials: LibsqlCredentials::local(path),
            _schema: PhantomData,
            _dialect: PhantomData,
            _connection: PhantomData,
            _out: PhantomData,
        }
    }

    /// Use libsql with embedded replica (local + sync to remote)
    pub fn libsql_sync(
        self,
        path: impl Into<String>,
        sync_url: impl Into<String>,
        auth_token: impl Into<String>,
    ) -> ConfigBuilder<S, SqliteDialect, LibsqlConnection, LibsqlCredentials, Out> {
        ConfigBuilder {
            out: self.out,
            breakpoints: self.breakpoints,
            credentials: LibsqlCredentials::with_sync(path, sync_url, auth_token),
            _schema: PhantomData,
            _dialect: PhantomData,
            _connection: PhantomData,
            _out: PhantomData,
        }
    }
}

// Turso: takes URL and auth token
#[cfg(feature = "turso")]
impl<S, Creds, Out> ConfigBuilder<S, SqliteDialect, NoConnection, Creds, Out> {
    /// Use turso as the connection driver
    pub fn turso(
        self,
        url: impl Into<String>,
        auth_token: impl Into<String>,
    ) -> ConfigBuilder<S, SqliteDialect, TursoConnection, TursoCredentials, Out> {
        ConfigBuilder {
            out: self.out,
            breakpoints: self.breakpoints,
            credentials: TursoCredentials::new(url, auth_token),
            _schema: PhantomData,
            _dialect: PhantomData,
            _connection: PhantomData,
            _out: PhantomData,
        }
    }
}

// Tokio-postgres: takes credentials struct
#[cfg(feature = "tokio-postgres")]
impl<S, Creds, Out> ConfigBuilder<S, PostgresDialect, NoConnection, Creds, Out> {
    /// Use tokio-postgres with full credentials
    pub fn tokio_postgres(
        self,
        host: impl Into<String>,
        port: u16,
        username: impl Into<String>,
        password: impl Into<String>,
        database: impl Into<String>,
    ) -> ConfigBuilder<S, PostgresDialect, TokioPostgresConnection, PostgresCredentials, Out> {
        ConfigBuilder {
            out: self.out,
            breakpoints: self.breakpoints,
            credentials: PostgresCredentials::new(host, port, username, password, database),
            _schema: PhantomData,
            _dialect: PhantomData,
            _connection: PhantomData,
            _out: PhantomData,
        }
    }

    /// Use tokio-postgres with credentials from a connection URL
    pub fn tokio_postgres_url(
        self,
        url: impl Into<String>,
    ) -> ConfigBuilder<S, PostgresDialect, TokioPostgresConnection, PostgresCredentials, Out> {
        ConfigBuilder {
            out: self.out,
            breakpoints: self.breakpoints,
            credentials: PostgresCredentials::from_url(url),
            _schema: PhantomData,
            _dialect: PhantomData,
            _connection: PhantomData,
            _out: PhantomData,
        }
    }
}

// Sync postgres: takes credentials struct
#[cfg(feature = "postgres-sync")]
impl<S, Creds, Out> ConfigBuilder<S, PostgresDialect, NoConnection, Creds, Out> {
    /// Use sync postgres with full credentials
    pub fn postgres_sync(
        self,
        host: impl Into<String>,
        port: u16,
        username: impl Into<String>,
        password: impl Into<String>,
        database: impl Into<String>,
    ) -> ConfigBuilder<S, PostgresDialect, PostgresSyncConnection, PostgresCredentials, Out> {
        ConfigBuilder {
            out: self.out,
            breakpoints: self.breakpoints,
            credentials: PostgresCredentials::new(host, port, username, password, database),
            _schema: PhantomData,
            _dialect: PhantomData,
            _connection: PhantomData,
            _out: PhantomData,
        }
    }

    /// Use sync postgres with credentials from a connection URL
    pub fn postgres_sync_url(
        self,
        url: impl Into<String>,
    ) -> ConfigBuilder<S, PostgresDialect, PostgresSyncConnection, PostgresCredentials, Out> {
        ConfigBuilder {
            out: self.out,
            breakpoints: self.breakpoints,
            credentials: PostgresCredentials::from_url(url),
            _schema: PhantomData,
            _dialect: PhantomData,
            _connection: PhantomData,
            _out: PhantomData,
        }
    }
}

// =============================================================================
// Output Directory Setter - only available when Out = OutNotSet
// =============================================================================

impl<S, D, C, Creds> ConfigBuilder<S, D, C, Creds, OutNotSet> {
    /// Set the output directory for migrations (can only be called once)
    pub fn out(self, path: impl Into<PathBuf>) -> ConfigBuilder<S, D, C, Creds, OutSet> {
        ConfigBuilder {
            out: path.into(),
            breakpoints: self.breakpoints,
            credentials: self.credentials,
            _schema: PhantomData,
            _dialect: PhantomData,
            _connection: PhantomData,
            _out: PhantomData,
        }
    }
}

// =============================================================================
// Breakpoints Setter - available on any builder state
// =============================================================================

impl<S, D, C, Creds, Out> ConfigBuilder<S, D, C, Creds, Out> {
    /// Enable or disable SQL statement breakpoints
    pub fn breakpoints(mut self, enabled: bool) -> Self {
        self.breakpoints = enabled;
        self
    }
}

// =============================================================================
// Build Methods - only available when schema AND dialect are set
// =============================================================================

/// Build for CLI-only use (no credentials needed) - requires schema and dialect set
impl<S: Schema + Default, D: DialectMarker, C, Out> ConfigBuilder<S, D, C, NoCredentials, Out> {
    /// Build the config for CLI-only use (no database connection)
    pub fn build(self) -> Config<S, D, C, NoCredentials> {
        Config {
            out: self.out,
            breakpoints: self.breakpoints,
            credentials: NoCredentials,
            schema: S::default(),
            _dialect: PhantomData,
            _connection: PhantomData,
        }
    }
}

/// Build with credentials - requires schema, dialect, and credentials set
impl<S: Schema + Default, D: DialectMarker, C, Out> ConfigBuilder<S, D, C, SqliteCredentials, Out> {
    /// Build the config with SQLite credentials
    pub fn build(self) -> Config<S, D, C, SqliteCredentials> {
        Config {
            out: self.out,
            breakpoints: self.breakpoints,
            credentials: self.credentials,
            schema: S::default(),
            _dialect: PhantomData,
            _connection: PhantomData,
        }
    }
}

impl<S: Schema + Default, D: DialectMarker, C, Out> ConfigBuilder<S, D, C, LibsqlCredentials, Out> {
    /// Build the config with LibSQL credentials
    pub fn build(self) -> Config<S, D, C, LibsqlCredentials> {
        Config {
            out: self.out,
            breakpoints: self.breakpoints,
            credentials: self.credentials,
            schema: S::default(),
            _dialect: PhantomData,
            _connection: PhantomData,
        }
    }
}

impl<S: Schema + Default, D: DialectMarker, C, Out> ConfigBuilder<S, D, C, TursoCredentials, Out> {
    /// Build the config with Turso credentials
    pub fn build(self) -> Config<S, D, C, TursoCredentials> {
        Config {
            out: self.out,
            breakpoints: self.breakpoints,
            credentials: self.credentials,
            schema: S::default(),
            _dialect: PhantomData,
            _connection: PhantomData,
        }
    }
}

impl<S: Schema + Default, D: DialectMarker, C, Out>
    ConfigBuilder<S, D, C, PostgresCredentials, Out>
{
    /// Build the config with PostgreSQL credentials
    pub fn build(self) -> Config<S, D, C, PostgresCredentials> {
        Config {
            out: self.out,
            breakpoints: self.breakpoints,
            credentials: self.credentials,
            schema: S::default(),
            _dialect: PhantomData,
            _connection: PhantomData,
        }
    }
}

/// Build with explicit schema instance (for non-Default schemas)
impl<S: Schema, D: DialectMarker, C, Creds, Out> ConfigBuilder<S, D, C, Creds, Out> {
    /// Build the config with a specific schema instance
    pub fn build_with_schema(self, schema: S) -> Config<S, D, C, Creds> {
        Config {
            out: self.out,
            breakpoints: self.breakpoints,
            credentials: self.credentials,
            schema,
            _dialect: PhantomData,
            _connection: PhantomData,
        }
    }
}

// =============================================================================
// Driver-Specific Config Builders
// These provide a cleaner API where required credentials are passed to new()
// =============================================================================

// -----------------------------------------------------------------------------
// RusqliteConfigBuilder
// -----------------------------------------------------------------------------

/// Builder for rusqlite (file-based SQLite) configurations.
///
/// Takes the database path as a required parameter in `new()`.
///
/// # Example
///
/// ```ignore
/// use drizzle_migrations::RusqliteConfigBuilder;
/// use my_app::schema::AppSchema;
///
/// let config = RusqliteConfigBuilder::new("./dev.db")
///     .schema::<AppSchema>()
///     .out("./drizzle")
///     .build();
/// ```
#[cfg(feature = "rusqlite")]
pub type RusqliteConfigBuilder<S, Out = OutNotSet> =
    ConfigBuilder<S, SqliteDialect, RusqliteConnection, SqliteCredentials, Out>;

#[cfg(feature = "rusqlite")]
impl RusqliteConfigBuilder<NoSchema, OutNotSet> {
    /// Create a new rusqlite config builder with the database path.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the SQLite database file (e.g., "./dev.db")
    pub fn new(path: impl Into<String>) -> Self {
        ConfigBuilder {
            out: PathBuf::from("./drizzle"),
            breakpoints: true,
            credentials: SqliteCredentials::new(path),
            _schema: PhantomData,
            _dialect: PhantomData,
            _connection: PhantomData,
            _out: PhantomData,
        }
    }

    /// Create a new rusqlite config builder with an in-memory database.
    pub fn in_memory() -> Self {
        ConfigBuilder {
            out: PathBuf::from("./drizzle"),
            breakpoints: true,
            credentials: SqliteCredentials::in_memory(),
            _schema: PhantomData,
            _dialect: PhantomData,
            _connection: PhantomData,
            _out: PhantomData,
        }
    }
}

// -----------------------------------------------------------------------------
// LibsqlConfigBuilder
// -----------------------------------------------------------------------------

/// Builder for libsql (embedded replica SQLite) configurations.
///
/// Takes the local database path as a required parameter in `new()`.
/// Use `with_sync()` to create a builder with embedded replica sync.
///
/// # Example
///
/// ```ignore
/// use drizzle_migrations::LibsqlConfigBuilder;
/// use my_app::schema::AppSchema;
///
/// // Local-only
/// let config = LibsqlConfigBuilder::new("./local.db")
///     .schema::<AppSchema>()
///     .build();
///
/// // With embedded replica sync
/// let config = LibsqlConfigBuilder::with_sync("./local.db", "libsql://mydb.turso.io", "auth_token")
///     .schema::<AppSchema>()
///     .build();
/// ```
#[cfg(feature = "libsql")]
pub type LibsqlConfigBuilder<S, Out = OutNotSet> =
    ConfigBuilder<S, SqliteDialect, LibsqlConnection, LibsqlCredentials, Out>;

#[cfg(feature = "libsql")]
impl LibsqlConfigBuilder<NoSchema, OutNotSet> {
    /// Create a new libsql config builder with a local database path.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the local SQLite database file
    pub fn new(path: impl Into<String>) -> Self {
        ConfigBuilder {
            out: PathBuf::from("./drizzle"),
            breakpoints: true,
            credentials: LibsqlCredentials::local(path),
            _schema: PhantomData,
            _dialect: PhantomData,
            _connection: PhantomData,
            _out: PhantomData,
        }
    }

    /// Create a new libsql config builder with embedded replica sync.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the local SQLite database file
    /// * `sync_url` - Remote sync URL (e.g., "libsql://mydb.turso.io")
    /// * `auth_token` - Authentication token for the remote database
    pub fn with_sync(
        path: impl Into<String>,
        sync_url: impl Into<String>,
        auth_token: impl Into<String>,
    ) -> Self {
        ConfigBuilder {
            out: PathBuf::from("./drizzle"),
            breakpoints: true,
            credentials: LibsqlCredentials::with_sync(path, sync_url, auth_token),
            _schema: PhantomData,
            _dialect: PhantomData,
            _connection: PhantomData,
            _out: PhantomData,
        }
    }
}

// -----------------------------------------------------------------------------
// TursoConfigBuilder
// -----------------------------------------------------------------------------

/// Builder for turso (edge SQLite) configurations.
///
/// Takes the database URL and auth token as required parameters in `new()`.
///
/// # Example
///
/// ```ignore
/// use drizzle_migrations::TursoConfigBuilder;
/// use my_app::schema::AppSchema;
///
/// let config = TursoConfigBuilder::new("libsql://mydb-myorg.turso.io", "auth_token")
///     .schema::<AppSchema>()
///     .out("./drizzle")
///     .build();
/// ```
#[cfg(feature = "turso")]
pub type TursoConfigBuilder<S, Out = OutNotSet> =
    ConfigBuilder<S, SqliteDialect, TursoConnection, TursoCredentials, Out>;

#[cfg(feature = "turso")]
impl TursoConfigBuilder<NoSchema, OutNotSet> {
    /// Create a new turso config builder with database URL and auth token.
    ///
    /// # Arguments
    ///
    /// * `url` - Database URL (e.g., "libsql://mydb-myorg.turso.io")
    /// * `auth_token` - Authentication token
    pub fn new(url: impl Into<String>, auth_token: impl Into<String>) -> Self {
        ConfigBuilder {
            out: PathBuf::from("./drizzle"),
            breakpoints: true,
            credentials: TursoCredentials::new(url, auth_token),
            _schema: PhantomData,
            _dialect: PhantomData,
            _connection: PhantomData,
            _out: PhantomData,
        }
    }
}

// -----------------------------------------------------------------------------
// TokioPostgresConfigBuilder
// -----------------------------------------------------------------------------

/// Builder for tokio-postgres (async PostgreSQL) configurations.
///
/// Takes connection parameters as required parameters in `new()`.
/// Use `from_url()` to create from a connection URL.
///
/// # Example
///
/// ```ignore
/// use drizzle_migrations::TokioPostgresConfigBuilder;
/// use my_app::schema::AppSchema;
///
/// let config = TokioPostgresConfigBuilder::new("localhost", 5432, "user", "pass", "mydb")
///     .schema::<AppSchema>()
///     .out("./drizzle")
///     .build();
///
/// // Or from URL
/// let config = TokioPostgresConfigBuilder::from_url("postgres://user:pass@localhost:5432/mydb")
///     .schema::<AppSchema>()
///     .build();
/// ```
#[cfg(feature = "tokio-postgres")]
pub type TokioPostgresConfigBuilder<S, Out = OutNotSet> =
    ConfigBuilder<S, PostgresDialect, TokioPostgresConnection, PostgresCredentials, Out>;

#[cfg(feature = "tokio-postgres")]
impl TokioPostgresConfigBuilder<NoSchema, OutNotSet> {
    /// Create a new tokio-postgres config builder with connection parameters.
    ///
    /// # Arguments
    ///
    /// * `host` - Database host address
    /// * `port` - Database port (typically 5432)
    /// * `username` - Database username
    /// * `password` - Database password
    /// * `database` - Database name
    pub fn new(
        host: impl Into<String>,
        port: u16,
        username: impl Into<String>,
        password: impl Into<String>,
        database: impl Into<String>,
    ) -> Self {
        ConfigBuilder {
            out: PathBuf::from("./drizzle"),
            breakpoints: true,
            credentials: PostgresCredentials::new(host, port, username, password, database),
            _schema: PhantomData,
            _dialect: PhantomData,
            _connection: PhantomData,
            _out: PhantomData,
        }
    }

    /// Create a new tokio-postgres config builder from a connection URL.
    ///
    /// # Arguments
    ///
    /// * `url` - Connection URL (e.g., "postgres://user:pass@localhost:5432/mydb")
    pub fn from_url(url: impl Into<String>) -> Self {
        ConfigBuilder {
            out: PathBuf::from("./drizzle"),
            breakpoints: true,
            credentials: PostgresCredentials::from_url(url),
            _schema: PhantomData,
            _dialect: PhantomData,
            _connection: PhantomData,
            _out: PhantomData,
        }
    }
}

// -----------------------------------------------------------------------------
// PostgresSyncConfigBuilder
// -----------------------------------------------------------------------------

/// Builder for postgres-sync (synchronous PostgreSQL) configurations.
///
/// Takes connection parameters as required parameters in `new()`.
/// Use `from_url()` to create from a connection URL.
///
/// # Example
///
/// ```ignore
/// use drizzle_migrations::PostgresSyncConfigBuilder;
/// use my_app::schema::AppSchema;
///
/// let config = PostgresSyncConfigBuilder::new("localhost", 5432, "user", "pass", "mydb")
///     .schema::<AppSchema>()
///     .out("./drizzle")
///     .build();
///
/// // Or from URL
/// let config = PostgresSyncConfigBuilder::from_url("postgres://user:pass@localhost:5432/mydb")
///     .schema::<AppSchema>()
///     .build();
/// ```
#[cfg(feature = "postgres-sync")]
pub type PostgresSyncConfigBuilder<S, Out = OutNotSet> =
    ConfigBuilder<S, PostgresDialect, PostgresSyncConnection, PostgresCredentials, Out>;

#[cfg(feature = "postgres-sync")]
impl PostgresSyncConfigBuilder<NoSchema, OutNotSet> {
    /// Create a new postgres-sync config builder with connection parameters.
    ///
    /// # Arguments
    ///
    /// * `host` - Database host address
    /// * `port` - Database port (typically 5432)
    /// * `username` - Database username
    /// * `password` - Database password
    /// * `database` - Database name
    pub fn new(
        host: impl Into<String>,
        port: u16,
        username: impl Into<String>,
        password: impl Into<String>,
        database: impl Into<String>,
    ) -> Self {
        ConfigBuilder {
            out: PathBuf::from("./drizzle"),
            breakpoints: true,
            credentials: PostgresCredentials::new(host, port, username, password, database),
            _schema: PhantomData,
            _dialect: PhantomData,
            _connection: PhantomData,
            _out: PhantomData,
        }
    }

    /// Create a new postgres-sync config builder from a connection URL.
    ///
    /// # Arguments
    ///
    /// * `url` - Connection URL (e.g., "postgres://user:pass@localhost:5432/mydb")
    pub fn from_url(url: impl Into<String>) -> Self {
        ConfigBuilder {
            out: PathBuf::from("./drizzle"),
            breakpoints: true,
            credentials: PostgresCredentials::from_url(url),
            _schema: PhantomData,
            _dialect: PhantomData,
            _connection: PhantomData,
            _out: PhantomData,
        }
    }
}

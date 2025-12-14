//! Configuration types for drizzle migrations
//!
//! This module provides Rust-based configuration using the `Config` struct
//! with typestate pattern for type-safe migration generation and database connections.

use crate::schema::{Schema, Snapshot};
use clap::{Parser, Subcommand};
use std::marker::PhantomData;
use std::path::PathBuf;

use drizzle_types::Dialect;

/// Configuration errors
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("IO error: {0}")]
    IoError(String),
    #[error("Parse error: {0}")]
    ParseError(String),
    #[error("Generation error: {0}")]
    GenerationError(String),
    #[error("Connection error: {0}")]
    ConnectionError(String),
}

// =============================================================================
// CLI Arguments (Clap Derive)
// =============================================================================

/// Drizzle CLI arguments
#[derive(Parser, Debug)]
#[command(name = "drizzle")]
#[command(about = "Drizzle migration CLI", long_about = None)]
pub struct CliArgs {
    #[command(subcommand)]
    pub command: CliCommand,
}

/// CLI subcommands
#[derive(Subcommand, Debug)]
pub enum CliCommand {
    /// Generate a new migration
    Generate {
        /// Migration name (optional, auto-generated if not provided)
        #[arg(short, long)]
        name: Option<String>,

        /// Create a custom (empty) migration file
        #[arg(long)]
        custom: bool,
    },
    /// Show migration status
    Status,
    /// Push schema changes directly to the database (no migration file)
    Push,
    /// Introspect an existing database and generate a snapshot
    Introspect {
        /// Output directory for the generated snapshot (defaults to out directory)
        #[arg(short, long)]
        output: Option<std::path::PathBuf>,
    },
}

// =============================================================================
// Dialect Markers (Typestate)
// =============================================================================

/// Marker for no dialect selected
pub struct NoDialect;

/// Marker for SQLite dialect
pub struct SqliteDialect;

/// Marker for PostgreSQL dialect  
pub struct PostgresDialect;

/// Marker for MySQL dialect
pub struct MysqlDialect;

/// Trait for dialect markers
pub trait DialectMarker {
    const DIALECT: Dialect;
}

impl DialectMarker for SqliteDialect {
    const DIALECT: Dialect = Dialect::SQLite;
}

impl DialectMarker for PostgresDialect {
    const DIALECT: Dialect = Dialect::PostgreSQL;
}

impl DialectMarker for MysqlDialect {
    const DIALECT: Dialect = Dialect::MySQL;
}

// =============================================================================
// Connection Markers (Typestate)
// =============================================================================

/// Marker for no connection configured
pub struct NoConnection;

/// Marker for rusqlite connection (file-based SQLite)
#[cfg(feature = "rusqlite")]
pub struct RusqliteConnection;

/// Marker for libsql connection (embedded replica)
#[cfg(feature = "libsql")]
pub struct LibsqlConnection;

/// Marker for turso connection (edge SQLite with auth)
#[cfg(feature = "turso")]
pub struct TursoConnection;

/// Marker for tokio-postgres connection (async PostgreSQL)
#[cfg(feature = "tokio-postgres")]
pub struct TokioPostgresConnection;

/// Marker for sync postgres connection  
#[cfg(feature = "postgres-sync")]
pub struct PostgresSyncConnection;

// =============================================================================
// Driver-Specific Credentials
// =============================================================================

/// Credentials for rusqlite (file-based SQLite)
#[derive(Clone, Debug, Default)]
pub struct SqliteCredentials {
    /// Path to the database file (e.g., "./dev.db", ":memory:")
    pub path: String,
}

impl SqliteCredentials {
    pub fn new(path: impl Into<String>) -> Self {
        Self { path: path.into() }
    }

    pub fn in_memory() -> Self {
        Self {
            path: ":memory:".to_string(),
        }
    }
}

/// Credentials for Turso/LibSQL remote connections
#[derive(Clone, Debug, Default)]
pub struct TursoCredentials {
    /// Remote database URL (e.g., "libsql://mydb-myorg.turso.io")
    pub url: String,
    /// Auth token for authentication
    pub auth_token: String,
}

impl TursoCredentials {
    pub fn new(url: impl Into<String>, auth_token: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            auth_token: auth_token.into(),
        }
    }
}

/// Credentials for LibSQL embedded replica  
#[derive(Clone, Debug, Default)]
pub struct LibsqlCredentials {
    /// Path to local database file
    pub path: String,
    /// Optional sync URL for embedded replica
    pub sync_url: Option<String>,
    /// Optional auth token for sync
    pub auth_token: Option<String>,
}

impl LibsqlCredentials {
    pub fn local(path: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            sync_url: None,
            auth_token: None,
        }
    }

    pub fn with_sync(
        path: impl Into<String>,
        sync_url: impl Into<String>,
        auth_token: impl Into<String>,
    ) -> Self {
        Self {
            path: path.into(),
            sync_url: Some(sync_url.into()),
            auth_token: Some(auth_token.into()),
        }
    }
}

/// Credentials for PostgreSQL connections
#[derive(Clone, Debug, Default)]
pub struct PostgresCredentials {
    /// Host address
    pub host: String,
    /// Port number (default: 5432)
    pub port: u16,
    /// Username
    pub username: String,
    /// Password
    pub password: String,
    /// Database name
    pub database: String,
    /// SSL mode (optional)
    pub ssl_mode: Option<String>,
}

impl PostgresCredentials {
    pub fn new(
        host: impl Into<String>,
        port: u16,
        username: impl Into<String>,
        password: impl Into<String>,
        database: impl Into<String>,
    ) -> Self {
        Self {
            host: host.into(),
            port,
            username: username.into(),
            password: password.into(),
            database: database.into(),
            ssl_mode: None,
        }
    }

    /// Create from a connection URL
    pub fn from_url(url: impl Into<String>) -> Self {
        // Simple URL parsing - a real implementation would parse properly
        Self {
            host: url.into(), // Just store the full URL
            port: 5432,
            username: String::new(),
            password: String::new(),
            database: String::new(),
            ssl_mode: None,
        }
    }

    pub fn ssl_mode(mut self, mode: impl Into<String>) -> Self {
        self.ssl_mode = Some(mode.into());
        self
    }

    /// Build connection string
    pub fn connection_string(&self) -> String {
        let mut s = format!(
            "host={} port={} user={} password={} dbname={}",
            self.host, self.port, self.username, self.password, self.database
        );
        if let Some(ref ssl) = self.ssl_mode {
            s.push_str(&format!(" sslmode={}", ssl));
        }
        s
    }
}

/// Marker for no credentials (CLI-only mode)
#[derive(Clone, Debug, Default)]
pub struct NoCredentials;

// =============================================================================
// Config Struct with Typestate Pattern
// =============================================================================

/// Type-safe configuration for Drizzle migrations.
///
/// This struct uses the typestate pattern with four type parameters:
/// - `S`: The schema type (must implement `Schema`)
/// - `D`: The dialect marker (`SqliteDialect`, `PostgresDialect`, etc.)
/// - `C`: The connection marker (`NoConnection`, `RusqliteConnection`, etc.)
/// - `Creds`: The credentials type (driver-specific)
///
/// # Example
///
/// ```ignore
/// use drizzle_migrations::{Config, SqliteDialect, RusqliteConnection, SqliteCredentials};
/// use my_app::schema::AppSchema;
///
/// // drizzle_config.rs
/// pub fn config() -> Config<AppSchema, SqliteDialect, RusqliteConnection, SqliteCredentials> {
///     Config::builder()
///         .schema::<AppSchema>()
///         .sqlite()
///         .rusqlite("./dev.db")
///         .out("./drizzle")
///         .build()
/// }
///
/// fn main() {
///     config().run_cli()
/// }
/// ```
#[derive(Clone, Debug)]
pub struct Config<S, D, C = NoConnection, Creds = NoCredentials> {
    /// Output directory for migrations (default: "./drizzle")
    pub out: PathBuf,
    /// Enable SQL statement breakpoints (default: true)
    pub breakpoints: bool,
    /// Driver-specific credentials
    pub credentials: Creds,
    /// The schema instance
    pub schema: S,
    pub(crate) _dialect: PhantomData<D>,
    pub(crate) _connection: PhantomData<C>,
}

impl<S: Schema, D: DialectMarker, C, Creds> Config<S, D, C, Creds> {
    /// Get the dialect from the schema
    pub fn dialect(&self) -> Dialect {
        D::DIALECT
    }

    /// Convert the schema to a snapshot
    pub fn to_snapshot(&self) -> Snapshot {
        self.schema.to_snapshot()
    }

    /// Get the migrations directory path
    pub fn migrations_dir(&self) -> PathBuf {
        self.out.clone()
    }

    /// Get the meta directory path
    pub fn meta_dir(&self) -> PathBuf {
        self.migrations_dir().join("meta")
    }

    /// Get the journal file path
    pub fn journal_path(&self) -> PathBuf {
        self.meta_dir().join("_journal.json")
    }

    /// Get reference to the schema
    pub fn schema(&self) -> &S {
        &self.schema
    }

    /// Get the credentials
    pub fn credentials(&self) -> &Creds {
        &self.credentials
    }
}

// =============================================================================
// CLI Helper Methods (no connection required)
// =============================================================================

impl<S: Schema, D: DialectMarker, C, Creds> Config<S, D, C, Creds> {
    /// Generate a new migration
    fn cmd_generate(&self, name: Option<String>, custom: bool) -> Result<(), ConfigError> {
        use crate::journal::Journal;
        use crate::words::generate_migration_tag;

        let migrations_dir = self.migrations_dir();
        let meta_dir = self.meta_dir();
        let journal_path = self.journal_path();

        // Ensure directories exist
        std::fs::create_dir_all(&meta_dir).map_err(|e| ConfigError::IoError(e.to_string()))?;

        // Load or create journal
        let mut journal = Journal::load_or_create(&journal_path, self.dialect())
            .map_err(|e| ConfigError::IoError(e.to_string()))?;

        // Get the migration tag
        let idx = journal.next_idx();
        let tag = if let Some(n) = name {
            format!("{:04}_{}", idx, n)
        } else {
            generate_migration_tag(idx)
        };

        // Create migration folder
        let migration_folder = migrations_dir.join(&tag);
        std::fs::create_dir_all(&migration_folder)
            .map_err(|e| ConfigError::IoError(e.to_string()))?;

        if custom {
            // Custom migration: empty SQL file
            let sql_path = migration_folder.join("migration.sql");
            std::fs::write(
                &sql_path,
                "-- Custom SQL migration file, put your code below! --\n",
            )
            .map_err(|e| ConfigError::IoError(e.to_string()))?;

            // Load previous snapshot or create empty
            let snapshot = self
                .load_latest_snapshot()?
                .unwrap_or_else(|| Snapshot::empty(self.dialect()));
            snapshot
                .save(&migration_folder.join("snapshot.json"))
                .map_err(|e| ConfigError::IoError(e.to_string()))?;

            journal.add_entry(tag.clone(), self.breakpoints);
            journal
                .save(&journal_path)
                .map_err(|e| ConfigError::IoError(e.to_string()))?;

            println!("✓ Created custom migration: {}", tag);
            println!("  Edit: {}", sql_path.display());
            return Ok(());
        }

        // Get current schema snapshot
        let current_snapshot = self.to_snapshot();

        // Load previous snapshot
        let prev_snapshot = self
            .load_latest_snapshot()?
            .unwrap_or_else(|| Snapshot::empty(self.dialect()));

        // Generate diff and SQL statements
        let sql_statements = self.generate_diff(&prev_snapshot, &current_snapshot)?;

        if sql_statements.is_empty() {
            // Clean up the empty folder
            let _ = std::fs::remove_dir(&migration_folder);
            println!("No schema changes, nothing to migrate 😴");
            return Ok(());
        }

        // Write migration.sql
        let sql_content = if self.breakpoints {
            sql_statements.join("\n--> statement-breakpoint\n")
        } else {
            sql_statements.join("\n")
        };
        std::fs::write(migration_folder.join("migration.sql"), &sql_content)
            .map_err(|e| ConfigError::IoError(e.to_string()))?;

        // Save current snapshot
        current_snapshot
            .save(&migration_folder.join("snapshot.json"))
            .map_err(|e| ConfigError::IoError(e.to_string()))?;

        // Update journal
        journal.add_entry(tag.clone(), self.breakpoints);
        journal
            .save(&journal_path)
            .map_err(|e| ConfigError::IoError(e.to_string()))?;

        println!("✓ Your SQL migration ➜ {} 🚀", migration_folder.display());

        Ok(())
    }

    /// Show migration status
    fn cmd_status(&self) -> Result<(), ConfigError> {
        use crate::journal::Journal;

        let journal_path = self.journal_path();

        if !journal_path.exists() {
            println!("No migrations found.");
            return Ok(());
        }

        let journal =
            Journal::load(&journal_path).map_err(|e| ConfigError::IoError(e.to_string()))?;

        println!("Migration Status:");
        println!("  Dialect: {:?}", self.dialect());
        println!("  Output:  {}", self.out.display());
        println!("  Migrations: {}", journal.entries.len());

        for entry in &journal.entries {
            println!("    [{:04}] {}", entry.idx, entry.tag);
        }

        Ok(())
    }

    /// Load the latest snapshot from the migrations folder
    fn load_latest_snapshot(&self) -> Result<Option<Snapshot>, ConfigError> {
        use crate::journal::Journal;

        let journal_path = self.journal_path();

        if !journal_path.exists() {
            return Ok(None);
        }

        let journal =
            Journal::load(&journal_path).map_err(|e| ConfigError::IoError(e.to_string()))?;

        if let Some(last_entry) = journal.entries.last() {
            let snapshot_path = self
                .migrations_dir()
                .join(&last_entry.tag)
                .join("snapshot.json");
            if snapshot_path.exists() {
                let snapshot = Snapshot::load(&snapshot_path, self.dialect())
                    .map_err(|e| ConfigError::IoError(e.to_string()))?;
                return Ok(Some(snapshot));
            }
        }

        Ok(None)
    }

    /// Generate SQL diff between two snapshots
    fn generate_diff(
        &self,
        prev: &Snapshot,
        current: &Snapshot,
    ) -> Result<Vec<String>, ConfigError> {
        match (prev, current) {
            (Snapshot::Sqlite(prev_snap), Snapshot::Sqlite(curr_snap)) => {
                use crate::sqlite::{diff_snapshots, statements::SqliteGenerator};

                let diff = diff_snapshots(prev_snap, curr_snap);
                if !diff.has_changes() {
                    return Ok(Vec::new());
                }

                let generator = SqliteGenerator::new().with_breakpoints(false);
                Ok(generator.generate_migration(&diff))
            }
            (Snapshot::Postgres(prev_snap), Snapshot::Postgres(curr_snap)) => {
                use crate::postgres::{diff_snapshots, statements::PostgresGenerator};

                let diff = diff_snapshots(&prev_snap.ddl, &curr_snap.ddl);
                if !diff.has_changes() {
                    return Ok(Vec::new());
                }

                let generator = PostgresGenerator::new().with_breakpoints(false);
                Ok(generator.generate(&diff.diffs))
            }
            _ => Err(ConfigError::GenerationError(
                "Mismatched snapshot dialects".into(),
            )),
        }
    }
}

// =============================================================================
// Rusqlite Driver (SQLite)
// =============================================================================

#[cfg(feature = "rusqlite")]
impl<S: Schema> Config<S, SqliteDialect, RusqliteConnection, SqliteCredentials> {
    /// Run the CLI with command line arguments for rusqlite connections.
    pub fn run_cli(self) {
        let args = CliArgs::parse();

        let result = match args.command {
            CliCommand::Generate { name, custom } => self.cmd_generate(name, custom),
            CliCommand::Status => self.cmd_status(),
            CliCommand::Push => {
                eprintln!("Push command not yet implemented");
                Ok(())
            }
            CliCommand::Introspect { output } => self.cmd_introspect(output),
        };

        if let Err(e) = result {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }

    /// Introspect the SQLite database and generate a snapshot
    fn cmd_introspect(&self, output: Option<PathBuf>) -> Result<(), ConfigError> {
        use crate::schema::Snapshot;
        use crate::sqlite::ddl::{Table, View, extract_generated_columns};
        use crate::sqlite::introspect::{
            IntrospectionResult, RawColumnInfo, RawForeignKey, RawIndexColumn, RawIndexInfo,
            process_columns, process_foreign_keys, process_indexes, queries,
        };

        let output_dir = output.unwrap_or_else(|| self.out.clone());

        // Ensure output directory exists
        std::fs::create_dir_all(&output_dir).map_err(|e| ConfigError::IoError(e.to_string()))?;

        println!("🔍 Introspecting SQLite database...");
        println!("  Path:   {}", self.credentials.path);
        println!("  Output: {}", output_dir.display());

        // Connect to the database
        let conn = rusqlite::Connection::open(&self.credentials.path)
            .map_err(|e| ConfigError::ConnectionError(e.to_string()))?;

        let mut result = IntrospectionResult::default();

        // Get tables
        let mut stmt = conn
            .prepare(queries::TABLES_QUERY)
            .map_err(|e| ConfigError::ConnectionError(e.to_string()))?;
        let tables: Vec<(String, Option<String>)> = stmt
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
            .map_err(|e| ConfigError::ConnectionError(e.to_string()))?
            .filter_map(|r| r.ok())
            .collect();

        let mut table_sql_map = std::collections::HashMap::new();
        for (name, sql) in &tables {
            if let Some(sql) = sql {
                table_sql_map.insert(name.clone(), sql.clone());
            }
            result.tables.push(Table::new(name.clone()));
        }

        // Get columns
        let mut stmt = conn
            .prepare(queries::COLUMNS_QUERY)
            .map_err(|e| ConfigError::ConnectionError(e.to_string()))?;
        let raw_columns: Vec<RawColumnInfo> = stmt
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
            .map_err(|e| ConfigError::ConnectionError(e.to_string()))?
            .filter_map(|r| r.ok())
            .collect();

        // Parse generated columns from table SQL
        let mut generated_columns = std::collections::HashMap::new();
        for (table_name, sql) in &table_sql_map {
            let table_gens = extract_generated_columns(sql);
            for (col_name, generated_col) in table_gens {
                let key = format!("{}:{}", table_name, col_name);
                generated_columns.insert(key, generated_col);
            }
        }

        let pk_columns = std::collections::HashSet::new();
        let (columns, primary_keys) =
            process_columns(&raw_columns, &generated_columns, &pk_columns);
        result.columns = columns;
        result.primary_keys = primary_keys;

        // Get indexes and foreign keys per table
        for table in &result.tables {
            // Indexes
            let index_query = queries::indexes_query(&table.name);
            if let Ok(mut stmt) = conn.prepare(&index_query) {
                let raw_indexes: Vec<RawIndexInfo> = stmt
                    .query_map([], |row| {
                        Ok(RawIndexInfo {
                            table: table.name.clone(),
                            name: row.get(1)?,
                            unique: row.get(2)?,
                            origin: row.get(3)?,
                            partial: row.get(4)?,
                        })
                    })
                    .map_err(|e| ConfigError::ConnectionError(e.to_string()))?
                    .filter_map(|r| r.ok())
                    .collect();

                // Get index columns
                let mut index_columns = Vec::new();
                for idx in &raw_indexes {
                    let info_query = queries::index_info_query(&idx.name);
                    if let Ok(mut stmt) = conn.prepare(&info_query) {
                        let cols: Vec<RawIndexColumn> = stmt
                            .query_map([], |row| {
                                Ok(RawIndexColumn {
                                    index_name: idx.name.clone(),
                                    seqno: row.get(0)?,
                                    cid: row.get(1)?,
                                    name: row.get(2)?,
                                    desc: row.get(3)?,
                                    coll: row.get(4)?,
                                    key: row.get(5)?,
                                })
                            })
                            .map_err(|e| ConfigError::ConnectionError(e.to_string()))?
                            .filter_map(|r| r.ok())
                            .collect();
                        index_columns.extend(cols);
                    }
                }

                let indexes = process_indexes(&raw_indexes, &index_columns, &table_sql_map);
                result.indexes.extend(indexes);
            }

            // Foreign keys
            let fk_query = queries::foreign_keys_query(&table.name);
            if let Ok(mut stmt) = conn.prepare(&fk_query) {
                let raw_fks: Vec<RawForeignKey> = stmt
                    .query_map([], |row| {
                        Ok(RawForeignKey {
                            table: table.name.clone(),
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
                    .map_err(|e| ConfigError::ConnectionError(e.to_string()))?
                    .filter_map(|r| r.ok())
                    .collect();

                let fks = process_foreign_keys(&raw_fks);
                result.foreign_keys.extend(fks);
            }
        }

        // Get views
        if let Ok(mut stmt) = conn.prepare(queries::VIEWS_QUERY) {
            let views: Vec<View> = stmt
                .query_map([], |row| {
                    let name: String = row.get(0)?;
                    let sql: Option<String> = row.get(1)?;
                    let definition =
                        sql.and_then(|s| crate::sqlite::introspect::parse_view_sql(&s));
                    Ok(View {
                        name,
                        definition,
                        is_existing: false,
                    })
                })
                .map_err(|e| ConfigError::ConnectionError(e.to_string()))?
                .filter_map(|r| r.ok())
                .collect();
            result.views = views;
        }

        // Convert to snapshot and save
        let snapshot = result.to_snapshot();
        let snapshot_path = output_dir.join("snapshot.json");

        Snapshot::Sqlite(snapshot)
            .save(&snapshot_path)
            .map_err(|e| ConfigError::IoError(e.to_string()))?;

        println!("✓ Introspection complete!");
        println!("  Tables:       {}", result.tables.len());
        println!("  Columns:      {}", result.columns.len());
        println!("  Indexes:      {}", result.indexes.len());
        println!("  Foreign keys: {}", result.foreign_keys.len());
        println!("  Views:        {}", result.views.len());
        println!("  Snapshot:     {}", snapshot_path.display());

        Ok(())
    }
}

// =============================================================================
// Postgres-Sync Driver (PostgreSQL)
// =============================================================================

#[cfg(feature = "postgres-sync")]
impl<S: Schema> Config<S, PostgresDialect, PostgresSyncConnection, PostgresCredentials> {
    /// Run the CLI with command line arguments for postgres-sync connections.
    pub fn run_cli(self) {
        let args = CliArgs::parse();

        let result = match args.command {
            CliCommand::Generate { name, custom } => self.cmd_generate(name, custom),
            CliCommand::Status => self.cmd_status(),
            CliCommand::Push => {
                eprintln!("Push command not yet implemented");
                Ok(())
            }
            CliCommand::Introspect { output } => self.cmd_introspect(output),
        };

        if let Err(e) = result {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }

    /// Introspect the PostgreSQL database and generate a snapshot
    fn cmd_introspect(&self, output: Option<PathBuf>) -> Result<(), ConfigError> {
        use crate::postgres::ddl::Schema as DbSchema;
        use crate::postgres::introspect::{
            IntrospectionResult, RawColumnInfo, RawEnumInfo, RawSequenceInfo, RawTableInfo,
            RawViewInfo, process_columns, process_enums, process_sequences, process_tables,
            process_views, queries,
        };
        use crate::schema::Snapshot;

        let output_dir = output.unwrap_or_else(|| self.out.clone());

        // Ensure output directory exists
        std::fs::create_dir_all(&output_dir).map_err(|e| ConfigError::IoError(e.to_string()))?;

        // Build connection URL from credentials
        let conn_url = format!(
            "host={} port={} user={} password={} dbname={}",
            self.credentials.host,
            self.credentials.port,
            self.credentials.username,
            self.credentials.password,
            self.credentials.database
        );

        println!("🔍 Introspecting PostgreSQL database...");
        println!(
            "  Host:   {}:{}",
            self.credentials.host, self.credentials.port
        );
        println!("  DB:     {}", self.credentials.database);
        println!("  Output: {}", output_dir.display());

        // Connect to the database
        let mut client = postgres::Client::connect(&conn_url, postgres::NoTls)
            .map_err(|e| ConfigError::ConnectionError(e.to_string()))?;

        let mut result = IntrospectionResult::default();

        // Get schemas
        let schema_rows = client
            .query(queries::SCHEMAS_QUERY, &[])
            .map_err(|e| ConfigError::ConnectionError(e.to_string()))?;
        for row in &schema_rows {
            let name: String = row.get(0);
            result.schemas.push(DbSchema { name });
        }

        // Get tables
        let table_rows = client
            .query(queries::TABLES_QUERY, &[])
            .map_err(|e| ConfigError::ConnectionError(e.to_string()))?;
        let raw_tables: Vec<RawTableInfo> = table_rows
            .iter()
            .map(|row| RawTableInfo {
                schema: row.get(0),
                name: row.get(1),
                is_rls_enabled: row.get(2),
            })
            .collect();
        result.tables = process_tables(&raw_tables);

        // Get columns
        let column_rows = client
            .query(queries::COLUMNS_QUERY, &[])
            .map_err(|e| ConfigError::ConnectionError(e.to_string()))?;
        let raw_columns: Vec<RawColumnInfo> = column_rows
            .iter()
            .map(|row| RawColumnInfo {
                schema: row.get(0),
                table: row.get(1),
                name: row.get(2),
                column_type: row.get(3),
                type_schema: row.get(4),
                not_null: row.get(5),
                default_value: row.get(6),
                is_identity: row.get(7),
                identity_type: row.get(8),
                is_generated: row.get(9),
                generated_expression: row.get(10),
                ordinal_position: row.get(11),
            })
            .collect();
        result.columns = process_columns(&raw_columns);

        // Get enums
        let enum_rows = client
            .query(queries::ENUMS_QUERY, &[])
            .map_err(|e| ConfigError::ConnectionError(e.to_string()))?;
        let raw_enums: Vec<RawEnumInfo> = enum_rows
            .iter()
            .map(|row| RawEnumInfo {
                schema: row.get(0),
                name: row.get(1),
                values: row.get(2),
            })
            .collect();
        result.enums = process_enums(&raw_enums);

        // Get sequences
        let seq_rows = client
            .query(queries::SEQUENCES_QUERY, &[])
            .map_err(|e| ConfigError::ConnectionError(e.to_string()))?;
        let raw_seqs: Vec<RawSequenceInfo> = seq_rows
            .iter()
            .map(|row| RawSequenceInfo {
                schema: row.get(0),
                name: row.get(1),
                data_type: row.get(2),
                start_value: row.get(3),
                min_value: row.get(4),
                max_value: row.get(5),
                increment: row.get(6),
                cycle: row.get(7),
                cache_value: row.get(8),
            })
            .collect();
        result.sequences = process_sequences(&raw_seqs);

        // Get views
        let view_rows = client
            .query(queries::VIEWS_QUERY, &[])
            .map_err(|e| ConfigError::ConnectionError(e.to_string()))?;
        let raw_views: Vec<RawViewInfo> = view_rows
            .iter()
            .map(|row| RawViewInfo {
                schema: row.get(0),
                name: row.get(1),
                definition: row.get(2),
                is_materialized: row.get(3),
            })
            .collect();
        result.views = process_views(&raw_views);

        // Convert to snapshot and save
        let snapshot = result.to_snapshot();
        let snapshot_path = output_dir.join("snapshot.json");

        Snapshot::Postgres(snapshot)
            .save(&snapshot_path)
            .map_err(|e| ConfigError::IoError(e.to_string()))?;

        println!("✓ Introspection complete!");
        println!("  Schemas:      {}", result.schemas.len());
        println!("  Tables:       {}", result.tables.len());
        println!("  Columns:      {}", result.columns.len());
        println!("  Enums:        {}", result.enums.len());
        println!("  Sequences:    {}", result.sequences.len());
        println!("  Views:        {}", result.views.len());
        println!("  Snapshot:     {}", snapshot_path.display());

        Ok(())
    }
}

// =============================================================================
// LibSQL Driver (SQLite with Embedded Replica) - ASYNC
// =============================================================================

#[cfg(feature = "libsql")]
impl<S: Schema> Config<S, SqliteDialect, LibsqlConnection, LibsqlCredentials> {
    /// Introspect the SQLite database via libsql and generate a snapshot.
    ///
    /// This is an async function - the caller must provide an async runtime.
    pub async fn introspect(&self, output: Option<PathBuf>) -> Result<(), ConfigError> {
        use crate::schema::Snapshot;
        use crate::sqlite::ddl::{Table, extract_generated_columns};
        use crate::sqlite::introspect::{
            IntrospectionResult, RawColumnInfo, RawForeignKey, RawIndexColumn, RawIndexInfo,
            process_columns, process_foreign_keys, process_indexes, queries,
        };

        let output_dir = output.unwrap_or_else(|| self.out.clone());
        std::fs::create_dir_all(&output_dir).map_err(|e| ConfigError::IoError(e.to_string()))?;

        println!("🔍 Introspecting SQLite database via libsql...");
        println!("  Path:   {}", self.credentials.path);
        println!("  Output: {}", output_dir.display());

        // Connect to libsql
        let db = if let (Some(sync_url), Some(auth_token)) =
            (&self.credentials.sync_url, &self.credentials.auth_token)
        {
            libsql::Builder::new_remote_replica(
                &self.credentials.path,
                sync_url.clone(),
                auth_token.clone(),
            )
            .build()
            .await
            .map_err(|e| ConfigError::ConnectionError(e.to_string()))?
        } else {
            libsql::Builder::new_local(&self.credentials.path)
                .build()
                .await
                .map_err(|e| ConfigError::ConnectionError(e.to_string()))?
        };

        let conn = db
            .connect()
            .map_err(|e| ConfigError::ConnectionError(e.to_string()))?;

        let mut result = IntrospectionResult::default();

        // Get tables
        let mut rows = conn
            .query(queries::TABLES_QUERY, ())
            .await
            .map_err(|e| ConfigError::ConnectionError(e.to_string()))?;

        let mut table_sql_map = std::collections::HashMap::new();
        while let Some(row) = rows
            .next()
            .await
            .map_err(|e| ConfigError::ConnectionError(e.to_string()))?
        {
            let name: String = row
                .get(0)
                .map_err(|e| ConfigError::ConnectionError(e.to_string()))?;
            let sql: Option<String> = row.get(1).ok();
            if let Some(ref sql) = sql {
                table_sql_map.insert(name.clone(), sql.clone());
            }
            result.tables.push(Table::new(name));
        }

        // Get columns
        let mut column_rows = conn
            .query(queries::COLUMNS_QUERY, ())
            .await
            .map_err(|e| ConfigError::ConnectionError(e.to_string()))?;

        let mut raw_columns = Vec::new();
        while let Some(row) = column_rows
            .next()
            .await
            .map_err(|e| ConfigError::ConnectionError(e.to_string()))?
        {
            raw_columns.push(RawColumnInfo {
                table: row.get(0).unwrap_or_default(),
                name: row.get(1).unwrap_or_default(),
                column_type: row.get(2).unwrap_or_default(),
                not_null: row.get(3).unwrap_or(false),
                default_value: row.get(4).ok(),
                pk: row.get(5).unwrap_or(0),
                hidden: row.get(6).unwrap_or(0),
                sql: row.get(7).ok(),
            });
        }

        // Parse generated columns
        let mut generated_columns = std::collections::HashMap::new();
        for (table_name, sql) in &table_sql_map {
            let table_gens = extract_generated_columns(sql);
            for (col_name, generated_col) in table_gens {
                let key = format!("{}:{}", table_name, col_name);
                generated_columns.insert(key, generated_col);
            }
        }

        let pk_columns = std::collections::HashSet::new();
        let (columns, primary_keys) =
            process_columns(&raw_columns, &generated_columns, &pk_columns);
        result.columns = columns;
        result.primary_keys = primary_keys;

        // Get indexes per table
        for table in &result.tables {
            let index_query = queries::indexes_query(&table.name);
            if let Ok(mut stmt) = conn.query(&index_query, ()).await {
                let mut raw_indexes = Vec::new();
                while let Ok(Some(row)) = stmt.next().await {
                    raw_indexes.push(RawIndexInfo {
                        table: table.name.clone(),
                        name: row.get(1).unwrap_or_default(),
                        unique: row.get(2).unwrap_or(false),
                        origin: row.get(3).unwrap_or_default(),
                        partial: row.get(4).unwrap_or(false),
                    });
                }

                let mut index_columns = Vec::new();
                for idx in &raw_indexes {
                    let info_query = queries::index_info_query(&idx.name);
                    if let Ok(mut cols_stmt) = conn.query(&info_query, ()).await {
                        while let Ok(Some(row)) = cols_stmt.next().await {
                            index_columns.push(RawIndexColumn {
                                index_name: idx.name.clone(),
                                seqno: row.get(0).unwrap_or(0),
                                cid: row.get(1).unwrap_or(0),
                                name: row.get(2).ok(),
                                desc: row.get(3).unwrap_or(false),
                                coll: row.get(4).unwrap_or_default(),
                                key: row.get(5).unwrap_or(false),
                            });
                        }
                    }
                }

                let indexes = process_indexes(&raw_indexes, &index_columns, &table_sql_map);
                result.indexes.extend(indexes);
            }

            // Foreign keys
            let fk_query = queries::foreign_keys_query(&table.name);
            if let Ok(mut fk_stmt) = conn.query(&fk_query, ()).await {
                let mut raw_fks = Vec::new();
                while let Ok(Some(row)) = fk_stmt.next().await {
                    raw_fks.push(RawForeignKey {
                        table: table.name.clone(),
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
                let fks = process_foreign_keys(&raw_fks);
                result.foreign_keys.extend(fks);
            }
        }

        // Convert to snapshot and save
        let snapshot = result.to_snapshot();
        let snapshot_path = output_dir.join("snapshot.json");

        Snapshot::Sqlite(snapshot)
            .save(&snapshot_path)
            .map_err(|e| ConfigError::IoError(e.to_string()))?;

        println!("✓ Introspection complete!");
        println!("  Tables:       {}", result.tables.len());
        println!("  Columns:      {}", result.columns.len());
        println!("  Indexes:      {}", result.indexes.len());
        println!("  Foreign keys: {}", result.foreign_keys.len());
        println!("  Snapshot:     {}", snapshot_path.display());

        Ok(())
    }
}

// =============================================================================
// Turso Driver (Remote SQLite) - ASYNC
// =============================================================================

#[cfg(feature = "turso")]
impl<S: Schema> Config<S, SqliteDialect, TursoConnection, TursoCredentials> {
    /// Introspect the SQLite database via Turso and generate a snapshot.
    ///
    /// This is an async function - the caller must provide an async runtime.
    pub async fn introspect(&self, output: Option<PathBuf>) -> Result<(), ConfigError> {
        use crate::schema::Snapshot;
        use crate::sqlite::ddl::{Table, extract_generated_columns};
        use crate::sqlite::introspect::{
            IntrospectionResult, RawColumnInfo, RawForeignKey, RawIndexColumn, RawIndexInfo,
            process_columns, process_foreign_keys, process_indexes, queries,
        };

        let output_dir = output.unwrap_or_else(|| self.out.clone());
        std::fs::create_dir_all(&output_dir).map_err(|e| ConfigError::IoError(e.to_string()))?;

        println!("🔍 Introspecting SQLite database via Turso...");
        println!("  URL:    {}", self.credentials.url);
        println!("  Output: {}", output_dir.display());

        // Connect to Turso (remote libsql database)
        // Since turso is remote libsql, we use libsql::Builder::new_remote
        let db = libsql::Builder::new_remote(
            self.credentials.url.clone(),
            self.credentials.auth_token.clone(),
        )
        .build()
        .await
        .map_err(|e| ConfigError::ConnectionError(e.to_string()))?;

        let conn = db
            .connect()
            .map_err(|e| ConfigError::ConnectionError(e.to_string()))?;

        let mut result = IntrospectionResult::default();

        // Get tables
        let mut rows = conn
            .query(queries::TABLES_QUERY, ())
            .await
            .map_err(|e| ConfigError::ConnectionError(e.to_string()))?;

        let mut table_sql_map = std::collections::HashMap::new();
        while let Some(row) = rows
            .next()
            .await
            .map_err(|e| ConfigError::ConnectionError(e.to_string()))?
        {
            let name: String = row
                .get(0)
                .map_err(|e| ConfigError::ConnectionError(e.to_string()))?;
            let sql: Option<String> = row.get(1).ok();
            if let Some(ref sql) = sql {
                table_sql_map.insert(name.clone(), sql.clone());
            }
            result.tables.push(Table::new(name));
        }

        // Get columns
        let mut column_rows = conn
            .query(queries::COLUMNS_QUERY, ())
            .await
            .map_err(|e| ConfigError::ConnectionError(e.to_string()))?;

        let mut raw_columns = Vec::new();
        while let Some(row) = column_rows
            .next()
            .await
            .map_err(|e| ConfigError::ConnectionError(e.to_string()))?
        {
            raw_columns.push(RawColumnInfo {
                table: row.get(0).unwrap_or_default(),
                name: row.get(1).unwrap_or_default(),
                column_type: row.get(2).unwrap_or_default(),
                not_null: row.get(3).unwrap_or(false),
                default_value: row.get(4).ok(),
                pk: row.get(5).unwrap_or(0),
                hidden: row.get(6).unwrap_or(0),
                sql: row.get(7).ok(),
            });
        }

        // Parse generated columns
        let mut generated_columns = std::collections::HashMap::new();
        for (table_name, sql) in &table_sql_map {
            let table_gens = extract_generated_columns(sql);
            for (col_name, generated_col) in table_gens {
                let key = format!("{}:{}", table_name, col_name);
                generated_columns.insert(key, generated_col);
            }
        }

        let pk_columns = std::collections::HashSet::new();
        let (columns, primary_keys) =
            process_columns(&raw_columns, &generated_columns, &pk_columns);
        result.columns = columns;
        result.primary_keys = primary_keys;

        // Get indexes per table
        for table in &result.tables {
            let index_query = queries::indexes_query(&table.name);
            if let Ok(mut stmt) = conn.query(&index_query, ()).await {
                let mut raw_indexes = Vec::new();
                while let Ok(Some(row)) = stmt.next().await {
                    raw_indexes.push(RawIndexInfo {
                        table: table.name.clone(),
                        name: row.get(1).unwrap_or_default(),
                        unique: row.get(2).unwrap_or(false),
                        origin: row.get(3).unwrap_or_default(),
                        partial: row.get(4).unwrap_or(false),
                    });
                }

                let mut index_columns = Vec::new();
                for idx in &raw_indexes {
                    let info_query = queries::index_info_query(&idx.name);
                    if let Ok(mut cols_stmt) = conn.query(&info_query, ()).await {
                        while let Ok(Some(row)) = cols_stmt.next().await {
                            index_columns.push(RawIndexColumn {
                                index_name: idx.name.clone(),
                                seqno: row.get(0).unwrap_or(0),
                                cid: row.get(1).unwrap_or(0),
                                name: row.get(2).ok(),
                                desc: row.get(3).unwrap_or(false),
                                coll: row.get(4).unwrap_or_default(),
                                key: row.get(5).unwrap_or(false),
                            });
                        }
                    }
                }

                let indexes = process_indexes(&raw_indexes, &index_columns, &table_sql_map);
                result.indexes.extend(indexes);
            }

            // Foreign keys
            let fk_query = queries::foreign_keys_query(&table.name);
            if let Ok(mut fk_stmt) = conn.query(&fk_query, ()).await {
                let mut raw_fks = Vec::new();
                while let Ok(Some(row)) = fk_stmt.next().await {
                    raw_fks.push(RawForeignKey {
                        table: table.name.clone(),
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
                let fks = process_foreign_keys(&raw_fks);
                result.foreign_keys.extend(fks);
            }
        }

        // Convert to snapshot and save
        let snapshot = result.to_snapshot();
        let snapshot_path = output_dir.join("snapshot.json");

        Snapshot::Sqlite(snapshot)
            .save(&snapshot_path)
            .map_err(|e| ConfigError::IoError(e.to_string()))?;

        println!("✓ Introspection complete!");
        println!("  Tables:       {}", result.tables.len());
        println!("  Columns:      {}", result.columns.len());
        println!("  Indexes:      {}", result.indexes.len());
        println!("  Foreign keys: {}", result.foreign_keys.len());
        println!("  Snapshot:     {}", snapshot_path.display());

        Ok(())
    }
}

// =============================================================================
// Tokio-Postgres Driver (Async PostgreSQL) - ASYNC
// =============================================================================

#[cfg(feature = "tokio-postgres")]
impl<S: Schema> Config<S, PostgresDialect, TokioPostgresConnection, PostgresCredentials> {
    /// Introspect the PostgreSQL database via tokio-postgres and generate a snapshot.
    ///
    /// This is an async function - the caller must provide an async runtime.
    /// The caller is responsible for spawning the connection task.
    pub async fn introspect(
        &self,
        output: Option<PathBuf>,
        client: &tokio_postgres::Client,
    ) -> Result<(), ConfigError> {
        use crate::postgres::ddl::Schema as DbSchema;
        use crate::postgres::introspect::{
            IntrospectionResult, RawColumnInfo, RawEnumInfo, RawSequenceInfo, RawTableInfo,
            RawViewInfo, process_columns, process_enums, process_sequences, process_tables,
            process_views, queries,
        };
        use crate::schema::Snapshot;

        let output_dir = output.unwrap_or_else(|| self.out.clone());
        std::fs::create_dir_all(&output_dir).map_err(|e| ConfigError::IoError(e.to_string()))?;

        println!("🔍 Introspecting PostgreSQL database via tokio-postgres...");
        println!(
            "  Host:   {}:{}",
            self.credentials.host, self.credentials.port
        );
        println!("  DB:     {}", self.credentials.database);
        println!("  Output: {}", output_dir.display());

        let mut result = IntrospectionResult::default();

        // Get schemas
        let schema_rows = client
            .query(queries::SCHEMAS_QUERY, &[])
            .await
            .map_err(|e| ConfigError::ConnectionError(e.to_string()))?;
        for row in &schema_rows {
            let name: String = row.get(0);
            result.schemas.push(DbSchema { name });
        }

        // Get tables
        let table_rows = client
            .query(queries::TABLES_QUERY, &[])
            .await
            .map_err(|e| ConfigError::ConnectionError(e.to_string()))?;
        let raw_tables: Vec<RawTableInfo> = table_rows
            .iter()
            .map(|row| RawTableInfo {
                schema: row.get(0),
                name: row.get(1),
                is_rls_enabled: row.get(2),
            })
            .collect();
        result.tables = process_tables(&raw_tables);

        // Get columns
        let column_rows = client
            .query(queries::COLUMNS_QUERY, &[])
            .await
            .map_err(|e| ConfigError::ConnectionError(e.to_string()))?;
        let raw_columns: Vec<RawColumnInfo> = column_rows
            .iter()
            .map(|row| RawColumnInfo {
                schema: row.get(0),
                table: row.get(1),
                name: row.get(2),
                column_type: row.get(3),
                type_schema: row.get(4),
                not_null: row.get(5),
                default_value: row.get(6),
                is_identity: row.get(7),
                identity_type: row.get(8),
                is_generated: row.get(9),
                generated_expression: row.get(10),
                ordinal_position: row.get(11),
            })
            .collect();
        result.columns = process_columns(&raw_columns);

        // Get enums
        let enum_rows = client
            .query(queries::ENUMS_QUERY, &[])
            .await
            .map_err(|e| ConfigError::ConnectionError(e.to_string()))?;
        let raw_enums: Vec<RawEnumInfo> = enum_rows
            .iter()
            .map(|row| RawEnumInfo {
                schema: row.get(0),
                name: row.get(1),
                values: row.get(2),
            })
            .collect();
        result.enums = process_enums(&raw_enums);

        // Get sequences
        let seq_rows = client
            .query(queries::SEQUENCES_QUERY, &[])
            .await
            .map_err(|e| ConfigError::ConnectionError(e.to_string()))?;
        let raw_seqs: Vec<RawSequenceInfo> = seq_rows
            .iter()
            .map(|row| RawSequenceInfo {
                schema: row.get(0),
                name: row.get(1),
                data_type: row.get(2),
                start_value: row.get(3),
                min_value: row.get(4),
                max_value: row.get(5),
                increment: row.get(6),
                cycle: row.get(7),
                cache_value: row.get(8),
            })
            .collect();
        result.sequences = process_sequences(&raw_seqs);

        // Get views
        let view_rows = client
            .query(queries::VIEWS_QUERY, &[])
            .await
            .map_err(|e| ConfigError::ConnectionError(e.to_string()))?;
        let raw_views: Vec<RawViewInfo> = view_rows
            .iter()
            .map(|row| RawViewInfo {
                schema: row.get(0),
                name: row.get(1),
                definition: row.get(2),
                is_materialized: row.get(3),
            })
            .collect();
        result.views = process_views(&raw_views);

        // Convert to snapshot and save
        let snapshot = result.to_snapshot();
        let snapshot_path = output_dir.join("snapshot.json");

        Snapshot::Postgres(snapshot)
            .save(&snapshot_path)
            .map_err(|e| ConfigError::IoError(e.to_string()))?;

        println!("✓ Introspection complete!");
        println!("  Schemas:      {}", result.schemas.len());
        println!("  Tables:       {}", result.tables.len());
        println!("  Columns:      {}", result.columns.len());
        println!("  Enums:        {}", result.enums.len());
        println!("  Sequences:    {}", result.sequences.len());
        println!("  Views:        {}", result.views.len());
        println!("  Snapshot:     {}", snapshot_path.display());

        Ok(())
    }
}

// =============================================================================
// ConfigBuilder with Typestate Pattern
// =============================================================================

/// Marker for output directory not set
pub struct OutNotSet;
/// Marker for output directory set  
pub struct OutSet;

/// Builder for creating a `Config` with progressive type refinement.
///
/// Uses typestate pattern to enforce:
/// - `schema()` can only be called once (when S = ())
/// - `sqlite()`/`postgres()` can only be called once (when D = ())
/// - `rusqlite()`/`libsql()`/etc can only be called once (when C = NoConnection)
/// - `out()` can only be called once (transitions OutNotSet -> OutSet)
/// - `build()` is only available when schema and dialect are set
#[derive(Clone, Debug)]
pub struct ConfigBuilder<S, D, C, Creds, Out = OutNotSet> {
    out: PathBuf,
    breakpoints: bool,
    credentials: Creds,
    _schema: PhantomData<S>,
    _dialect: PhantomData<D>,
    _connection: PhantomData<C>,
    _out: PhantomData<Out>,
}

/// Initial state: no schema, no dialect, no connection, output not set
impl ConfigBuilder<(), (), NoConnection, NoCredentials, OutNotSet> {
    /// Create a new builder
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

impl Default for ConfigBuilder<(), (), NoConnection, NoCredentials, OutNotSet> {
    fn default() -> Self {
        Self::new()
    }
}

// Static entry point
impl Config<(), (), NoConnection, NoCredentials> {
    /// Create a new config builder
    pub fn builder() -> ConfigBuilder<(), (), NoConnection, NoCredentials, OutNotSet> {
        ConfigBuilder::new()
    }
}

/// Set schema type - only available when S = () (not yet set)
impl<D, C, Creds, Out> ConfigBuilder<(), D, C, Creds, Out> {
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

/// Set dialect - only available when D = () (not yet set)
impl<S, C, Creds, Out> ConfigBuilder<S, (), C, Creds, Out> {
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

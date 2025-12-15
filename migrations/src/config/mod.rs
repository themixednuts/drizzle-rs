//! Configuration types for drizzle migrations
//!
//! This module provides Rust-based configuration using the `Config` struct
//! with typestate pattern for type-safe migration generation and database connections.
//!
//! # Driver-Specific Builders
//!
//! For the cleanest API, use driver-specific builders:
//!
//! ```ignore
//! use drizzle_migrations::RusqliteConfigBuilder;
//!
//! let config = RusqliteConfigBuilder::new("./dev.db")
//!     .schema::<AppSchema>()
//!     .out("./drizzle")
//!     .build();
//!
//! config.run_cli();
//! ```
//!
//! Available builders:
//! - [`RusqliteConfigBuilder`] - rusqlite (file-based SQLite)
//! - [`LibsqlConfigBuilder`] - libsql (embedded replica SQLite)
//! - [`TursoConfigBuilder`] - turso (edge SQLite)
//! - [`TokioPostgresConfigBuilder`] - tokio-postgres (async PostgreSQL)
//! - [`PostgresSyncConfigBuilder`] - postgres-sync (sync PostgreSQL)

use std::marker::PhantomData;
use std::path::PathBuf;

use drizzle_types::Dialect;

use crate::schema::{Schema, Snapshot};

// =============================================================================
// Submodules
// =============================================================================

mod builder;
pub mod cli;
pub mod credentials;
mod drivers;
pub mod error;
pub mod markers;

// =============================================================================
// Re-exports
// =============================================================================

pub use builder::ConfigBuilder;
pub use cli::{CliArgs, CliCommand};
pub use credentials::{
    LibsqlCredentials, NoCredentials, PostgresCredentials, SqliteCredentials, TursoCredentials,
};
pub use error::ConfigError;
pub use markers::{
    DialectMarker, MysqlDialect, NoConnection, NoDialect, NoSchema, OutNotSet, OutSet,
    PostgresDialect, SqliteDialect,
};

#[cfg(feature = "rusqlite")]
pub use builder::RusqliteConfigBuilder;
#[cfg(feature = "rusqlite")]
pub use markers::RusqliteConnection;

#[cfg(feature = "libsql")]
pub use builder::LibsqlConfigBuilder;
#[cfg(feature = "libsql")]
pub use markers::LibsqlConnection;

#[cfg(feature = "turso")]
pub use builder::TursoConfigBuilder;
#[cfg(feature = "turso")]
pub use markers::TursoConnection;

#[cfg(feature = "tokio-postgres")]
pub use builder::TokioPostgresConfigBuilder;
#[cfg(feature = "tokio-postgres")]
pub use markers::TokioPostgresConnection;

#[cfg(feature = "postgres-sync")]
pub use builder::PostgresSyncConfigBuilder;
#[cfg(feature = "postgres-sync")]
pub use markers::PostgresSyncConnection;

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
    pub(crate) fn cmd_generate(
        &self,
        name: Option<String>,
        custom: bool,
    ) -> Result<(), ConfigError> {
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
    pub(crate) fn cmd_status(&self) -> Result<(), ConfigError> {
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

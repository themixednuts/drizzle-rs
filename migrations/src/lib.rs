//! Drizzle Migrations - Migration infrastructure for drizzle-rs
//!
//! This crate provides types and utilities for:
//! - Schema snapshots compatible with drizzle-kit format
//! - Migration diffing and SQL generation  
//! - **Runtime migration execution** with embedded or remote migrations
//! - Rust-based configuration via `Config` typestate
//!
//! # Runtime Migrations
//!
//! ## Embedded Migrations (recommended for production/serverless)
//!
//! For production deployments, especially serverless, embed migrations at compile time:
//!
//! ```ignore
//! use drizzle_migrations::{Migration, MigrationSet, migrations};
//! use drizzle_types::Dialect;
//!
//! // Option 1: Use the migrations! macro
//! const MIGRATIONS: &[Migration] = migrations![
//!     ("0000_init", include_str!("../drizzle/0000_init/migration.sql")),
//!     ("0001_users", include_str!("../drizzle/0001_users/migration.sql")),
//! ];
//!
//! async fn run_migrations(db: &YourDatabase) {
//!     let set = MigrationSet::new(MIGRATIONS.iter().cloned(), Dialect::PostgreSQL);
//!     
//!     // Get applied migrations from DB
//!     let applied = db.query_column::<String>(&set.query_applied_sql()).await?;
//!     
//!     // Apply pending migrations
//!     for migration in set.pending(&applied) {
//!         for statement in migration.statements() {
//!             db.execute(statement).await?;
//!         }
//!         db.execute(&set.record_migration_sql(migration.tag())).await?;
//!     }
//! }
//! ```
//!
//! ## Loading from Filesystem (for development)
//!
//! During development, load migrations from the migrations directory:
//!
//! ```ignore
//! use drizzle_migrations::MigrationSet;
//! use drizzle_types::Dialect;
//!
//! let set = MigrationSet::from_dir("./drizzle", Dialect::SQLite)?;
//! ```
//!
//! ## Loading from Remote (S3, etc.)
//!
//! For serverless or dynamic environments, load migrations from any source:
//!
//! ```ignore
//! // Load migrations from S3, HTTP, or any other source
//! let migrations_data = fetch_from_s3("my-bucket", "migrations.json").await?;
//! let migrations: Vec<Migration> = serde_json::from_slice(&migrations_data)?;
//! let set = MigrationSet::new(migrations, Dialect::PostgreSQL);
//! ```
//!
//! # CLI Usage
//!
//! For generating migrations, use the `drizzle-cli` crate or the programmatic config API:
//!
//! ## Using drizzle-cli (recommended)
//!
//! ```bash
//! # Install
//! cargo install drizzle-cli
//!
//! # Initialize config
//! drizzle init --dialect sqlite
//!
//! # Generate migrations
//! drizzle generate
//! ```
//!
//! ## Using Programmatic Config
//!
//! If you need the schema in Rust (rather than parsing from files), use the config builders:
//!
//! ```ignore
//! // src/bin/drizzle.rs
//! use drizzle_migrations::RusqliteConfigBuilder;
//! use my_app::schema::AppSchema;
//!
//! fn main() {
//!     RusqliteConfigBuilder::new("./dev.db")
//!         .schema::<AppSchema>()
//!         .out("./drizzle")
//!         .build()
//!         .run_cli();
//! }
//! ```
//!
//! # Available Builders
//!
//! - [`RusqliteConfigBuilder`] - rusqlite (file-based SQLite) - sync
//! - [`LibsqlConfigBuilder`] - libsql (embedded replica SQLite) - async
//! - [`TursoConfigBuilder`] - turso (edge SQLite) - async
//! - [`TokioPostgresConfigBuilder`] - tokio-postgres (async PostgreSQL) - async
//! - [`PostgresSyncConfigBuilder`] - postgres-sync (sync PostgreSQL) - sync

pub mod collection;
pub mod config;

pub mod journal;
pub mod migrator;
pub mod parser;
pub mod postgres;
pub mod schema;
pub mod sqlite;
pub mod traits;
pub mod upgrade;
pub mod utils;
pub mod version;
pub mod words;
pub mod writer;

pub use journal::{Journal, JournalEntry};
pub use migrator::{Migration, MigrationSet, MigratorError};
pub use version::{
    JOURNAL_VERSION, MYSQL_SNAPSHOT_VERSION, ORIGIN_UUID, POSTGRES_SNAPSHOT_VERSION,
    SINGLESTORE_SNAPSHOT_VERSION, SQLITE_SNAPSHOT_VERSION, is_latest_version, is_supported_version,
    needs_upgrade, snapshot_version,
};

// Re-export upgrade utilities
pub use upgrade::{latest_version_for_dialect, needs_upgrade_for_dialect, upgrade_to_latest};

// Re-export core traits and dialect markers
pub use traits::{
    CanUpgrade, Dialect as DialectTrait, DiffType, Entity, EntityKey, EntityKind, MigrationResult,
    Mysql, Postgres, Sqlite, Upgradable, V5, V6, V7, V8, Version, VersionLt, Versioned,
    assert_can_upgrade,
};

// Re-export collection types
pub use collection::{Collection, EntityDiff, diff_collections};

// Re-export serde_json for generated code to use
pub use serde_json;

// Re-export schema types for config-based usage
pub use schema::{Schema, Snapshot};

// Re-export Config typestate types for Rust-based configuration
pub use config::{
    Config, ConfigBuilder, ConfigError, DialectMarker, LibsqlCredentials, MysqlDialect,
    NoConnection, NoCredentials, NoDialect, NoSchema, PostgresCredentials, PostgresDialect,
    SqliteCredentials, SqliteDialect, TursoCredentials,
};

// Conditionally re-export connection markers and driver-specific builders
#[cfg(feature = "rusqlite")]
pub use config::{RusqliteConfigBuilder, RusqliteConnection};

#[cfg(feature = "libsql")]
pub use config::{LibsqlConfigBuilder, LibsqlConnection};

#[cfg(feature = "turso")]
pub use config::{TursoConfigBuilder, TursoConnection};

#[cfg(feature = "tokio-postgres")]
pub use config::{TokioPostgresConfigBuilder, TokioPostgresConnection};

#[cfg(feature = "postgres-sync")]
pub use config::{PostgresSyncConfigBuilder, PostgresSyncConnection};

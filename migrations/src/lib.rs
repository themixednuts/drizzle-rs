//! Drizzle Migrations - Migration infrastructure for drizzle-rs
//!
//! This crate provides types and utilities for:
//! - Schema snapshots compatible with drizzle-kit format
//! - Migration diffing and SQL generation  
//! - Rust-based configuration via `Config` typestate
//! - Migration file writing
//!
//! # Configuration
//!
//! Use the `Config` builder for Rust-based configuration:
//!
//! ```ignore
//! use drizzle_migrations::Config;
//!
//! let config = Config::builder()
//!     .schema::<AppSchema>()
//!     .sqlite()
//!     .rusqlite("./dev.db")
//!     .out("./drizzle")
//!     .build();
//!
//! // Run CLI commands
//! config.run_cli();
//! ```

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
pub use migrator::{Migration, Migrator, MigratorError};
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
    NoConnection, NoCredentials, NoDialect, PostgresCredentials, PostgresDialect,
    SqliteCredentials, SqliteDialect, TursoCredentials,
};

// Conditionally re-export connection markers
#[cfg(feature = "rusqlite")]
pub use config::RusqliteConnection;

#[cfg(feature = "libsql")]
pub use config::LibsqlConnection;

#[cfg(feature = "turso")]
pub use config::TursoConnection;

#[cfg(feature = "tokio-postgres")]
pub use config::TokioPostgresConnection;

#[cfg(feature = "postgres-sync")]
pub use config::PostgresSyncConnection;

//! Drizzle Migrations - Migration infrastructure for drizzle-rs
//!
//! This crate provides types and utilities for:
//! - Schema snapshots compatible with drizzle-kit format
//! - Migration diffing and SQL generation  
//! - Rust-based configuration via `Config` typestate
//! - Migration file writing
//!
//! # Usage
//!
//! ## Step 1: Add a binary target to your Cargo.toml
//!
//! ```toml
//! [[bin]]
//! name = "drizzle"
//! path = "src/bin/drizzle.rs"
//! ```
//!
//! ## Step 2: Create the CLI binary
//!
//! ### Sync Drivers (rusqlite, postgres-sync)
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
//! ### Async Drivers (libsql, turso, tokio-postgres)
//!
//! ```ignore
//! // src/bin/drizzle.rs
//! use drizzle_migrations::TokioPostgresConfigBuilder;
//! use my_app::schema::AppSchema;
//!
//! #[tokio::main]
//! async fn main() {
//!     TokioPostgresConfigBuilder::new("localhost", 5432, "user", "pass", "mydb")
//!         .schema::<AppSchema>()
//!         .out("./drizzle")
//!         .build()
//!         .run_cli()
//!         .await;
//! }
//! ```
//!
//! ## Step 3: Run CLI commands
//!
//! ```bash
//! # Generate a migration from schema changes
//! cargo run --bin drizzle -- generate
//!
//! # Generate with a custom name
//! cargo run --bin drizzle -- generate --name "add_users_table"
//!
//! # Run pending migrations
//! cargo run --bin drizzle -- migrate
//!
//! # Push schema directly to database (no migration file)
//! cargo run --bin drizzle -- push
//!
//! # Introspect database and generate snapshot
//! cargo run --bin drizzle -- introspect
//!
//! # Show migration status
//! cargo run --bin drizzle -- status
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

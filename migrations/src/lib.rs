//! Drizzle Migrations - DDL and migration infrastructure for drizzle-rs
//!
//! This crate provides types and utilities for:
//! - Schema snapshots compatible with drizzle-kit format
//! - Migration diffing and SQL generation
//! - Runtime migration execution with embedded or filesystem migrations
//!
//! # Runtime Migrations
//!
//! ## Using db.migrate() (recommended)
//!
//! The simplest way to run migrations is using the `migrate()` method on your Drizzle instance:
//!
//! ```ignore
//! use drizzle::sqlite::rusqlite::Drizzle;
//! use drizzle_migrations::{migrations, MigrationSet};
//! use drizzle_types::Dialect;
//!
//! // Embed migrations at compile time
//! const MIGRATIONS: &[Migration] = migrations![
//!     ("0000_init", include_str!("../drizzle/0000_init/migration.sql")),
//!     ("0001_users", include_str!("../drizzle/0001_users/migration.sql")),
//! ];
//!
//! fn main() -> drizzle::Result<()> {
//!     let conn = rusqlite::Connection::open("./dev.db")?;
//!     let (mut db, _) = Drizzle::new(conn, ());
//!
//!     let set = MigrationSet::new(MIGRATIONS.iter().cloned(), Dialect::SQLite);
//!     db.migrate(&set)?;
//!
//!     Ok(())
//! }
//! ```
//!
//! ## Loading from Filesystem
//!
//! During development, load migrations from the migrations directory:
//!
//! ```ignore
//! use drizzle_migrations::MigrationSet;
//! use drizzle_types::Dialect;
//!
//! let set = MigrationSet::from_dir("./drizzle", Dialect::SQLite)?;
//! db.migrate(&set)?;
//! ```
//!
//! # CLI Usage
//!
//! For generating migrations, use the `drizzle-cli` crate:
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
//!
//! # Run migrations
//! drizzle migrate
//! ```

pub mod collection;
pub mod generate;
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

// Core migration types
pub use journal::{Journal, JournalEntry};
pub use migrator::{Migration, MigrationSet, MigratorError};
pub use words::PrefixMode;
pub use writer::{MigrationError, MigrationWriter};

// Version constants
pub use version::{
    JOURNAL_VERSION, MYSQL_SNAPSHOT_VERSION, ORIGIN_UUID, POSTGRES_SNAPSHOT_VERSION,
    SINGLESTORE_SNAPSHOT_VERSION, SQLITE_SNAPSHOT_VERSION, is_latest_version, is_supported_version,
    needs_upgrade, snapshot_version,
};

// Upgrade utilities
pub use upgrade::{latest_version_for_dialect, needs_upgrade_for_dialect, upgrade_to_latest};

// Core traits and dialect markers
pub use traits::{
    CanUpgrade, Dialect as DialectTrait, DiffType, Entity, EntityKey, EntityKind, MigrationResult,
    Mysql, Postgres, Sqlite, Upgradable, V5, V6, V7, V8, Version, VersionLt, Versioned,
    assert_can_upgrade,
};

// Collection types for diffing
pub use collection::{Collection, EntityDiff, diff_collections};

// Re-export serde_json for generated code
pub use serde_json;

// Schema types
pub use schema::{Schema, Snapshot};

// Programmatic migration generation
pub use generate::generate;

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
//! ```rust
//! use drizzle_migrations::{Migration, MigrationSet};
//! use drizzle_types::Dialect;
//!
//! // Embed migrations at compile time
//! let migrations = drizzle_migrations::migrations![
//!     ("0000_init", "CREATE TABLE users (id INTEGER PRIMARY KEY);"),
//!     ("0001_posts", "CREATE TABLE posts (id INTEGER PRIMARY KEY);"),
//! ];
//!
//! let set = MigrationSet::new(migrations, Dialect::SQLite);
//! // Then: db.migrate(&set)?;
//! ```
//!
//! ## Loading from Filesystem
//!
//! During development, load migrations from the migrations directory:
//!
//! ```rust,no_run
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! use drizzle_migrations::MigrationSet;
//! use drizzle_types::Dialect;
//!
//! let set = MigrationSet::from_dir("./drizzle", Dialect::SQLite)?;
//! # let _ = set;
//! // db.migrate(&set)?;
//! # Ok(())
//! # }
//! ```
//!
//! # Programmatic Migration Generation (No CLI)
//!
//! Diff two schema snapshots and get SQL statements directly — no files, no CLI:
//!
//! ```rust
//! use drizzle_migrations::{Snapshot, generate};
//!
//! let prev = Snapshot::empty(drizzle_types::Dialect::SQLite);
//! let current = Snapshot::empty(drizzle_types::Dialect::SQLite);
//! let statements = generate(&prev, &current).unwrap();
//! assert!(statements.is_empty());
//! ```
//!
//! ## Introspect & Push (per-driver)
//!
//! Each driver on the `drizzle` crate provides `introspect()` to capture a live
//! database as a [`Snapshot`], and `push()` to diff and apply changes in one call:
//!
//! ```rust,no_run
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! use drizzle_migrations::{Snapshot, generate};
//!
//! // db.introspect() returns a Snapshot from the live database.
//! // db.push(&schema) introspects, diffs, and executes the resulting SQL.
//! // Both are available on all 5 drivers (rusqlite, libsql, turso, postgres, tokio-postgres).
//!
//! // push() is equivalent to introspect + generate + execute:
//! let live = Snapshot::empty(drizzle_types::Dialect::SQLite);
//! let desired = Snapshot::empty(drizzle_types::Dialect::SQLite);
//! let sql = generate(&live, &desired)?;
//! // ... then execute each statement against the database
//! # Ok(())
//! # }
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

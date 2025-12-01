//! Drizzle Migrations - Migration infrastructure for drizzle-rs
//!
//! This crate provides types and utilities for:
//! - Schema snapshots compatible with drizzle-kit format
//! - Migration diffing and SQL generation
//! - Configuration parsing (drizzle.toml)
//! - Migration file writing
//! - Compile-time embedded migrations
//!
//! # Embedded Migrations (Recommended)
//!
//! Use the `include_migrations!` macro to embed migrations at compile time:
//!
//! ```ignore
//! use drizzle::prelude::*;
//!
//! // Embed migrations at compile time
//! const MIGRATIONS: EmbeddedMigrations = include_migrations!("./drizzle");
//!
//! fn main() -> Result<()> {
//!     let conn = rusqlite::Connection::open("app.db")?;
//!     let (db, schema) = Drizzle::new(conn, AppSchema::new());
//!
//!     // Apply embedded migrations
//!     db.migrate(&MIGRATIONS)?;
//!     
//!     Ok(())
//! }
//! ```

pub mod config;
pub mod embedded;
pub mod journal;
pub mod migrator;
pub mod postgres;
pub mod sqlgen;
pub mod sqlite;
pub mod upgrade;
pub mod version;
pub mod words;
pub mod writer;

pub use config::{Dialect, DrizzleConfig};
pub use embedded::{EmbeddedMigration, EmbeddedMigrations};
pub use journal::{Journal, JournalEntry};
pub use migrator::{Migration, Migrator, MigratorError};
pub use version::{
    is_latest_version, is_supported_version, needs_upgrade, snapshot_version, JOURNAL_VERSION,
    MYSQL_SNAPSHOT_VERSION, ORIGIN_UUID, POSTGRES_SNAPSHOT_VERSION, SINGLESTORE_SNAPSHOT_VERSION,
    SQLITE_SNAPSHOT_VERSION,
};

// Re-export serde_json for generated code to use
pub use serde_json;

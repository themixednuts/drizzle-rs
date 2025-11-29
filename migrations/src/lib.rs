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
mod words;
pub mod writer;

pub use config::DrizzleConfig;
pub use embedded::{Dialect, EmbeddedMigration, EmbeddedMigrations};
pub use journal::{Journal, JournalEntry};
pub use migrator::{Migration, Migrator, MigratorError};

// Re-export serde_json for generated code to use
pub use serde_json;

//! Drizzle Schema - Schema metadata and migration types for drizzle-rs
//!
//! This crate provides types and utilities for:
//! - Schema snapshots compatible with drizzle-kit format
//! - Migration diffing and SQL generation
//! - Configuration parsing (drizzle.toml)
//! - Migration file writing
//! - Runtime migration runner
//!
//! # Runtime Migrations
//!
//! ```no_run
//! use drizzle_schema::migrator::Migrator;
//! use std::path::Path;
//!
//! // Load migrator from config
//! let migrator = Migrator::from_config_file(Path::new("drizzle.toml")).unwrap();
//!
//! // Create migrations table
//! let create_table_sql = migrator.create_migrations_table_sql();
//! // Execute: db.execute(&create_table_sql)?;
//!
//! // Get applied migrations from your database
//! let query_sql = migrator.query_applied_sql();
//! // let applied: Vec<String> = db.query(&query_sql)?;
//!
//! // Get pending migrations
//! let applied: Vec<String> = vec![]; // from database
//! let pending = migrator.pending_migrations(&applied);
//!
//! // Apply each pending migration
//! for migration in pending {
//!     for stmt in migration.statements() {
//!         // db.execute(stmt)?;
//!     }
//!     let record_sql = migrator.record_migration_sql(&migration.tag);
//!     // db.execute(&record_sql)?;
//! }
//! ```

pub mod config;
pub mod journal;
pub mod migrator;
pub mod postgres;
pub mod sqlgen;
pub mod sqlite;
mod words;
pub mod writer;

pub use config::DrizzleConfig;
pub use journal::{Journal, JournalEntry};
pub use migrator::{Migration, Migrator, MigratorError};

// Re-export serde_json for generated code to use
pub use serde_json;

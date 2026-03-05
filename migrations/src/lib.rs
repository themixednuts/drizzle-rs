//! Drizzle Migrations - DDL and migration infrastructure for drizzle-rs
//!
//! This crate provides:
//! - migration discovery (`MigrationDir`)
//! - runtime migrate config (`MigrateConfig`)
//! - pure diff APIs (`generate`, `generate_schemas_with`)
//! - build-time migration generation (`build::generate_to_dir`)
//!
//! # Recommended No-CLI Flow
//!
//! 1. In `build.rs`, keep `./drizzle` up to date:
//!
//! ```rust,no_run
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! use drizzle_migrations::build::{GenerateConfig, GenerateOutcome, generate_to_dir};
//! use drizzle_types::Dialect;
//!
//! let cfg = GenerateConfig::new(Dialect::SQLite)
//!     .schema("./src/schema.rs")
//!     .out("./drizzle");
//!
//! // Registers schema files as build.rs inputs.
//! cfg.emit_rerun_if_changed();
//!
//! match generate_to_dir(&cfg)? {
//!     GenerateOutcome::NoChanges => {}
//!     GenerateOutcome::Generated { tag, .. } => {
//!         println!("cargo:warning=generated migration {tag}");
//!     }
//! }
//! # Ok(())
//! # }
//! ```
//!
//! 2. In app code, embed and run migrations:
//!
//! ```rust,no_run
//! # use drizzle_migrations::{MigrateConfig, Migration};
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! # struct Db;
//! # impl Db {
//! #     fn migrate(&self, _migrations: &[Migration], _config: MigrateConfig<'_>) -> Result<(), Box<dyn std::error::Error>> {
//! #         Ok(())
//! #     }
//! # }
//! # let db = Db;
//! // Usually produced by: `drizzle::include_migrations!("./drizzle")`
//! let migrations: Vec<Migration> = Vec::new();
//! db.migrate(&migrations, MigrateConfig::SQLITE)?;
//! # Ok(())
//! # }
//! ```
//!
//! # Runtime Generation APIs (No CLI)
//!
//! Use these when you need runtime diffing between two inputs.
//!
//! ## Snapshot-to-snapshot
//!
//! ```rust
//! use drizzle_migrations::{Snapshot, generate};
//!
//! let prev = Snapshot::empty(drizzle_types::Dialect::SQLite);
//! let current = Snapshot::empty(drizzle_types::Dialect::SQLite);
//! let migration = generate(&prev, &current).unwrap();
//! assert!(migration.statements.is_empty());
//! ```
//!
//! ## Schema-to-schema with rename hints
//!
//! ```rust,no_run
//! use drizzle_migrations::{GenerateOptions, Schema, Snapshot, generate_schemas_with};
//! use drizzle_types::Dialect;
//!
//! # #[derive(Default)]
//! # struct AppSchemaV1;
//! # #[derive(Default)]
//! # struct AppSchemaV2;
//! # impl Schema for AppSchemaV1 {
//! #     fn to_snapshot(&self) -> Snapshot { Snapshot::empty(Dialect::SQLite) }
//! #     fn dialect(&self) -> Dialect { Dialect::SQLite }
//! # }
//! # impl Schema for AppSchemaV2 {
//! #     fn to_snapshot(&self) -> Snapshot { Snapshot::empty(Dialect::SQLite) }
//! #     fn dialect(&self) -> Dialect { Dialect::SQLite }
//! # }
//! let migration = generate_schemas_with(
//!     &AppSchemaV1,
//!     &AppSchemaV2,
//!     GenerateOptions::new()
//!         .rename_table("users_old", "users")
//!         .rename_column("users", "full_name", "name")
//!         .strict_renames(true),
//! )?;
//! # let _ = migration;
//! # Ok::<(), drizzle_migrations::MigrationError>(())
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

pub mod build;
pub mod collection;
pub mod config;
pub mod dir;
pub mod generate;
pub mod journal;
pub mod migrator;
pub mod parser;
pub mod postgres;
pub mod schema;
mod snapshot_builder;
pub mod sqlite;
pub mod traits;
pub mod upgrade;
pub mod utils;
pub mod version;
pub mod words;
pub mod writer;

// Core migration types
pub use config::MigrateConfig;
pub use dir::MigrationDir;
pub use journal::{Journal, JournalEntry};
pub use migrator::{Migration, MigrationSet, MigratorError};
pub use words::{PrefixMode, generate_migration_tag};
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
    Mysql, Postgres, Sqlite, Upgradable, V5, V6, V7, V8, Version, Versioned, assert_can_upgrade,
};

// Collection types for diffing
pub use collection::{Collection, EntityDiff, diff_collections};

// Re-export serde_json for generated code
pub use serde_json;

// Schema types
pub use schema::{Schema, Snapshot};

// Programmatic migration generation
pub use generate::{
    ColumnRenameHint, GenerateOptions, GeneratedMigration, RenameHints, TableRenameHint, generate,
    generate_schemas, generate_schemas_with, generate_with,
};
pub use snapshot_builder::parse_result_to_snapshot;

// Build-time generation helpers (no CLI)
pub use build::{BuildError, Casing, GenerateConfig, GenerateOutcome, generate_to_dir};

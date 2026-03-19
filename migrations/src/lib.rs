//! Drizzle Migrations - DDL and migration infrastructure for drizzle-rs
//!
//! This crate provides:
//! - migration discovery (`MigrationDir`)
//! - runtime tracking config (`Tracking`)
//! - pure diff APIs (`diff`, `diff_schemas_with`)
//! - build-time migration generation (`build::run`)
//!
//! # Recommended No-CLI Flow
//!
//! 1. In `build.rs`, keep `./drizzle` up to date:
//!
//! ```rust,no_run
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! use drizzle_migrations::build::{Config, Output, run};
//! use drizzle_types::Dialect;
//!
//! let cfg = Config::new(Dialect::SQLite)
//!     .file("./src/schema.rs")
//!     .out("./drizzle");
//!
//! // Registers schema files as build.rs inputs.
//! cfg.watch();
//!
//! match run(&cfg)? {
//!     Output::NoChanges => {}
//!     Output::Generated { tag, .. } => {
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
//! # use drizzle_migrations::{Migration, Tracking};
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! # struct Db;
//! # impl Db {
//! #     fn migrate(&self, _migrations: &[Migration], _config: Tracking) -> Result<(), Box<dyn std::error::Error>> {
//! #         Ok(())
//! #     }
//! # }
//! # let db = Db;
//! // Usually produced by: `drizzle::include_migrations!("./drizzle")`
//! let migrations: Vec<Migration> = Vec::new();
//! db.migrate(&migrations, Tracking::SQLITE)?;
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
//! use drizzle_migrations::{Snapshot, diff};
//!
//! let prev = Snapshot::empty(drizzle_types::Dialect::SQLite);
//! let current = Snapshot::empty(drizzle_types::Dialect::SQLite);
//! let migration = diff(&prev, &current).unwrap();
//! assert!(migration.statements.is_empty());
//! ```
//!
//! ## Schema-to-schema with rename hints
//!
//! ```rust,no_run
//! use drizzle_migrations::{Options, Schema, Snapshot, diff_schemas_with};
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
//! let migration = diff_schemas_with(
//!     &AppSchemaV1,
//!     &AppSchemaV2,
//!     Options::new()
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
pub use config::Tracking;
pub use dir::MigrationDir;
pub use journal::{Journal, JournalEntry};
pub use migrator::{
    AppliedMigrationMetadata, MatchedMigrationMetadata, Migration, Migrations, MigratorError,
    match_applied_migration_metadata,
};
pub use words::{PrefixMode, generate_migration_tag};
pub use writer::{MigrationError, Writer};

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
    ColumnRenameHint, Options, Plan, RenameHints, TableRenameHint, diff, diff_schemas,
    diff_schemas_with, diff_with,
};
pub use snapshot_builder::parse_result_to_snapshot;

// Build-time generation helpers (no CLI)
pub use build::{BuildError, Casing, Config, Output, run};

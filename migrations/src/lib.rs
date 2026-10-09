//! Migration generation, tracking, and DDL types for drizzle-rs.
//!
//! What this crate gives you:
//! - [`build::run`]: generate migration folders from `build.rs`.
//! - [`diff`] / [`diff_schemas_with`]: diff two schemas in memory.
//! - [`MigrationDir`] and [`Migration`]: load migrations from disk or memory.
//! - [`Migrations`] and [`Tracking`]: the tracking-table SQL drivers use when
//!   applying migrations.
//! - [`repair::plan`]: reconcile a migration that was interrupted mid-apply.
//!
//! Most users reach this crate through `drizzle` (`db.migrate(...)`,
//! `include_migrations!`) or the `drizzle` CLI.
//!
//! # Examples
//!
//! ## Generate and run migrations without the CLI
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
//! ## Diff two schemas at runtime
//!
//! Snapshot to snapshot:
//!
//! ```rust
//! use drizzle_migrations::{Snapshot, diff};
//!
//! let prev = Snapshot::empty(drizzle_types::Dialect::SQLite);
//! let current = Snapshot::empty(drizzle_types::Dialect::SQLite);
//! let migration = diff(&prev, &current).unwrap();
//! assert!(migration.is_empty());
//! ```
//!
//! Schema to schema, with rename hints:
//!
//! ```rust,no_run
//! use drizzle_migrations::{DiffOptions, Schema, Snapshot, diff_schemas_with};
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
//!     &DiffOptions::new()
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
//! # Install with the drivers the CLI should connect through
//! cargo install drizzle-cli --locked --features sqlite-all   # or postgres-all, mysql-all
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
pub mod history;
pub mod journal;
pub mod migrator;
pub mod mysql;
pub mod naming;
pub mod parser;
pub mod postgres;
pub mod renames;
pub mod repair;
pub mod schema;
pub mod snapshot;
mod snapshot_builder;
pub mod sqlite;
pub mod traits;
pub mod upgrade;
pub mod utils;
pub mod version;
pub mod writer;

// Core migration types
pub use config::Tracking;
pub use dir::MigrationDir;
pub use journal::{Journal, JournalEntry};
pub use migrator::{
    AppliedMigrationMetadata, MatchedMigrationMetadata, MigrateOutcome, Migration, Migrations,
    MigratorError, SqliteMigrationExecution, SqliteMigrationExecutionError,
    is_postgres_concurrent_index_statement, match_applied_migration_metadata,
};
pub use naming::{PrefixMode, generate_migration_tag};
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
    CanUpgrade, Dialect as DialectTrait, DiffResult, DiffType, Entity, EntityKey, EntityKind,
    Mysql, Postgres, Sqlite, Upgradable, V5, V6, V7, V8, Version, Versioned, assert_can_upgrade,
};

// Shared entity-collection backbone (per-dialect lookup helpers attach
// in `sqlite::collection` and `postgres::collection`).
pub use collection::EntityCollection;

// Re-export serde_json for generated code
pub use serde_json;

// Schema types
pub use schema::{Schema, Snapshot};

// Programmatic migration generation
pub use generate::{
    ColumnRenameHint, ConstraintKind, ConstraintRenameHint, DataLoss, DiffOptions, EnumRenameHint,
    IndexRenameHint, Plan, RenameHints, SchemaRenameHint, TableRenameHint, ViewRenameHint, diff,
    diff_schemas, diff_schemas_with, diff_with,
};

// Rename-or-create questions (what `drizzle generate`/`push` prompt for)
pub use renames::{CreateHint, RenameAnswer, RenameKind, RenameQuestion, rename_questions};

// Build-time generation helpers (no CLI)
pub use build::{BuildError, Casing, Config, Output, run};

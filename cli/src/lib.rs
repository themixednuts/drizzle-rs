//! Drizzle CLI - Command-line interface for drizzle-rs migrations
//!
//! This crate provides a standalone CLI tool for managing database migrations
//! using a `drizzle.toml` configuration file instead of requiring Rust code.
//!
//! # Quick Start
//!
//! 1. Install the CLI: `cargo install drizzle-cli`
//! 2. Run `drizzle init` to create a `drizzle.toml`
//! 3. Run `drizzle generate` to create migrations
//!
//! # Configuration
//!
//! Create a `drizzle.toml` file in your project root (or run `drizzle init`):
//!
//! ```toml
//! dialect = "sqlite"
//! schema = "src/schema.rs"
//! out = "./drizzle"
//!
//! [dbCredentials]
//! url = "./dev.db"
//! ```
//!
//! For PostgreSQL:
//!
//! ```toml
//! dialect = "postgresql"
//! schema = "src/schema.rs"
//! out = "./drizzle"
//!
//! [dbCredentials]
//! url = "postgres://user:pass@localhost:5432/mydb"
//! ```
//!
//! # Commands
//!
//! - `drizzle init` - Create a new drizzle.toml configuration file
//! - `drizzle generate` - Generate a new migration from schema changes
//! - `drizzle generate --custom` - Create an empty migration for manual SQL
//! - `drizzle status` - Show migration status
//! - `drizzle migrate` - Run pending migrations (requires database connection)
//! - `drizzle push` - Push schema directly to database (requires database connection)
//! - `drizzle introspect` - Introspect database and generate snapshot (requires database connection)

pub mod commands;
pub mod config;
pub mod db;
pub mod error;
pub mod snapshot;

pub use config::{Config, Credentials, Dialect, Driver, Error as ConfigError};
pub use error::CliError;

// Backwards compatibility alias
pub type DrizzleConfig = Config;

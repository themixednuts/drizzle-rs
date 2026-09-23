//! Drizzle CLI - Command-line interface for drizzle-rs migrations
//!
//! This crate provides a standalone CLI tool for managing database migrations
//! using a `drizzle.config.toml` configuration file instead of requiring Rust code.
//!
//! # Quick Start
//!
//! 1. Install the CLI with the drivers it should connect through:
//!    `cargo install drizzle-cli --locked --features sqlite-all` (or
//!    `postgres-all`, `mysql-all`, or individual driver features such as
//!    `rusqlite`). Without a driver, `migrate`, `push`, and `introspect`
//!    report "No driver available".
//! 2. Run `drizzle init` to create a `drizzle.config.toml`
//! 3. Run `drizzle generate` to create migrations
//!
//! # Configuration
//!
//! Create a `drizzle.config.toml` file in your project root (or run `drizzle init`):
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
//! For `PostgreSQL`:
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
//! - `drizzle init` - Create a new drizzle.config.toml configuration file
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
pub mod output;
pub mod snapshot;

pub use config::{Config, Credentials, Dialect, Driver, Error};
pub use error::CliError;

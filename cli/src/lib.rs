//! Library behind the `drizzle` migration CLI for drizzle-rs.
//!
//! The CLI reads a `drizzle.config.toml` instead of Rust code, so you can
//! generate and apply migrations without a `build.rs`. This crate exposes the
//! config loader ([`Config`]) and the command implementations
//! ([`commands`]) that the binary dispatches to.
//!
//! # Quick start
//!
//! 1. Install the CLI with the drivers it should connect through:
//!    `cargo install drizzle-cli --locked --features sqlite-all` (or
//!    `postgres-all`, `mysql-all`, or single driver features such as
//!    `rusqlite`). Without a driver, `migrate`, `push`, and `introspect`
//!    fail with "No driver available".
//! 2. Run `drizzle init` to create `drizzle.config.toml`.
//! 3. Run `drizzle generate` to write a migration, then `drizzle migrate`.
//!
//! # Configuration
//!
//! `drizzle.config.toml` in the project root (or run `drizzle init`):
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
//! | Command | What it does | Needs a database |
//! |---|---|---|
//! | `drizzle init` | Create `drizzle.config.toml` | no |
//! | `drizzle generate` | Write a migration from schema changes (`--custom` for an empty one) | no |
//! | `drizzle migrate` | Apply pending migrations | yes |
//! | `drizzle push` | Apply the schema directly, without migration files | yes |
//! | `drizzle introspect` / `drizzle pull` | Generate a schema file from a live database | yes |
//! | `drizzle status` | List local migrations and whether each has a snapshot | no |
//! | `drizzle check` | Validate the config file | no |
//! | `drizzle export` | Print the schema as SQL | no |
//! | `drizzle up` | Upgrade old snapshots to the current format | no |
//! | `drizzle import <folder>` | Write the Rust schema of a TypeScript drizzle-orm project from its drizzle-kit snapshots | no |
//! | `drizzle new` | Build a schema file interactively | no |

pub mod codegen;
pub mod commands;
pub mod config;
pub mod db;
pub mod error;
pub mod output;
pub mod snapshot;

pub use config::{Config, Credentials, Dialect, Driver, Error};
pub use error::CliError;

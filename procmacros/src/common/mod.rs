//! Common utilities shared across SQLite and PostgreSQL macro implementations.
//!
//! This module provides shared abstractions to reduce code duplication between
//! the dialect-specific macro implementations.

mod context;
pub(crate) mod generators;
mod helpers;

pub(crate) use context::ModelType;
pub(crate) use helpers::{
    extract_struct_fields, generate_try_from_impl, make_uppercase_path, parse_column_reference,
};

// Re-export dialect traits (always available)
#[allow(unused_imports)]
pub(crate) use generators::{Dialect, GeneratorPaths};

// Re-export dialect implementations (feature-gated)
#[cfg(feature = "sqlite")]
pub(crate) use generators::SqliteDialect;

#[cfg(feature = "postgres")]
pub(crate) use generators::PostgresDialect;

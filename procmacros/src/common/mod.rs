//! Common utilities shared across SQLite and PostgreSQL macro implementations.
//!
//! This module provides shared abstractions to reduce code duplication between
//! the dialect-specific macro implementations.

mod context;
pub(crate) mod generators;
mod helpers;
pub(crate) mod type_mapping;

pub(crate) use context::ModelType;
pub(crate) use helpers::{
    extract_struct_fields, generate_try_from_impl, make_uppercase_path, parse_column_reference,
};
pub(crate) use type_mapping::{
    generate_arithmetic_ops, generate_expr_impl, is_numeric_sql_type, rust_type_to_nullability,
    rust_type_to_sql_type,
};

// Re-export dialect traits (always available)
#[allow(unused_imports)]
pub(crate) use generators::{Dialect, GeneratorPaths};

// Re-export dialect implementations (feature-gated)
#[cfg(feature = "sqlite")]
pub(crate) use generators::SqliteDialect;

#[cfg(feature = "postgres")]
pub(crate) use generators::PostgresDialect;

//! PostgreSQL type definitions
//!
//! This module provides type definitions for PostgreSQL including:
//!
//! - [`PostgreSQLType`] - PostgreSQL column types
//! - [`TypeCategory`] - Rust type classification for PostgreSQL mapping
//! - [`PgTypeCategory`] - SQL type categories for parsing

mod sql_type;
mod type_category;

pub use sql_type::PostgreSQLType;
pub use type_category::{PgTypeCategory, TypeCategory};

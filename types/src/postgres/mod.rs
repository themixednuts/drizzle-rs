//! PostgreSQL type definitions
//!
//! This module provides type definitions for PostgreSQL including:
//!
//! - [`PostgreSQLType`] - PostgreSQL column types
//! - [`TypeCategory`] - Rust type classification for PostgreSQL mapping
//! - [`PgTypeCategory`] - SQL type categories for parsing

pub mod ddl;
mod sql_type;
mod type_category;

pub mod types {
    #[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
    pub struct Int2;

    #[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
    pub struct Int4;

    #[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
    pub struct Int8;

    #[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
    pub struct Float4;

    #[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
    pub struct Float8;

    #[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
    pub struct Varchar;

    #[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
    pub struct Bytea;

    #[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
    pub struct Boolean;

    #[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
    pub struct Timestamptz;
}

pub use sql_type::PostgreSQLType;
pub use type_category::{PgTypeCategory, TypeCategory};

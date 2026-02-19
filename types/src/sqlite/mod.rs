//! SQLite type definitions
//!
//! This module provides type definitions for SQLite including:
//!
//! - [`SQLiteType`] - SQLite column storage types
//! - [`TypeCategory`] - Rust type classification for SQLite mapping
//! - [`SQLTypeCategory`] - SQL type affinity categories for parsing

pub mod ddl;
mod sql_type;
mod type_category;

pub mod types {
    #[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
    pub struct Integer;

    #[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
    pub struct Real;

    #[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
    pub struct Blob;
}

pub use sql_type::SQLiteType;
pub use type_category::{SQLTypeCategory, TypeCategory};

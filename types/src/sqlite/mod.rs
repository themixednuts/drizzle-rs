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
    pub struct Text;

    #[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
    pub struct Real;

    #[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
    pub struct Blob;

    #[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
    pub struct Numeric;

    #[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
    pub struct Any;
}

pub use sql_type::{SQLiteAffinity, SQLiteType};
pub use type_category::{SQLTypeCategory, TypeCategory};

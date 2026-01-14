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

pub use sql_type::SQLiteType;
pub use type_category::{SQLTypeCategory, TypeCategory};

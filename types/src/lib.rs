//! Shared type definitions for Drizzle ORM
//!
//! This crate provides common type definitions used across multiple Drizzle crates,
//! including:
//!
//! - [`Dialect`] - Database dialect enum (SQLite, PostgreSQL, MySQL)
//! - SQLite types in the [`sqlite`] module
//! - PostgreSQL types in the [`postgres`] module
//!
//! # Features
//!
//! - `std` - Standard library support (enabled by default)
//! - `alloc` - Allocator support for no_std environments
//! - `uuid` - Enable UUID type support
//! - `serde` - Enable serde serialization/deserialization
//! - `chrono` - Enable chrono date/time type support
//! - `time` - Enable time crate type support
//! - `geo-types` - Enable geometric type support
//! - `cidr` - Enable network address type support
//! - `bit-vec` - Enable bit vector type support

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(all(feature = "alloc", not(feature = "std")))]
extern crate alloc;

// Internal prelude for std/alloc compatibility
#[allow(unused_imports)]
pub(crate) mod alloc_prelude {
    #[cfg(feature = "std")]
    pub use std::{
        borrow::Cow,
        boxed::Box,
        format,
        string::{String, ToString},
        vec,
        vec::Vec,
    };

    #[cfg(all(feature = "alloc", not(feature = "std")))]
    pub use alloc::{
        borrow::Cow,
        boxed::Box,
        format,
        string::{String, ToString},
        vec,
        vec::Vec,
    };
}

mod dialect;
pub mod postgres;
pub mod serde_helpers;
pub mod sqlite;

pub use dialect::{Dialect, DialectParseError};

/// Prelude module for commonly used types
pub mod prelude {
    pub use crate::Dialect;
    pub use crate::postgres::{PgTypeCategory, PostgreSQLType, TypeCategory as PgRustTypeCategory};
    pub use crate::sqlite::{SQLiteType, SqlTypeCategory, TypeCategory as SqliteRustTypeCategory};
}

//! Shared type definitions for Drizzle ORM
//!
//! This crate provides common type definitions used across multiple Drizzle crates,
//! including:
//!
//! - [`Dialect`] - Database dialect enum (`SQLite`, `PostgreSQL`, `MySQL`)
//! - `SQLite` types in the [`sqlite`] module
//! - `PostgreSQL` types in the [`postgres`] module
//!
//! # Features
//!
//! - `std` - Standard library support (enabled by default)
//! - `alloc` - Allocator support for `no_std` environments
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
#[cfg(any(feature = "std", feature = "alloc"))]
mod migration;
#[cfg(any(feature = "std", feature = "alloc"))]
pub mod postgres;
#[cfg(any(feature = "std", feature = "alloc"))]
pub mod serde_helpers;
#[cfg(any(feature = "std", feature = "alloc"))]
pub mod sql;
#[cfg(any(feature = "std", feature = "alloc"))]
pub mod sqlite;

pub use dialect::{Dialect, DialectParseError};
#[cfg(any(feature = "std", feature = "alloc"))]
pub use migration::{Casing, MigrationTracking};
#[cfg(any(feature = "std", feature = "alloc"))]
pub use sql::*;

/// Prelude module for commonly used types
pub mod prelude {
    pub use crate::Dialect;
    #[cfg(any(feature = "std", feature = "alloc"))]
    pub use crate::postgres::{PgTypeCategory, PostgreSQLType, TypeCategory as PgRustTypeCategory};
    #[cfg(any(feature = "std", feature = "alloc"))]
    pub use crate::sqlite::{SQLTypeCategory, SQLiteType, TypeCategory as SqliteRustTypeCategory};
}

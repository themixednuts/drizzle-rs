//! SQL type definitions shared by the drizzle crates.
//!
//! - [`sql`]: compile-time SQL type markers' capabilities ([`DataType`],
//!   [`Compatible`], [`Assignable`], [`Numeric`], [`Textual`], ...), also
//!   re-exported at the crate root.
//! - [`sqlite`], [`postgres`], [`mysql`]: each dialect's type markers, column
//!   types and DDL definitions.
//! - [`Dialect`]: names a database.
//! - [`MigrationTracking`], [`ConfigValue`], [`Casing`]: migration settings.
//!
//! # Features
//!
//! - `std` (default): standard library support.
//! - `alloc`: `no_std` with an allocator. Without `std` or `alloc`, only
//!   [`Dialect`] is available.
//! - `serde`, `schemars`: serialization and JSON Schema for the DDL types.
//! - `uuid`, `chrono`, `geo-types`, `cidr`, `bit-vec`: add the column types
//!   and Rust type mappings for those crates. `time` is accepted but
//!   currently adds nothing.
//! - `col16`, `col32`, `col64`, `col128`, `col200`: allow row-value tuples of
//!   up to that many columns (8 by default).

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
pub mod mysql;
#[cfg(any(feature = "std", feature = "alloc"))]
mod names;
#[cfg(any(feature = "std", feature = "alloc"))]
pub mod postgres;
#[cfg(any(feature = "std", feature = "alloc"))]
pub mod serde_helpers;
#[cfg(any(feature = "std", feature = "alloc"))]
pub mod sql;
#[cfg(any(feature = "std", feature = "alloc"))]
pub mod sqlite;

pub use dialect::{Dialect, DialectParseError};
#[cfg(feature = "std")]
pub use migration::ConfigValueError;
#[cfg(any(feature = "std", feature = "alloc"))]
pub use migration::{Casing, ConfigValue, MigrationTracking};
#[cfg(any(feature = "std", feature = "alloc"))]
pub use sql::*;

/// The dialect enum and each dialect's column-type and category enums.
pub mod prelude {
    pub use crate::Dialect;
    #[cfg(any(feature = "std", feature = "alloc"))]
    pub use crate::mysql::{MySQLType, MySQLTypeCategory, TypeCategory as MySQLRustTypeCategory};
    #[cfg(any(feature = "std", feature = "alloc"))]
    pub use crate::postgres::{PgTypeCategory, PostgreSQLType, TypeCategory as PgRustTypeCategory};
    #[cfg(any(feature = "std", feature = "alloc"))]
    pub use crate::sqlite::{SQLTypeCategory, SQLiteType, TypeCategory as SqliteRustTypeCategory};
}

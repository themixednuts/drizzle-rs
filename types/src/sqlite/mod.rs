//! `SQLite` types.
//!
//! - [`types`]: zero-sized SQL type markers used for compile-time checks.
//! - [`SQLiteType`]: column storage types as written in DDL.
//! - [`TypeCategory`]: how a Rust field type maps to a `SQLite` column.
//! - [`SQLTypeCategory`]: type affinity categories, used when parsing.
//! - [`ddl`]: schema objects (tables, columns, indexes, ...) for migrations.

pub mod ddl;
mod sql_type;
mod type_category;

/// Zero-sized SQL type markers for the `SQLite` dialect.
///
/// Each marker stands for one storage class or affinity at compile time. The
/// traits in [`crate::sql`] say which markers can be compared, assigned, or
/// used in arithmetic.
pub mod types {
    /// `INTEGER` storage class: a 64-bit signed integer. Also used for booleans.
    #[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
    pub struct Integer;

    /// `TEXT` storage class: a UTF-8 string.
    #[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
    pub struct Text;

    /// `REAL` storage class: a 64-bit float.
    #[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
    pub struct Real;

    /// `BLOB` storage class: raw bytes.
    #[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
    pub struct Blob;

    /// `NUMERIC` affinity: stored as an integer or real when possible.
    #[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
    pub struct Numeric;

    /// Untyped SQL, such as a raw `SQL` fragment. Compatible with every `SQLite` marker.
    #[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
    pub struct Any;
}

pub use sql_type::{SQLiteAffinity, SQLiteType};
pub use type_category::{SQLTypeCategory, TypeCategory};

//! Traits for `PostgreSQL` tables, columns, enums and custom column types,
//! and for decoding values and driver rows.
//!
//! The macros implement most of these: `#[PostgresTable]` implements
//! [`PostgresTable`] and [`PostgresColumn`], and `#[derive(PostgresEnum)]`
//! implements [`DrizzlePostgresColumn`] (and [`PostgresEnum`] for native enums).
//! Implement [`FromPostgresValue`] yourself to decode a custom type.

mod column;
mod table;
mod value;

#[cfg(not(feature = "std"))]
use crate::prelude::*;
pub use column::*;
use core::any::Any;
use drizzle_core::error::DrizzleError;
pub use table::*;
pub use value::*;

use crate::values::{OwnedPostgresValue, PostgresValue};

/// Object-safe view of a Rust enum stored as a `PostgreSQL` enum value.
///
/// `#[derive(PostgresEnum)]` implements it for enums stored as a native
/// `PostgreSQL` enum type; integer-backed (`#[repr(...)]`) enums do not get it.
/// It lets a [`PostgresValue`] hold any enum value as `dyn PostgresEnum` and
/// bind it with its type name.
#[allow(clippy::wrong_self_convention)]
pub trait PostgresEnum: Send + Sync + Any {
    /// Returns the `PostgreSQL` enum type name, such as `"mood"`.
    fn enum_type_name(&self) -> &'static str;

    /// Returns `self` as a trait object.
    fn as_enum(&self) -> &dyn PostgresEnum;

    /// Returns the variant's SQL label, such as `"happy"`.
    fn variant_name(&self) -> &'static str;

    /// Clones this value into a boxed trait object.
    fn into_boxed(&self) -> Box<dyn PostgresEnum>;

    /// Parses a variant from its SQL label.
    ///
    /// # Errors
    ///
    /// Returns [`DrizzleError::ConversionError`] when `value` is not a valid
    /// variant name for this enum.
    fn try_from_str(value: &str) -> Result<Self, DrizzleError>
    where
        Self: Sized;
}

impl core::fmt::Debug for &dyn PostgresEnum {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("PostgresSQLEnum")
            .field("type", &self.enum_type_name())
            .field("variant", &self.variant_name())
            .finish()
    }
}

impl PartialEq for &dyn PostgresEnum {
    fn eq(&self, other: &Self) -> bool {
        self.enum_type_name() == other.enum_type_name()
            && self.variant_name() == other.variant_name()
    }
}

impl core::fmt::Debug for Box<dyn PostgresEnum> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("PostgresSQLEnum")
            .field("type", &self.enum_type_name())
            .field("variant", &self.variant_name())
            .finish()
    }
}

impl Clone for Box<dyn PostgresEnum> {
    fn clone(&self) -> Self {
        self.into_boxed()
    }
}

impl PartialEq for Box<dyn PostgresEnum> {
    fn eq(&self, other: &Self) -> bool {
        self.enum_type_name() == other.enum_type_name()
            && self.variant_name() == other.variant_name()
    }
}

/// A custom Rust type that can be used as a `PostgreSQL` column type.
///
/// `#[derive(PostgresEnum)]` implements it. The table macro uses this trait to
/// detect enum fields, so they need no `#[column(ENUM)]` attribute.
///
/// The associated items say how the column is declared (`SQLType`,
/// `SQL_TYPE`, `NEEDS_CREATE_TYPE`, `SCHEMA`) and how values are read
/// (`decode`) and written (`encode`).
///
/// The blanket `From<Self> for PostgresValue` owns the encoded value because
/// insert/update models may store SQL fragments after the source value is
/// dropped. Call `encode()` directly when you need an immediate borrowed value.
/// Override `encode_owned()` when consuming `self` can avoid cloning owned data.
#[cfg(any(feature = "postgres-sync", feature = "tokio-postgres"))]
#[diagnostic::on_unimplemented(
    message = "`{Self}` cannot be used as a PostgreSQL column type",
    note = "add #[derive(PostgresEnum)] for enum types, or use a supported primitive type"
)]
pub trait DrizzlePostgresColumn: Sized {
    /// Drizzle SQL type marker for this column.
    ///
    /// Use one of the built-in PostgreSQL markers, such as `Text`, `Int4`,
    /// `Bytea`, `Boolean`, `Numeric`, `Enum`, or `Any`.
    type SQLType: drizzle_core::types::DataType;

    /// Column type used in DDL: `"text"`, `"integer"`, or the native enum type name.
    const SQL_TYPE: &'static str;

    /// Whether this requires a `CREATE TYPE` (native PG enum).
    const NEEDS_CREATE_TYPE: bool = false;

    /// Schema the custom type lives in (native PG enums with
    /// `#[postgres_enum(schema = "...")]`). Defaults to `public`.
    const SCHEMA: &'static str = "public";

    /// Reads a value from column `idx` of a driver row.
    ///
    /// # Errors
    ///
    /// Returns [`DrizzleError::ConversionError`] when the column at `idx`
    /// cannot be decoded into this type.
    fn decode(row: &crate::Row, idx: usize) -> Result<Self, DrizzleError>;

    /// Converts the value to a borrowed [`PostgresValue`] for binding.
    fn encode(&self) -> PostgresValue<'_>;

    /// Convert self to an owned `PostgreSQL` value for stored bind parameters.
    ///
    /// The default implementation owns the borrowed result of [`encode`](Self::encode).
    /// Override this for wrappers that can move an internal string or byte buffer
    /// directly into the SQL parameter.
    fn encode_owned(self) -> OwnedPostgresValue {
        self.encode().into_owned()
    }
}

impl<'a, T> From<T> for PostgresValue<'a>
where
    T: DrizzlePostgresColumn,
{
    fn from(value: T) -> Self {
        value.encode_owned().into()
    }
}

/// A custom Rust type that can be used as a `PostgreSQL` column type
/// (variant without a driver feature).
///
/// Without `postgres-sync` or `tokio-postgres` there is no row type, so this
/// version has no `decode` method. Enum derives still compile, but the table
/// macro does not generate row-decoding `TryFrom` impls.
///
/// The blanket `From<Self> for PostgresValue` owns the encoded value because
/// insert/update models may store SQL fragments after the source value is
/// dropped. Call `encode()` directly when you need an immediate borrowed value.
/// Override `encode_owned()` when consuming `self` can avoid cloning owned data.
#[cfg(not(any(feature = "postgres-sync", feature = "tokio-postgres")))]
#[diagnostic::on_unimplemented(
    message = "`{Self}` cannot be used as a PostgreSQL column type",
    note = "add #[derive(PostgresEnum)] for enum types, or use a supported primitive type"
)]
pub trait DrizzlePostgresColumn: Sized {
    /// Drizzle SQL type marker for this column.
    ///
    /// Use one of the built-in PostgreSQL markers, such as `Text`, `Int4`,
    /// `Bytea`, `Boolean`, `Numeric`, `Enum`, or `Any`.
    type SQLType: drizzle_core::types::DataType;

    /// Column type used in DDL: `"text"`, `"integer"`, or the native enum type name.
    const SQL_TYPE: &'static str;

    /// Whether this requires a `CREATE TYPE` (native PG enum).
    const NEEDS_CREATE_TYPE: bool = false;

    /// Schema the custom type lives in (native PG enums with
    /// `#[postgres_enum(schema = "...")]`). Defaults to `public`.
    const SCHEMA: &'static str = "public";

    /// Converts the value to a borrowed [`PostgresValue`] for binding.
    fn encode(&self) -> PostgresValue<'_>;

    /// Convert self to an owned `PostgreSQL` value for stored bind parameters.
    ///
    /// The default implementation owns the borrowed result of [`encode`](Self::encode).
    /// Override this for wrappers that can move an internal string or byte buffer
    /// directly into the SQL parameter.
    fn encode_owned(self) -> OwnedPostgresValue {
        self.encode().into_owned()
    }
}

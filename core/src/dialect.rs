//! Dialect type re-exported from drizzle-types with core-specific extensions.

/// Re-export the unified Dialect enum from drizzle-types
pub use drizzle_types::Dialect;

// =============================================================================
// Type-level dialect markers
// =============================================================================

/// Type-level marker for SQLite.
///
/// Used by [`crate::row::SQLTypeToRust`] to provide SQLite-specific type mappings.
/// SQLite stores dates, UUIDs, and JSON as TEXT, so String fallbacks are always available.
#[derive(Debug, Clone, Copy)]
pub struct SQLiteDialect;

/// Type-level marker for PostgreSQL.
///
/// Used by [`crate::row::SQLTypeToRust`] to provide PostgreSQL-specific type mappings.
/// PostgreSQL uses native binary formats for dates, UUIDs, and JSON, so the corresponding
/// feature flags (`chrono`, `uuid`, `serde`) must be enabled.
#[derive(Debug, Clone, Copy)]
pub struct PostgresDialect;

// =============================================================================
// DialectTypes — maps conceptual SQL types to dialect-native markers
// =============================================================================

use crate::types::{Binary, BooleanLike, DataType, Floating, Integral, Temporal, Textual};

/// Maps conceptual SQL types (Int, Text, Bool, ...) to dialect-native markers.
///
/// Implemented for [`SQLiteDialect`] and [`PostgresDialect`] so that
/// expressions like `i32` can resolve to `sqlite::types::Integer` or
/// `postgres::types::Int4` depending on the value type `V`.
pub trait DialectTypes {
    type SmallInt: DataType + Integral;
    type Int: DataType + Integral;
    type BigInt: DataType + Integral;
    type Float: DataType + Floating;
    type Double: DataType + Floating;
    type Text: DataType + Textual;
    type Bool: DataType + BooleanLike;
    type Bytes: DataType + Binary;
    type Date: DataType + Temporal;
    type Time: DataType + Temporal;
    type Timestamp: DataType + Temporal;
    type TimestampTz: DataType + Temporal;
    type Uuid: DataType;
    type Json: DataType;
    type Jsonb: DataType;
    type Any: DataType;
}

impl DialectTypes for SQLiteDialect {
    type SmallInt = drizzle_types::sqlite::types::Integer;
    type Int = drizzle_types::sqlite::types::Integer;
    type BigInt = drizzle_types::sqlite::types::Integer;
    type Float = drizzle_types::sqlite::types::Real;
    type Double = drizzle_types::sqlite::types::Real;
    type Text = drizzle_types::sqlite::types::Text;
    type Bool = drizzle_types::sqlite::types::Integer;
    type Bytes = drizzle_types::sqlite::types::Blob;
    type Date = drizzle_types::sqlite::types::Text;
    type Time = drizzle_types::sqlite::types::Text;
    type Timestamp = drizzle_types::sqlite::types::Text;
    type TimestampTz = drizzle_types::sqlite::types::Text;
    type Uuid = drizzle_types::sqlite::types::Text;
    type Json = drizzle_types::sqlite::types::Text;
    type Jsonb = drizzle_types::sqlite::types::Text;
    type Any = drizzle_types::sqlite::types::Any;
}

impl DialectTypes for PostgresDialect {
    type SmallInt = drizzle_types::postgres::types::Int2;
    type Int = drizzle_types::postgres::types::Int4;
    type BigInt = drizzle_types::postgres::types::Int8;
    type Float = drizzle_types::postgres::types::Float4;
    type Double = drizzle_types::postgres::types::Float8;
    type Text = drizzle_types::postgres::types::Text;
    type Bool = drizzle_types::postgres::types::Boolean;
    type Bytes = drizzle_types::postgres::types::Bytea;
    type Date = drizzle_types::postgres::types::Date;
    type Time = drizzle_types::postgres::types::Time;
    type Timestamp = drizzle_types::postgres::types::Timestamp;
    type TimestampTz = drizzle_types::postgres::types::Timestamptz;
    type Uuid = drizzle_types::postgres::types::Uuid;
    type Json = drizzle_types::postgres::types::Json;
    type Jsonb = drizzle_types::postgres::types::Jsonb;
    type Any = drizzle_types::postgres::types::Any;
}

/// Parameter placeholder rendering style.
///
/// Decouples placeholder syntax from [`Dialect`] so drivers that speak a
/// given SQL dialect but bind parameters differently (e.g. AWS Aurora Data
/// API — Postgres SQL, named `:N` parameters) can request a non-default
/// style without duplicating the whole dialect plumbing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParamStyle {
    /// `$1, $2, ...` — PostgreSQL native wire protocol.
    DollarNumbered,
    /// `?` — SQLite / MySQL positional.
    Question,
    /// `:1, :2, ...` — AWS Aurora Data API (and drizzle-orm TS driver).
    ///
    /// Names are stringified 1-indexed ordinals, matching the
    /// `SqlParameter { name: "1", ... }` encoding the Data API expects.
    ColonNumbered,
}

impl ParamStyle {
    /// Default placeholder style for a given dialect when the driver hasn't
    /// overridden it.
    #[inline]
    pub const fn for_dialect(dialect: Dialect) -> Self {
        match dialect {
            Dialect::PostgreSQL => ParamStyle::DollarNumbered,
            Dialect::SQLite | Dialect::MySQL => ParamStyle::Question,
        }
    }

    /// Write the placeholder for `index` (1-indexed) to the buffer.
    #[inline]
    pub fn write(self, index: usize, buf: &mut impl core::fmt::Write) {
        match self {
            ParamStyle::DollarNumbered => {
                let _ = buf.write_char('$');
                let _ = write!(buf, "{}", index);
            }
            ParamStyle::ColonNumbered => {
                let _ = buf.write_char(':');
                let _ = write!(buf, "{}", index);
            }
            ParamStyle::Question => {
                let _ = buf.write_char('?');
            }
        }
    }
}

/// Writes a dialect-appropriate placeholder directly to a buffer.
///
/// Equivalent to `ParamStyle::for_dialect(dialect).write(index, buf)`. Kept
/// as a free function for existing call sites that don't need a style override.
#[inline]
pub fn write_placeholder(dialect: Dialect, index: usize, buf: &mut impl core::fmt::Write) {
    ParamStyle::for_dialect(dialect).write(index, buf)
}

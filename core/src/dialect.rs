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

/// Writes a dialect-appropriate placeholder directly to a buffer.
#[inline]
pub fn write_placeholder(dialect: Dialect, index: usize, buf: &mut impl core::fmt::Write) {
    match dialect {
        Dialect::PostgreSQL => {
            let _ = buf.write_char('$');
            let _ = write!(buf, "{}", index);
        }
        Dialect::SQLite | Dialect::MySQL => {
            let _ = buf.write_char('?');
        }
    }
}

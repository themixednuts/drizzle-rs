//! Dialect type re-exported from drizzle-types with core-specific extensions.

use crate::prelude::*;

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

/// Extension trait for Dialect-specific placeholder rendering
pub trait DialectExt {
    /// Renders a placeholder for this dialect with the given 1-based index.
    ///
    /// Returns `Cow::Borrowed("?")` for SQLite/MySQL (zero allocation),
    /// `Cow::Owned` for PostgreSQL numbered placeholders.
    ///
    /// # Examples
    /// - PostgreSQL: `$1`, `$2`, `$3`
    /// - SQLite/MySQL: `?`
    fn render_placeholder(&self, index: usize) -> Cow<'static, str>;

    /// Appends a placeholder directly into an output buffer.
    #[inline]
    fn write_placeholder(&self, index: usize, out: &mut String) {
        match self.render_placeholder(index) {
            Cow::Borrowed(v) => out.push_str(v),
            Cow::Owned(v) => out.push_str(&v),
        }
    }
}

impl DialectExt for Dialect {
    #[inline]
    fn render_placeholder(&self, index: usize) -> Cow<'static, str> {
        match self {
            Dialect::PostgreSQL => Cow::Owned(format!("${}", index)),
            Dialect::SQLite | Dialect::MySQL => Cow::Borrowed("?"),
        }
    }
}

/// Writes a dialect-appropriate placeholder directly to a buffer without allocation.
///
/// Unlike `render_placeholder` which returns `Cow<'static, str>` (allocating for PostgreSQL),
/// this writes directly to any `fmt::Write` implementor with zero allocation.
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

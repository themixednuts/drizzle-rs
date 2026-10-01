//! Dialect markers, per-dialect type mappings, and dialect-only features.
//!
//! - [`Dialect`]: the runtime dialect enum.
//! - [`SQLiteDialect`], [`PostgresDialect`], [`MySQLDialect`]: type-level
//!   markers, chosen by a value type's [`SQLParam::DialectMarker`](crate::SQLParam::DialectMarker).
//! - [`DialectTypes`]: maps generic SQL types (`Int`, `Text`, ...) to each
//!   dialect's native types.
//! - [`DialectSupports`] and [`feature`]: gate functions that only some
//!   dialects have.
//! - [`ParamStyle`]: how placeholders are written.

/// The SQL dialect a value type or schema targets, known at runtime.
pub use drizzle_types::Dialect;

// =============================================================================
// Type-level dialect markers
// =============================================================================

/// Type-level marker for SQLite.
///
/// Selects SQLite type mappings ([`DialectTypes`],
/// [`SQLTypeToRust`](crate::row::SQLTypeToRust)). SQLite stores UUIDs as
/// BLOB and date/time and JSON values as TEXT.
#[derive(Debug, Clone, Copy)]
pub struct SQLiteDialect;

/// Type-level marker for PostgreSQL.
///
/// Selects PostgreSQL type mappings ([`DialectTypes`],
/// [`SQLTypeToRust`](crate::row::SQLTypeToRust)). PostgreSQL has native
/// date/time, UUID and JSON types, so selecting them needs a matching
/// feature (`chrono`, `time` or `jiff`; `uuid`; `serde`).
#[derive(Debug, Clone, Copy)]
pub struct PostgresDialect;

/// Type-level marker for MySQL.
///
/// Selects MySQL type mappings ([`DialectTypes`],
/// [`SQLTypeToRust`](crate::row::SQLTypeToRust)). MySQL keeps signed and
/// unsigned integer types apart, quotes identifiers with backticks, and has
/// one native JSON type. It has no native UUID or time-zone-aware datetime:
/// UUIDs use `BINARY(16)`, and `TimestampTz` maps to `TIMESTAMP`, which
/// follows the session time zone.
#[derive(Debug, Clone, Copy)]
pub struct MySQLDialect;

// =============================================================================
// DialectTypes — maps conceptual SQL types to dialect-native markers
// =============================================================================

use crate::types::{Binary, BooleanLike, DataType, Floating, Integral, Temporal, Textual};

/// Maps generic SQL types (`Int`, `Text`, `Bool`, ...) to each dialect's
/// native type markers.
///
/// Implemented for [`SQLiteDialect`], [`PostgresDialect`] and
/// [`MySQLDialect`]. Dialect-neutral expressions use it to pick a result
/// type: `Int` is `sqlite::types::Integer` for SQLite and
/// `postgres::types::Int4` for PostgreSQL.
pub trait DialectTypes {
    /// 16-bit integer.
    type SmallInt: DataType + Integral;
    /// 32-bit integer.
    type Int: DataType + Integral;
    /// 64-bit integer.
    type BigInt: DataType + Integral;
    /// Single-precision float.
    type Float: DataType + Floating;
    /// Double-precision float.
    type Double: DataType + Floating;
    /// Text.
    type Text: DataType + Textual;
    /// Boolean (an integer on SQLite).
    type Bool: DataType + BooleanLike;
    /// Binary data.
    type Bytes: DataType + Binary;
    /// Calendar date.
    type Date: DataType + Temporal;
    /// Time of day.
    type Time: DataType + Temporal;
    /// Date and time without a time zone.
    type Timestamp: DataType + Temporal;
    /// Date and time with a time zone.
    type TimestampTz: DataType + Temporal;
    /// UUID.
    type Uuid: DataType;
    /// JSON.
    type Json: DataType;
    /// Binary JSON (`jsonb`; plain JSON where the dialect has no `jsonb`).
    type Jsonb: DataType;
    /// A value of unknown type.
    type Any: DataType;

    /// Result type of `RANDOM()`.
    type Random: DataType;
    /// Result type of `SIGN(x)`.
    type Sign: DataType;
    /// Nullability of a math function whose domain is narrower than its
    /// input type (`SQRT`, `LN`, `LOG`, ...) given input nullability `Input`:
    /// SQLite and MySQL answer NULL outside the domain, PostgreSQL raises.
    type DomainNullable<Input: crate::expr::Nullability>: crate::expr::Nullability;
    /// Name of the character-count function (`LENGTH` on SQLite).
    const CHAR_LENGTH_FN: &'static str;
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
    type Uuid = drizzle_types::sqlite::types::Blob;
    type Json = drizzle_types::sqlite::types::Text;
    type Jsonb = drizzle_types::sqlite::types::Text;
    type Any = drizzle_types::sqlite::types::Any;

    type Random = drizzle_types::sqlite::types::Integer;
    type Sign = drizzle_types::sqlite::types::Integer;
    type DomainNullable<Input: crate::expr::Nullability> = crate::expr::Null;
    const CHAR_LENGTH_FN: &'static str = "LENGTH";
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

    type Random = drizzle_types::postgres::types::Float8;
    type Sign = drizzle_types::postgres::types::Float8;
    type DomainNullable<Input: crate::expr::Nullability> = Input;
    const CHAR_LENGTH_FN: &'static str = "CHAR_LENGTH";
}

impl DialectTypes for MySQLDialect {
    type SmallInt = drizzle_types::mysql::types::SmallInt;
    type Int = drizzle_types::mysql::types::Int;
    type BigInt = drizzle_types::mysql::types::BigInt;
    type Float = drizzle_types::mysql::types::Float;
    type Double = drizzle_types::mysql::types::Double;
    type Text = drizzle_types::mysql::types::Text;
    type Bool = drizzle_types::mysql::types::Boolean;
    type Bytes = drizzle_types::mysql::types::Blob;
    type Date = drizzle_types::mysql::types::Date;
    type Time = drizzle_types::mysql::types::Time;
    type Timestamp = drizzle_types::mysql::types::DateTime;
    type TimestampTz = drizzle_types::mysql::types::Timestamp;
    type Uuid = drizzle_types::mysql::types::Binary;
    type Json = drizzle_types::mysql::types::Json;
    type Jsonb = drizzle_types::mysql::types::Json;
    type Any = drizzle_types::mysql::types::Any;

    type Random = drizzle_types::mysql::types::Double;
    type Sign = drizzle_types::mysql::types::BigInt;
    type DomainNullable<Input: crate::expr::Nullability> = crate::expr::Null;
    const CHAR_LENGTH_FN: &'static str = "CHAR_LENGTH";
}

/// How parameter placeholders are written.
///
/// Each [`Dialect`] has a default style ([`ParamStyle::for_dialect`]). A
/// driver that speaks a dialect but binds parameters differently, such as
/// the AWS Aurora Data API (PostgreSQL SQL with `:1, :2` parameters), can
/// pick another style.
///
/// # Examples
///
/// ```
/// use drizzle_core::dialect::{Dialect, ParamStyle};
///
/// let mut sql = String::new();
/// ParamStyle::for_dialect(Dialect::PostgreSQL).write(2, &mut sql);
/// ParamStyle::ColonNumbered.write(3, &mut sql);
/// assert_eq!(sql, "$2:3");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParamStyle {
    /// `$1, $2, ...`: PostgreSQL.
    DollarNumbered,
    /// `?`: SQLite and MySQL, positional.
    Question,
    /// `:1, :2, ...`: AWS Aurora Data API.
    ///
    /// The names are 1-based positions, matching the
    /// `SqlParameter { name: "1", ... }` encoding the Data API expects.
    ColonNumbered,
}

impl ParamStyle {
    /// The default style for `dialect`: `DollarNumbered` for PostgreSQL,
    /// `Question` for SQLite and MySQL.
    #[inline]
    #[must_use]
    pub const fn for_dialect(dialect: Dialect) -> Self {
        match dialect {
            Dialect::PostgreSQL => Self::DollarNumbered,
            Dialect::SQLite | Dialect::MySQL => Self::Question,
        }
    }

    /// Writes the placeholder for the parameter at 1-based position `index`.
    #[inline]
    pub fn write(self, index: usize, buf: &mut impl core::fmt::Write) {
        match self {
            Self::DollarNumbered => {
                let _ = buf.write_char('$');
                let _ = write!(buf, "{index}");
            }
            Self::ColonNumbered => {
                let _ = buf.write_char(':');
                let _ = write!(buf, "{index}");
            }
            Self::Question => {
                let _ = buf.write_char('?');
            }
        }
    }
}

/// Writes the default placeholder of `dialect` for the parameter at 1-based
/// position `index`.
///
/// Same as `ParamStyle::for_dialect(dialect).write(index, buf)`.
#[inline]
pub fn write_placeholder(dialect: Dialect, index: usize, buf: &mut impl core::fmt::Write) {
    ParamStyle::for_dialect(dialect).write(index, buf);
}

/// Feature markers for [`DialectSupports`]: SQL functions that only some
/// dialects have.
pub mod feature {
    /// SQLite date/time functions (`unixepoch`, `strftime`, ...).
    #[derive(Debug, Clone, Copy, Default)]
    pub struct SQLiteDateTime;
    /// PostgreSQL date/time functions (`date_trunc`, `age`, ...).
    #[derive(Debug, Clone, Copy, Default)]
    pub struct PostgresDateTime;
    /// Sequence functions (`nextval`, `currval`, `setval`).
    #[derive(Debug, Clone, Copy, Default)]
    pub struct Sequence;
    /// `TYPEOF`.
    #[derive(Debug, Clone, Copy, Default)]
    pub struct Typeof;
    /// The `EXCLUDED` row in an upsert.
    #[derive(Debug, Clone, Copy, Default)]
    pub struct Excluded;
    /// Aggregate `FILTER (WHERE ...)`.
    #[derive(Debug, Clone, Copy, Default)]
    pub struct AggregateFilter;
    /// PostgreSQL aggregates (`array_agg`, `bool_and`, `json_agg`, ...).
    #[derive(Debug, Clone, Copy, Default)]
    pub struct PostgresAggregate;
    /// SQLite-only aggregates (`total`, ...).
    #[derive(Debug, Clone, Copy, Default)]
    pub struct SQLiteAggregate;
    /// `GROUP_CONCAT`.
    #[derive(Debug, Clone, Copy, Default)]
    pub struct GroupConcat;
    /// PostgreSQL string functions (`initcap`, `split_part`, ...).
    #[derive(Debug, Clone, Copy, Default)]
    pub struct PostgresString;
    /// `LEFT` / `RIGHT`.
    #[derive(Debug, Clone, Copy, Default)]
    pub struct LeftRight;
    /// `LPAD` / `RPAD`.
    #[derive(Debug, Clone, Copy, Default)]
    pub struct Pad;
    /// `REVERSE`.
    #[derive(Debug, Clone, Copy, Default)]
    pub struct Reverse;
    /// `REPEAT`.
    #[derive(Debug, Clone, Copy, Default)]
    pub struct Repeat;
}

/// The dialect `Self` has the SQL feature `Feature`.
///
/// Dialect-specific functions require
/// `V::DialectMarker: DialectSupports<feature::X>`, where `V` is the value
/// type. Calling one on a dialect that lacks it is a compile error rather
/// than a database error:
///
/// ```text
/// error[E0277]: `Sequence` is not available for `SQLiteDialect`
///   = note: use a dialect-specific alternative
/// ```
///
/// # Examples
///
/// ```
/// use drizzle_core::{DialectSupports, PostgresDialect, feature};
///
/// fn nextval<D: DialectSupports<feature::Sequence>>() {}
///
/// nextval::<PostgresDialect>();
/// ```
///
/// ```compile_fail
/// use drizzle_core::{DialectSupports, SQLiteDialect, feature};
///
/// fn nextval<D: DialectSupports<feature::Sequence>>() {}
///
/// // error: `Sequence` is not available for `SQLiteDialect`
/// nextval::<SQLiteDialect>();
/// ```
#[diagnostic::on_unimplemented(
    message = "`{Feature}` is not available for `{Self}`",
    label = "this function is not supported by this dialect",
    note = "use a dialect-specific alternative"
)]
pub trait DialectSupports<Feature> {}

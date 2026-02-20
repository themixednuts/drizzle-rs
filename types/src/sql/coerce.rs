use super::DataType;

#[diagnostic::on_unimplemented(
    message = "SQL type `{Self}` is not compatible with `{Rhs}`",
    label = "these SQL types cannot be compared or coerced",
    note = "compatible types include: integers with integers/floats, text with text/varchar, and any type with itself"
)]
pub trait Compatible<Rhs: DataType = Self>: DataType {}

impl<T: DataType> Compatible<T> for T {}

// =============================================================================
// SQLite compatibility
// =============================================================================

// Integer ↔ Real
impl Compatible<crate::sqlite::types::Real> for crate::sqlite::types::Integer {}
impl Compatible<crate::sqlite::types::Integer> for crate::sqlite::types::Real {}

// Numeric ↔ Integer/Real
impl Compatible<crate::sqlite::types::Integer> for crate::sqlite::types::Numeric {}
impl Compatible<crate::sqlite::types::Real> for crate::sqlite::types::Numeric {}
impl Compatible<crate::sqlite::types::Numeric> for crate::sqlite::types::Integer {}
impl Compatible<crate::sqlite::types::Numeric> for crate::sqlite::types::Real {}

// Blob ↔ Text (SQLite stores UUIDs, etc. as either)
impl Compatible<crate::sqlite::types::Text> for crate::sqlite::types::Blob {}
impl Compatible<crate::sqlite::types::Blob> for crate::sqlite::types::Text {}

// Any ↔ all SQLite types
impl Compatible<crate::sqlite::types::Integer> for crate::sqlite::types::Any {}
impl Compatible<crate::sqlite::types::Text> for crate::sqlite::types::Any {}
impl Compatible<crate::sqlite::types::Real> for crate::sqlite::types::Any {}
impl Compatible<crate::sqlite::types::Blob> for crate::sqlite::types::Any {}
impl Compatible<crate::sqlite::types::Numeric> for crate::sqlite::types::Any {}
impl Compatible<crate::sqlite::types::Any> for crate::sqlite::types::Integer {}
impl Compatible<crate::sqlite::types::Any> for crate::sqlite::types::Text {}
impl Compatible<crate::sqlite::types::Any> for crate::sqlite::types::Real {}
impl Compatible<crate::sqlite::types::Any> for crate::sqlite::types::Blob {}
impl Compatible<crate::sqlite::types::Any> for crate::sqlite::types::Numeric {}

// =============================================================================
// PostgreSQL compatibility
// =============================================================================

// Integer widening: Int2 ↔ Int4 ↔ Int8
impl Compatible<crate::postgres::types::Int4> for crate::postgres::types::Int2 {}
impl Compatible<crate::postgres::types::Int8> for crate::postgres::types::Int2 {}
impl Compatible<crate::postgres::types::Int2> for crate::postgres::types::Int4 {}
impl Compatible<crate::postgres::types::Int8> for crate::postgres::types::Int4 {}
impl Compatible<crate::postgres::types::Int2> for crate::postgres::types::Int8 {}
impl Compatible<crate::postgres::types::Int4> for crate::postgres::types::Int8 {}

// Float widening: Float4 ↔ Float8
impl Compatible<crate::postgres::types::Float8> for crate::postgres::types::Float4 {}
impl Compatible<crate::postgres::types::Float4> for crate::postgres::types::Float8 {}

// Int ↔ Float cross-compatibility
impl Compatible<crate::postgres::types::Float4> for crate::postgres::types::Int2 {}
impl Compatible<crate::postgres::types::Float8> for crate::postgres::types::Int2 {}
impl Compatible<crate::postgres::types::Float4> for crate::postgres::types::Int4 {}
impl Compatible<crate::postgres::types::Float8> for crate::postgres::types::Int4 {}
impl Compatible<crate::postgres::types::Float4> for crate::postgres::types::Int8 {}
impl Compatible<crate::postgres::types::Float8> for crate::postgres::types::Int8 {}
impl Compatible<crate::postgres::types::Int2> for crate::postgres::types::Float4 {}
impl Compatible<crate::postgres::types::Int4> for crate::postgres::types::Float4 {}
impl Compatible<crate::postgres::types::Int8> for crate::postgres::types::Float4 {}
impl Compatible<crate::postgres::types::Int2> for crate::postgres::types::Float8 {}
impl Compatible<crate::postgres::types::Int4> for crate::postgres::types::Float8 {}
impl Compatible<crate::postgres::types::Int8> for crate::postgres::types::Float8 {}

// Numeric ↔ all numeric types
impl Compatible<crate::postgres::types::Numeric> for crate::postgres::types::Int2 {}
impl Compatible<crate::postgres::types::Numeric> for crate::postgres::types::Int4 {}
impl Compatible<crate::postgres::types::Numeric> for crate::postgres::types::Int8 {}
impl Compatible<crate::postgres::types::Numeric> for crate::postgres::types::Float4 {}
impl Compatible<crate::postgres::types::Numeric> for crate::postgres::types::Float8 {}
impl Compatible<crate::postgres::types::Int2> for crate::postgres::types::Numeric {}
impl Compatible<crate::postgres::types::Int4> for crate::postgres::types::Numeric {}
impl Compatible<crate::postgres::types::Int8> for crate::postgres::types::Numeric {}
impl Compatible<crate::postgres::types::Float4> for crate::postgres::types::Numeric {}
impl Compatible<crate::postgres::types::Float8> for crate::postgres::types::Numeric {}

// Text type cross-compatibility
impl Compatible<crate::postgres::types::Varchar> for crate::postgres::types::Text {}
impl Compatible<crate::postgres::types::Text> for crate::postgres::types::Varchar {}
impl Compatible<crate::postgres::types::Char> for crate::postgres::types::Text {}
impl Compatible<crate::postgres::types::Text> for crate::postgres::types::Char {}
impl Compatible<crate::postgres::types::Varchar> for crate::postgres::types::Char {}
impl Compatible<crate::postgres::types::Char> for crate::postgres::types::Varchar {}

// Temporal cross-compatibility
impl Compatible<crate::postgres::types::Timestamp> for crate::postgres::types::Timestamptz {}
impl Compatible<crate::postgres::types::Timestamptz> for crate::postgres::types::Timestamp {}
impl Compatible<crate::postgres::types::Timetz> for crate::postgres::types::Time {}
impl Compatible<crate::postgres::types::Time> for crate::postgres::types::Timetz {}

// JSON cross-compatibility
impl Compatible<crate::postgres::types::Jsonb> for crate::postgres::types::Json {}
impl Compatible<crate::postgres::types::Json> for crate::postgres::types::Jsonb {}

// Text ↔ Temporal (string comparisons with timestamps)
impl Compatible<crate::postgres::types::Text> for crate::postgres::types::Timestamp {}
impl Compatible<crate::postgres::types::Text> for crate::postgres::types::Timestamptz {}
impl Compatible<crate::postgres::types::Text> for crate::postgres::types::Date {}
impl Compatible<crate::postgres::types::Text> for crate::postgres::types::Time {}
impl Compatible<crate::postgres::types::Timestamp> for crate::postgres::types::Text {}
impl Compatible<crate::postgres::types::Timestamptz> for crate::postgres::types::Text {}
impl Compatible<crate::postgres::types::Date> for crate::postgres::types::Text {}
impl Compatible<crate::postgres::types::Time> for crate::postgres::types::Text {}

// Any ↔ all PostgreSQL types
impl Compatible<crate::postgres::types::Int2> for crate::postgres::types::Any {}
impl Compatible<crate::postgres::types::Int4> for crate::postgres::types::Any {}
impl Compatible<crate::postgres::types::Int8> for crate::postgres::types::Any {}
impl Compatible<crate::postgres::types::Float4> for crate::postgres::types::Any {}
impl Compatible<crate::postgres::types::Float8> for crate::postgres::types::Any {}
impl Compatible<crate::postgres::types::Varchar> for crate::postgres::types::Any {}
impl Compatible<crate::postgres::types::Text> for crate::postgres::types::Any {}
impl Compatible<crate::postgres::types::Char> for crate::postgres::types::Any {}
impl Compatible<crate::postgres::types::Bytea> for crate::postgres::types::Any {}
impl Compatible<crate::postgres::types::Boolean> for crate::postgres::types::Any {}
impl Compatible<crate::postgres::types::Timestamp> for crate::postgres::types::Any {}
impl Compatible<crate::postgres::types::Date> for crate::postgres::types::Any {}
impl Compatible<crate::postgres::types::Time> for crate::postgres::types::Any {}
impl Compatible<crate::postgres::types::Timetz> for crate::postgres::types::Any {}
impl Compatible<crate::postgres::types::Numeric> for crate::postgres::types::Any {}
impl Compatible<crate::postgres::types::Timestamptz> for crate::postgres::types::Any {}
impl Compatible<crate::postgres::types::Uuid> for crate::postgres::types::Any {}
impl Compatible<crate::postgres::types::Json> for crate::postgres::types::Any {}
impl Compatible<crate::postgres::types::Jsonb> for crate::postgres::types::Any {}

impl Compatible<crate::postgres::types::Any> for crate::postgres::types::Int2 {}
impl Compatible<crate::postgres::types::Any> for crate::postgres::types::Int4 {}
impl Compatible<crate::postgres::types::Any> for crate::postgres::types::Int8 {}
impl Compatible<crate::postgres::types::Any> for crate::postgres::types::Float4 {}
impl Compatible<crate::postgres::types::Any> for crate::postgres::types::Float8 {}
impl Compatible<crate::postgres::types::Any> for crate::postgres::types::Varchar {}
impl Compatible<crate::postgres::types::Any> for crate::postgres::types::Text {}
impl Compatible<crate::postgres::types::Any> for crate::postgres::types::Char {}
impl Compatible<crate::postgres::types::Any> for crate::postgres::types::Bytea {}
impl Compatible<crate::postgres::types::Any> for crate::postgres::types::Boolean {}
impl Compatible<crate::postgres::types::Any> for crate::postgres::types::Timestamp {}
impl Compatible<crate::postgres::types::Any> for crate::postgres::types::Date {}
impl Compatible<crate::postgres::types::Any> for crate::postgres::types::Time {}
impl Compatible<crate::postgres::types::Any> for crate::postgres::types::Timetz {}
impl Compatible<crate::postgres::types::Any> for crate::postgres::types::Numeric {}
impl Compatible<crate::postgres::types::Any> for crate::postgres::types::Timestamptz {}
impl Compatible<crate::postgres::types::Any> for crate::postgres::types::Uuid {}
impl Compatible<crate::postgres::types::Any> for crate::postgres::types::Json {}
impl Compatible<crate::postgres::types::Any> for crate::postgres::types::Jsonb {}
impl Compatible<crate::postgres::types::Any> for crate::postgres::types::Interval {}
impl Compatible<crate::postgres::types::Any> for crate::postgres::types::Inet {}
impl Compatible<crate::postgres::types::Any> for crate::postgres::types::Cidr {}
impl Compatible<crate::postgres::types::Any> for crate::postgres::types::MacAddr {}
impl Compatible<crate::postgres::types::Any> for crate::postgres::types::MacAddr8 {}
impl Compatible<crate::postgres::types::Any> for crate::postgres::types::Point {}
impl Compatible<crate::postgres::types::Any> for crate::postgres::types::LineString {}
impl Compatible<crate::postgres::types::Any> for crate::postgres::types::Rect {}
impl Compatible<crate::postgres::types::Any> for crate::postgres::types::BitString {}

// Inet ↔ Cidr
impl Compatible<crate::postgres::types::Cidr> for crate::postgres::types::Inet {}
impl Compatible<crate::postgres::types::Inet> for crate::postgres::types::Cidr {}

// MacAddr ↔ MacAddr8
impl Compatible<crate::postgres::types::MacAddr8> for crate::postgres::types::MacAddr {}
impl Compatible<crate::postgres::types::MacAddr> for crate::postgres::types::MacAddr8 {}

// Any ↔ new markers (reverse direction)
impl Compatible<crate::postgres::types::Interval> for crate::postgres::types::Any {}
impl Compatible<crate::postgres::types::Inet> for crate::postgres::types::Any {}
impl Compatible<crate::postgres::types::Cidr> for crate::postgres::types::Any {}
impl Compatible<crate::postgres::types::MacAddr> for crate::postgres::types::Any {}
impl Compatible<crate::postgres::types::MacAddr8> for crate::postgres::types::Any {}
impl Compatible<crate::postgres::types::Point> for crate::postgres::types::Any {}
impl Compatible<crate::postgres::types::LineString> for crate::postgres::types::Any {}
impl Compatible<crate::postgres::types::Rect> for crate::postgres::types::Any {}
impl Compatible<crate::postgres::types::BitString> for crate::postgres::types::Any {}

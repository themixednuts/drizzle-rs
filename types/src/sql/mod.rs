//! SQL data type markers for compile-time type safety.
//!
//! This module provides zero-sized type markers that represent SQL data types
//! at the Rust type level, enabling the type system to verify compatible
//! comparisons and operations at compile time.

use core::marker::PhantomData;

mod coerce;
mod ops;

pub use coerce::*;
pub use ops::*;

mod private {
    pub trait Sealed {}
}

/// Represents a SQL data type at the type level.
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not a recognized SQL data type",
    label = "use a drizzle SQL type marker (Int, Text, Bool, etc.)"
)]
pub trait DataType: private::Sealed + Copy + Default + 'static {}

/// Numeric SQL types that support arithmetic operations (+, -, *, /).
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not a numeric SQL type",
    label = "arithmetic operations require Int, SmallInt, BigInt, Float, or Double"
)]
pub trait Numeric: DataType {}

/// Integer SQL types (SMALLINT, INTEGER, BIGINT).
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not an integer SQL type",
    label = "expected SmallInt, Int, or BigInt"
)]
pub trait Integral: Numeric {}

/// Floating-point SQL types (REAL, DOUBLE PRECISION).
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not a floating-point SQL type",
    label = "expected Float or Double"
)]
pub trait Floating: Numeric {}

/// String/text SQL types (TEXT, VARCHAR, CHAR).
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not a text SQL type",
    label = "expected Text or VarChar"
)]
pub trait Textual: DataType {}

/// Binary data types (BLOB, BYTEA).
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not a binary SQL type",
    label = "expected Bytes (BLOB/BYTEA)"
)]
pub trait Binary: DataType {}

/// Temporal SQL types (DATE, TIME, TIMESTAMP).
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not a temporal SQL type",
    label = "expected Date, Time, Timestamp, or TimestampTz"
)]
pub trait Temporal: DataType {}

/// Boolean-like SQL types that support logical operations (NOT, AND, OR).
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not a boolean SQL type",
    label = "logical operations require a boolean-typed expression"
)]
pub trait BooleanLike: DataType {}

/// PostgreSQL-style SQL array type marker.
///
/// `Array<T>` represents an array whose element SQL type is `T`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct Array<T: DataType>(pub PhantomData<T>);

/// Placeholder marker used for bind parameters before concrete typing.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct Placeholder;

impl<T: DataType> private::Sealed for Array<T> {}
impl<T: DataType> DataType for Array<T> {}
impl private::Sealed for Placeholder {}
impl DataType for Placeholder {}
impl Textual for Placeholder {}

// =============================================================================
// SQLite dialect marker impls
// =============================================================================

impl private::Sealed for crate::sqlite::types::Integer {}
impl private::Sealed for crate::sqlite::types::Text {}
impl private::Sealed for crate::sqlite::types::Real {}
impl private::Sealed for crate::sqlite::types::Blob {}
impl private::Sealed for crate::sqlite::types::Numeric {}
impl private::Sealed for crate::sqlite::types::Any {}

impl DataType for crate::sqlite::types::Integer {}
impl DataType for crate::sqlite::types::Text {}
impl DataType for crate::sqlite::types::Real {}
impl DataType for crate::sqlite::types::Blob {}
impl DataType for crate::sqlite::types::Numeric {}
impl DataType for crate::sqlite::types::Any {}

impl Numeric for crate::sqlite::types::Integer {}
impl Numeric for crate::sqlite::types::Real {}
impl Numeric for crate::sqlite::types::Numeric {}
impl Numeric for crate::sqlite::types::Any {}

impl Integral for crate::sqlite::types::Integer {}

impl Floating for crate::sqlite::types::Real {}

impl Textual for crate::sqlite::types::Text {}
impl Textual for crate::sqlite::types::Any {}

impl Binary for crate::sqlite::types::Blob {}

impl Temporal for crate::sqlite::types::Integer {}
impl Temporal for crate::sqlite::types::Real {}
impl Temporal for crate::sqlite::types::Text {}
impl Temporal for crate::sqlite::types::Numeric {}

impl BooleanLike for crate::sqlite::types::Integer {}

// =============================================================================
// PostgreSQL dialect marker impls
// =============================================================================

impl private::Sealed for crate::postgres::types::Int2 {}
impl private::Sealed for crate::postgres::types::Int4 {}
impl private::Sealed for crate::postgres::types::Int8 {}
impl private::Sealed for crate::postgres::types::Float4 {}
impl private::Sealed for crate::postgres::types::Float8 {}
impl private::Sealed for crate::postgres::types::Varchar {}
impl private::Sealed for crate::postgres::types::Text {}
impl private::Sealed for crate::postgres::types::Char {}
impl private::Sealed for crate::postgres::types::Bytea {}
impl private::Sealed for crate::postgres::types::Boolean {}
impl private::Sealed for crate::postgres::types::Timestamptz {}
impl private::Sealed for crate::postgres::types::Timestamp {}
impl private::Sealed for crate::postgres::types::Date {}
impl private::Sealed for crate::postgres::types::Time {}
impl private::Sealed for crate::postgres::types::Timetz {}
impl private::Sealed for crate::postgres::types::Numeric {}
impl private::Sealed for crate::postgres::types::Uuid {}
impl private::Sealed for crate::postgres::types::Json {}
impl private::Sealed for crate::postgres::types::Jsonb {}
impl private::Sealed for crate::postgres::types::Any {}
impl private::Sealed for crate::postgres::types::Interval {}
impl private::Sealed for crate::postgres::types::Inet {}
impl private::Sealed for crate::postgres::types::Cidr {}
impl private::Sealed for crate::postgres::types::MacAddr {}
impl private::Sealed for crate::postgres::types::MacAddr8 {}
impl private::Sealed for crate::postgres::types::Point {}
impl private::Sealed for crate::postgres::types::LineString {}
impl private::Sealed for crate::postgres::types::Rect {}
impl private::Sealed for crate::postgres::types::BitString {}
impl private::Sealed for crate::postgres::types::Line {}
impl private::Sealed for crate::postgres::types::LineSegment {}
impl private::Sealed for crate::postgres::types::Polygon {}
impl private::Sealed for crate::postgres::types::Circle {}
impl private::Sealed for crate::postgres::types::Enum {}

impl DataType for crate::postgres::types::Int2 {}
impl DataType for crate::postgres::types::Int4 {}
impl DataType for crate::postgres::types::Int8 {}
impl DataType for crate::postgres::types::Float4 {}
impl DataType for crate::postgres::types::Float8 {}
impl DataType for crate::postgres::types::Varchar {}
impl DataType for crate::postgres::types::Text {}
impl DataType for crate::postgres::types::Char {}
impl DataType for crate::postgres::types::Bytea {}
impl DataType for crate::postgres::types::Boolean {}
impl DataType for crate::postgres::types::Timestamptz {}
impl DataType for crate::postgres::types::Timestamp {}
impl DataType for crate::postgres::types::Date {}
impl DataType for crate::postgres::types::Time {}
impl DataType for crate::postgres::types::Timetz {}
impl DataType for crate::postgres::types::Numeric {}
impl DataType for crate::postgres::types::Uuid {}
impl DataType for crate::postgres::types::Json {}
impl DataType for crate::postgres::types::Jsonb {}
impl DataType for crate::postgres::types::Any {}
impl DataType for crate::postgres::types::Interval {}
impl DataType for crate::postgres::types::Inet {}
impl DataType for crate::postgres::types::Cidr {}
impl DataType for crate::postgres::types::MacAddr {}
impl DataType for crate::postgres::types::MacAddr8 {}
impl DataType for crate::postgres::types::Point {}
impl DataType for crate::postgres::types::LineString {}
impl DataType for crate::postgres::types::Rect {}
impl DataType for crate::postgres::types::BitString {}
impl DataType for crate::postgres::types::Line {}
impl DataType for crate::postgres::types::LineSegment {}
impl DataType for crate::postgres::types::Polygon {}
impl DataType for crate::postgres::types::Circle {}
impl DataType for crate::postgres::types::Enum {}

impl Numeric for crate::postgres::types::Int2 {}
impl Numeric for crate::postgres::types::Int4 {}
impl Numeric for crate::postgres::types::Int8 {}
impl Numeric for crate::postgres::types::Float4 {}
impl Numeric for crate::postgres::types::Float8 {}
impl Numeric for crate::postgres::types::Numeric {}

impl Integral for crate::postgres::types::Int2 {}
impl Integral for crate::postgres::types::Int4 {}
impl Integral for crate::postgres::types::Int8 {}

impl Floating for crate::postgres::types::Float4 {}
impl Floating for crate::postgres::types::Float8 {}

impl Textual for crate::postgres::types::Varchar {}
impl Textual for crate::postgres::types::Text {}
impl Textual for crate::postgres::types::Char {}
impl Textual for crate::postgres::types::Enum {}

impl Binary for crate::postgres::types::Bytea {}

impl Temporal for crate::postgres::types::Timestamptz {}
impl Temporal for crate::postgres::types::Timestamp {}
impl Temporal for crate::postgres::types::Date {}
impl Temporal for crate::postgres::types::Time {}
impl Temporal for crate::postgres::types::Timetz {}

impl Temporal for crate::postgres::types::Interval {}

impl BooleanLike for crate::postgres::types::Boolean {}

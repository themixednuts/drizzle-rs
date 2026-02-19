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

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct SmallInt;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct Int;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct BigInt;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct Float;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct Double;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct Text;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct VarChar;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct Bool;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct Bytes;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct Date;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct Time;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct Timestamp;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct TimestampTz;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct Uuid;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct Json;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct Jsonb;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct Any;

/// PostgreSQL-style SQL array type marker.
///
/// `Array<T>` represents an array whose element SQL type is `T`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct Array<T: DataType>(pub PhantomData<T>);

impl private::Sealed for SmallInt {}
impl private::Sealed for Int {}
impl private::Sealed for BigInt {}
impl private::Sealed for Float {}
impl private::Sealed for Double {}
impl private::Sealed for Text {}
impl private::Sealed for VarChar {}
impl private::Sealed for Bool {}
impl private::Sealed for Bytes {}
impl private::Sealed for Date {}
impl private::Sealed for Time {}
impl private::Sealed for Timestamp {}
impl private::Sealed for TimestampTz {}
impl private::Sealed for Uuid {}
impl private::Sealed for Json {}
impl private::Sealed for Jsonb {}
impl private::Sealed for Any {}
impl<T: DataType> private::Sealed for Array<T> {}
impl private::Sealed for crate::sqlite::types::Integer {}
impl private::Sealed for crate::sqlite::types::Real {}
impl private::Sealed for crate::sqlite::types::Blob {}
impl private::Sealed for crate::postgres::types::Int2 {}
impl private::Sealed for crate::postgres::types::Int4 {}
impl private::Sealed for crate::postgres::types::Int8 {}
impl private::Sealed for crate::postgres::types::Float4 {}
impl private::Sealed for crate::postgres::types::Float8 {}
impl private::Sealed for crate::postgres::types::Varchar {}
impl private::Sealed for crate::postgres::types::Bytea {}
impl private::Sealed for crate::postgres::types::Boolean {}
impl private::Sealed for crate::postgres::types::Timestamptz {}

impl DataType for SmallInt {}
impl DataType for Int {}
impl DataType for BigInt {}
impl DataType for Float {}
impl DataType for Double {}
impl DataType for Text {}
impl DataType for VarChar {}
impl DataType for Bool {}
impl DataType for Bytes {}
impl DataType for Date {}
impl DataType for Time {}
impl DataType for Timestamp {}
impl DataType for TimestampTz {}
impl DataType for Uuid {}
impl DataType for Json {}
impl DataType for Jsonb {}
impl DataType for Any {}
impl<T: DataType> DataType for Array<T> {}
impl DataType for crate::sqlite::types::Integer {}
impl DataType for crate::sqlite::types::Real {}
impl DataType for crate::sqlite::types::Blob {}
impl DataType for crate::postgres::types::Int2 {}
impl DataType for crate::postgres::types::Int4 {}
impl DataType for crate::postgres::types::Int8 {}
impl DataType for crate::postgres::types::Float4 {}
impl DataType for crate::postgres::types::Float8 {}
impl DataType for crate::postgres::types::Varchar {}
impl DataType for crate::postgres::types::Bytea {}
impl DataType for crate::postgres::types::Boolean {}
impl DataType for crate::postgres::types::Timestamptz {}

impl Numeric for SmallInt {}
impl Numeric for Int {}
impl Numeric for BigInt {}
impl Numeric for Float {}
impl Numeric for Double {}
impl Numeric for Any {}
impl Numeric for crate::sqlite::types::Integer {}
impl Numeric for crate::sqlite::types::Real {}
impl Numeric for crate::postgres::types::Int2 {}
impl Numeric for crate::postgres::types::Int4 {}
impl Numeric for crate::postgres::types::Int8 {}
impl Numeric for crate::postgres::types::Float4 {}
impl Numeric for crate::postgres::types::Float8 {}

impl Integral for SmallInt {}
impl Integral for Int {}
impl Integral for BigInt {}
impl Integral for crate::sqlite::types::Integer {}
impl Integral for crate::postgres::types::Int2 {}
impl Integral for crate::postgres::types::Int4 {}
impl Integral for crate::postgres::types::Int8 {}

impl Floating for Float {}
impl Floating for Double {}
impl Floating for crate::sqlite::types::Real {}
impl Floating for crate::postgres::types::Float4 {}
impl Floating for crate::postgres::types::Float8 {}

impl Textual for Text {}
impl Textual for VarChar {}
impl Textual for Any {}
impl Textual for crate::postgres::types::Varchar {}

impl Binary for Bytes {}
impl Binary for crate::sqlite::types::Blob {}
impl Binary for crate::postgres::types::Bytea {}

impl Temporal for Date {}
impl Temporal for Time {}
impl Temporal for Timestamp {}
impl Temporal for TimestampTz {}
impl Temporal for crate::postgres::types::Timestamptz {}

//! SQL data type markers for compile-time type safety.
//!
//! This module provides zero-sized type markers that represent SQL data types
//! at the Rust type level, enabling the type system to verify compatible
//! comparisons and operations at compile time.
//!
//! # Type Hierarchy
//!
//! ```text
//! DataType (base trait)
//! ├── Numeric (arithmetic operations)
//! │   ├── Integral (SMALLINT, INTEGER, BIGINT)
//! │   └── Floating (REAL, DOUBLE PRECISION)
//! ├── Textual (TEXT, VARCHAR)
//! ├── Binary (BLOB, BYTEA)
//! ├── Bool
//! ├── Temporal (DATE, TIME, TIMESTAMP)
//! ├── Uuid
//! └── Json
//! ```
//!
//! # Example
//!
//! ```ignore
//! use drizzle_core::types::{Int, Text, Compatible};
//!
//! // Int is compatible with Int (reflexive)
//! fn compare<L: Compatible<R>, R>() {}
//! compare::<Int, Int>(); // OK
//!
//! // Int is NOT compatible with Text
//! // compare::<Int, Text>(); // Compile error!
//! ```

mod coerce;
mod ops;

pub use coerce::*;
pub use ops::*;

// =============================================================================
// Sealed Trait Pattern
// =============================================================================

mod private {
    pub trait Sealed {}
}

// =============================================================================
// Type Category Traits
// =============================================================================

/// Represents a SQL data type at the type level.
///
/// This is the base trait for all SQL type markers. It is sealed to prevent
/// external implementations, ensuring only the predefined SQL types can be used.
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

// =============================================================================
// Concrete Type Markers (Zero-Sized Types)
// =============================================================================

/// SQL SMALLINT type marker (16-bit signed integer).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct SmallInt;

/// SQL INTEGER type marker (32-bit signed integer).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct Int;

/// SQL BIGINT type marker (64-bit signed integer).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct BigInt;

/// SQL REAL/FLOAT type marker (32-bit floating point).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct Float;

/// SQL DOUBLE PRECISION type marker (64-bit floating point).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct Double;

/// SQL TEXT type marker (variable-length string).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct Text;

/// SQL VARCHAR type marker (variable-length string with limit).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct VarChar;

/// SQL BOOLEAN type marker.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct Bool;

/// SQL BLOB/BYTEA type marker (binary data).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct Bytes;

/// SQL DATE type marker.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct Date;

/// SQL TIME type marker.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct Time;

/// SQL TIMESTAMP type marker.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct Timestamp;

/// SQL TIMESTAMP WITH TIME ZONE type marker (PostgreSQL).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct TimestampTz;

/// SQL UUID type marker.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct Uuid;

/// SQL JSON type marker.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct Json;

/// SQL JSONB type marker (PostgreSQL binary JSON).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct Jsonb;

/// Dynamic/unknown SQL type marker.
///
/// Compatible with any other type, useful for raw SQL expressions
/// or when the specific type is not known at compile time.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct Any;

// =============================================================================
// Sealed Implementations
// =============================================================================

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

// =============================================================================
// DataType Implementations
// =============================================================================

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

// =============================================================================
// Numeric Implementations
// =============================================================================

impl Numeric for SmallInt {}
impl Numeric for Int {}
impl Numeric for BigInt {}
impl Numeric for Float {}
impl Numeric for Double {}
impl Numeric for Any {}

// =============================================================================
// Integral Implementations
// =============================================================================

impl Integral for SmallInt {}
impl Integral for Int {}
impl Integral for BigInt {}

// =============================================================================
// Floating Implementations
// =============================================================================

impl Floating for Float {}
impl Floating for Double {}

// =============================================================================
// Textual Implementations
// =============================================================================

impl Textual for Text {}
impl Textual for VarChar {}
impl Textual for Any {}

// =============================================================================
// Binary Implementations
// =============================================================================

impl Binary for Bytes {}

// =============================================================================
// Temporal Implementations
// =============================================================================

impl Temporal for Date {}
impl Temporal for Time {}
impl Temporal for Timestamp {}
impl Temporal for TimestampTz {}

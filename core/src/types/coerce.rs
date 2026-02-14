//! Type compatibility/coercion rules for SQL types.
//!
//! This module defines which SQL types can be compared or coerced to each other,
//! following SQL's implicit type coercion rules.

use super::{
    Any, BigInt, Bool, Bytes, DataType, Date, Double, Float, Int, Json, Jsonb, SmallInt, Text,
    Time, Timestamp, TimestampTz, Uuid, VarChar,
};

/// Marker trait indicating two SQL types can be compared or implicitly coerced.
///
/// This follows SQL's implicit coercion rules:
/// - All integers can compare with each other (SMALLINT, INTEGER, BIGINT)
/// - All floats can compare with each other (REAL, DOUBLE PRECISION)
/// - Integers can compare with floats (widening conversion)
/// - Text types compare with text types
/// - Each type compares with itself (reflexive)
///
/// Types that are NOT compatible will cause a compile-time error:
/// - INTEGER cannot compare with TEXT
/// - BOOLEAN cannot compare with INTEGER
///
/// # Example
///
/// ```ignore
/// use drizzle_core::types::{Int, Text, Compatible};
///
/// fn requires_compatible<L: Compatible<R>, R: DataType>() {}
///
/// requires_compatible::<Int, Int>();      // OK - same type
/// requires_compatible::<Int, BigInt>();   // OK - integer family
/// requires_compatible::<Int, Float>();    // OK - int to float
/// // requires_compatible::<Int, Text>(); // ERROR - incompatible
/// ```
///
/// This trait is analogous to `PartialEq<Rhs>` but for SQL type compatibility.
#[diagnostic::on_unimplemented(
    message = "SQL type `{Self}` is not compatible with `{Rhs}`",
    label = "these SQL types cannot be compared or coerced",
    note = "compatible types include: integers with integers/floats, text with text/varchar, and any type with itself"
)]
pub trait Compatible<Rhs: DataType = Self>: DataType {}

// =============================================================================
// Self-Compatibility (Reflexive)
// Every type is compatible with itself.
// =============================================================================

impl<T: DataType> Compatible<T> for T {}

// =============================================================================
// Integer Family Cross-Compatibility
// SMALLINT <-> INTEGER <-> BIGINT
// =============================================================================

impl Compatible<Int> for SmallInt {}
impl Compatible<BigInt> for SmallInt {}
impl Compatible<SmallInt> for Int {}
impl Compatible<BigInt> for Int {}
impl Compatible<SmallInt> for BigInt {}
impl Compatible<Int> for BigInt {}

// =============================================================================
// Floating-Point Family Cross-Compatibility
// REAL <-> DOUBLE PRECISION
// =============================================================================

impl Compatible<Double> for Float {}
impl Compatible<Float> for Double {}

// =============================================================================
// Integer to Floating (Widening Coercion)
// SQL allows comparing integers with floating-point numbers.
// =============================================================================

// SmallInt -> Float/Double
impl Compatible<Float> for SmallInt {}
impl Compatible<Double> for SmallInt {}

// Int -> Float/Double
impl Compatible<Float> for Int {}
impl Compatible<Double> for Int {}

// BigInt -> Float/Double
impl Compatible<Float> for BigInt {}
impl Compatible<Double> for BigInt {}

// =============================================================================
// Floating to Integer (for comparing float literals with int columns)
// e.g., `WHERE int_column > 3.5`
// =============================================================================

// Float -> SmallInt/Int/BigInt
impl Compatible<SmallInt> for Float {}
impl Compatible<Int> for Float {}
impl Compatible<BigInt> for Float {}

// Double -> SmallInt/Int/BigInt
impl Compatible<SmallInt> for Double {}
impl Compatible<Int> for Double {}
impl Compatible<BigInt> for Double {}

// =============================================================================
// Text Family Cross-Compatibility
// TEXT <-> VARCHAR
// =============================================================================

impl Compatible<VarChar> for Text {}
impl Compatible<Text> for VarChar {}

// =============================================================================
// JSON Family Cross-Compatibility (PostgreSQL)
// JSON <-> JSONB
// =============================================================================

impl Compatible<Jsonb> for Json {}
impl Compatible<Json> for Jsonb {}

// =============================================================================
// Timestamp Compatibility
// TIMESTAMP <-> TIMESTAMP WITH TIME ZONE
// =============================================================================

impl Compatible<TimestampTz> for Timestamp {}
impl Compatible<Timestamp> for TimestampTz {}

// =============================================================================
// Any Type - Compatible with Everything
// =============================================================================

impl Compatible<SmallInt> for Any {}
impl Compatible<Int> for Any {}
impl Compatible<BigInt> for Any {}
impl Compatible<Float> for Any {}
impl Compatible<Double> for Any {}
impl Compatible<Text> for Any {}
impl Compatible<VarChar> for Any {}
impl Compatible<Bool> for Any {}
impl Compatible<Bytes> for Any {}
impl Compatible<Date> for Any {}
impl Compatible<Time> for Any {}
impl Compatible<Timestamp> for Any {}
impl Compatible<TimestampTz> for Any {}
impl Compatible<Uuid> for Any {}
impl Compatible<Json> for Any {}
impl Compatible<Jsonb> for Any {}

// Reverse: All types are compatible with Any
impl Compatible<Any> for SmallInt {}
impl Compatible<Any> for Int {}
impl Compatible<Any> for BigInt {}
impl Compatible<Any> for Float {}
impl Compatible<Any> for Double {}
impl Compatible<Any> for Text {}
impl Compatible<Any> for VarChar {}
impl Compatible<Any> for Bool {}
impl Compatible<Any> for Bytes {}
impl Compatible<Any> for Date {}
impl Compatible<Any> for Time {}
impl Compatible<Any> for Timestamp {}
impl Compatible<Any> for TimestampTz {}
impl Compatible<Any> for Uuid {}
impl Compatible<Any> for Json {}
impl Compatible<Any> for Jsonb {}

// =============================================================================
// UUID Cross-Compatibility with Text
// UUIDs are often represented as text strings (e.g., "550e8400-e29b-41d4-a716-446655440000")
// =============================================================================

impl Compatible<Text> for Uuid {}
impl Compatible<VarChar> for Uuid {}
impl Compatible<Uuid> for Text {}
impl Compatible<Uuid> for VarChar {}

// =============================================================================
// Note: The following are NOT implemented (intentionally incompatible):
// - Integer <-> Text
// - Boolean <-> Integer
// - Date <-> Time
// - Binary <-> Text
// =============================================================================

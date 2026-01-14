//! Arithmetic operation result types.
//!
//! This module defines the result types for arithmetic operations between
//! SQL numeric types, following SQL's type promotion rules.

use super::{BigInt, Double, Float, Int, Numeric, SmallInt};

/// Computes the result type of arithmetic operations between two SQL numeric types.
///
/// This follows SQL's type promotion rules:
/// - Integer + Integer = Integer (largest type wins)
/// - Float + Float = Float (largest type wins)
/// - Integer + Float = Float (float wins)
///
/// Similar to `std::ops::Add::Output` but for SQL type arithmetic.
///
/// # Example
///
/// ```ignore
/// use drizzle_core::types::{Int, BigInt, Float, ArithmeticOutput};
///
/// // Int + Int = Int
/// fn result_type<L: ArithmeticOutput<R>, R: Numeric>() -> L::Output { todo!() }
///
/// // Int + BigInt = BigInt (larger wins)
/// // Int + Float = Float (float wins)
/// ```
pub trait ArithmeticOutput<Rhs: Numeric = Self>: Numeric {
    /// The resulting type of the arithmetic operation.
    type Output: Numeric;
}

// =============================================================================
// Integer Arithmetic (Largest Type Wins)
// =============================================================================

// SmallInt operations
impl ArithmeticOutput<SmallInt> for SmallInt {
    type Output = SmallInt;
}
impl ArithmeticOutput<Int> for SmallInt {
    type Output = Int;
}
impl ArithmeticOutput<BigInt> for SmallInt {
    type Output = BigInt;
}

// Int operations
impl ArithmeticOutput<SmallInt> for Int {
    type Output = Int;
}
impl ArithmeticOutput<Int> for Int {
    type Output = Int;
}
impl ArithmeticOutput<BigInt> for Int {
    type Output = BigInt;
}

// BigInt operations
impl ArithmeticOutput<SmallInt> for BigInt {
    type Output = BigInt;
}
impl ArithmeticOutput<Int> for BigInt {
    type Output = BigInt;
}
impl ArithmeticOutput<BigInt> for BigInt {
    type Output = BigInt;
}

// =============================================================================
// Float Arithmetic (Largest Type Wins)
// =============================================================================

impl ArithmeticOutput<Float> for Float {
    type Output = Float;
}
impl ArithmeticOutput<Double> for Float {
    type Output = Double;
}
impl ArithmeticOutput<Float> for Double {
    type Output = Double;
}
impl ArithmeticOutput<Double> for Double {
    type Output = Double;
}

// =============================================================================
// Mixed Integer/Float Arithmetic (Float Wins)
// =============================================================================

// SmallInt + Float/Double
impl ArithmeticOutput<Float> for SmallInt {
    type Output = Float;
}
impl ArithmeticOutput<Double> for SmallInt {
    type Output = Double;
}

// Int + Float/Double
impl ArithmeticOutput<Float> for Int {
    type Output = Float;
}
impl ArithmeticOutput<Double> for Int {
    type Output = Double;
}

// BigInt + Float/Double
impl ArithmeticOutput<Float> for BigInt {
    type Output = Float;
}
impl ArithmeticOutput<Double> for BigInt {
    type Output = Double;
}

// Float + Integer (reverse direction)
impl ArithmeticOutput<SmallInt> for Float {
    type Output = Float;
}
impl ArithmeticOutput<Int> for Float {
    type Output = Float;
}
impl ArithmeticOutput<BigInt> for Float {
    type Output = Float;
}

// Double + Integer (reverse direction)
impl ArithmeticOutput<SmallInt> for Double {
    type Output = Double;
}
impl ArithmeticOutput<Int> for Double {
    type Output = Double;
}
impl ArithmeticOutput<BigInt> for Double {
    type Output = Double;
}

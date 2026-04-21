//! Runtime value-conversion helpers shared across dialects.
//!
//! Helpers here are intentionally small, dialect-agnostic primitives used by
//! per-dialect `FromXValue` implementations.

use crate::error::DrizzleError;
use crate::prelude::format;

/// Convert an `f64` database value to an integer type `T`, rejecting
/// non-finite, non-integral, and out-of-range inputs with a descriptive error.
///
/// The `type_name` is embedded in the error for context (e.g. `"i64"`, `"i32"`).
///
/// # Errors
///
/// Returns [`DrizzleError::ConversionError`] when:
/// - `value` is not finite (NaN, ±∞).
/// - `value` has a non-zero fractional part.
/// - `value` is outside the representable range of `T` (or of `i128`).
pub fn checked_float_to_int<T>(value: f64, type_name: &str) -> Result<T, DrizzleError>
where
    T: TryFrom<i128>,
    <T as TryFrom<i128>>::Error: core::fmt::Display,
{
    if !value.is_finite() {
        return Err(DrizzleError::ConversionError(
            format!("cannot convert non-finite float {value} to {type_name}").into(),
        ));
    }

    if value % 1.0 != 0.0 {
        return Err(DrizzleError::ConversionError(
            format!("cannot convert non-integer float {value} to {type_name}").into(),
        ));
    }

    // `2^127` is exactly representable in f64 and equals `i128::MAX + 1`
    // (and `-2^127` equals `i128::MIN`). Building the bounds directly in f64
    // space avoids a precision-losing `i128 as f64` cast. The bit pattern
    // encodes sign=0, biased_exp=127+1023=1150, mantissa=0.
    const TWO_POW_127: f64 = f64::from_bits(0x47E0_0000_0000_0000);
    let two_pow_127 = TWO_POW_127;
    if value < -two_pow_127 || value >= two_pow_127 {
        return Err(DrizzleError::ConversionError(
            format!("float {value} out of range for {type_name}").into(),
        ));
    }

    // Safe: `value` is finite, integer-valued, and strictly within `[-2^127, 2^127)`.
    // Round-trip through a decimal string to avoid an f64-as-i128 cast warning.
    let int_value: i128 = format!("{value:.0}").parse().map_err(|e| {
        DrizzleError::ConversionError(
            format!("float {value} could not be parsed as integer for {type_name}: {e}").into(),
        )
    })?;
    int_value.try_into().map_err(|e| {
        DrizzleError::ConversionError(
            format!("float {value} out of range for {type_name}: {e}").into(),
        )
    })
}

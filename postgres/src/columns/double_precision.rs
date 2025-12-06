//! PostgreSQL DOUBLE PRECISION column builder.

use core::marker::PhantomData;

/// Builder for PostgreSQL DOUBLE PRECISION columns.
///
/// DOUBLE PRECISION (or FLOAT8) stores 64-bit IEEE 754 floating-point numbers.
/// Provides approximately 15 decimal digits of precision.
///
/// See: <https://www.postgresql.org/docs/current/datatype-numeric.html#DATATYPE-FLOAT>
///
/// # REAL vs DOUBLE PRECISION
///
/// - `REAL` (FLOAT4) = 32-bit, ~6 decimal digits
/// - `DOUBLE PRECISION` (FLOAT8) = 64-bit, ~15 decimal digits
#[derive(Debug, Clone, Copy)]
pub struct DoublePrecisionBuilder<T> {
    _marker: PhantomData<T>,
    /// Whether this column has a NOT NULL constraint.
    pub is_not_null: bool,
    /// Whether this column has any default value.
    pub has_default: bool,
}

impl<T> DoublePrecisionBuilder<T> {
    /// Creates a new DOUBLE PRECISION column builder with no constraints.
    #[inline]
    pub const fn new() -> Self {
        Self {
            _marker: PhantomData,
            is_not_null: false,
            has_default: false,
        }
    }

    /// Adds a NOT NULL constraint to this column.
    #[inline]
    pub const fn not_null(self) -> Self {
        Self {
            is_not_null: true,
            ..self
        }
    }

    /// Sets a compile-time default value for this column.
    #[inline]
    pub const fn default(self, _value: f64) -> Self {
        Self {
            has_default: true,
            ..self
        }
    }

    /// Marks this column as having a Rust function to generate default values at runtime.
    #[inline]
    pub const fn has_default_fn(self) -> Self {
        Self {
            has_default: true,
            ..self
        }
    }
}

/// Creates a DOUBLE PRECISION column builder.
///
/// DOUBLE PRECISION stores 64-bit floating-point numbers.
///
/// See: <https://www.postgresql.org/docs/current/datatype-numeric.html#DATATYPE-FLOAT>
#[inline]
pub const fn double_precision<T>() -> DoublePrecisionBuilder<T> {
    DoublePrecisionBuilder::new()
}

/// Alias for DOUBLE PRECISION.
#[inline]
pub const fn float8<T>() -> DoublePrecisionBuilder<T> {
    DoublePrecisionBuilder::new()
}

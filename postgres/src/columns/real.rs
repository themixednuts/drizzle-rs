//! PostgreSQL REAL column builder.

use core::marker::PhantomData;

/// Builder for PostgreSQL REAL columns.
///
/// REAL (or FLOAT4) stores single-precision 32-bit floating-point numbers.
///
/// See: <https://www.postgresql.org/docs/current/datatype-numeric.html#DATATYPE-FLOAT>
///
/// # REAL vs DOUBLE PRECISION
///
/// - `REAL` = 32-bit (6 decimal digits precision)
/// - `DOUBLE PRECISION` = 64-bit (15 decimal digits precision)
#[derive(Debug, Clone, Copy)]
pub struct RealBuilder<T> {
    _marker: PhantomData<T>,
    /// Whether this column has a NOT NULL constraint.
    pub is_not_null: bool,
    /// Whether this column has any default value.
    pub has_default: bool,
}

impl<T> RealBuilder<T> {
    /// Creates a new REAL column builder with no constraints.
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
    pub const fn default(self, _value: f32) -> Self {
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

/// Creates a REAL column builder.
///
/// REAL stores 32-bit floating-point numbers.
///
/// See: <https://www.postgresql.org/docs/current/datatype-numeric.html#DATATYPE-FLOAT>
#[inline]
pub const fn real<T>() -> RealBuilder<T> {
    RealBuilder::new()
}

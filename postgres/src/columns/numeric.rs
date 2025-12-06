//! PostgreSQL NUMERIC column builder.

use core::marker::PhantomData;

/// Builder for PostgreSQL NUMERIC columns.
///
/// NUMERIC (or DECIMAL) stores exact numbers with user-specified precision.
/// Unlike floating-point types, NUMERIC stores values exactly without rounding errors.
///
/// See: <https://www.postgresql.org/docs/current/datatype-numeric.html#DATATYPE-NUMERIC-DECIMAL>
///
/// # Use Cases
///
/// Use NUMERIC for:
/// - Money/currency (exact calculations)
/// - Scientific measurements requiring precision
/// - Any case where floating-point rounding is unacceptable
#[derive(Debug, Clone, Copy)]
pub struct NumericBuilder<T> {
    _marker: PhantomData<T>,
    /// Whether this column has a NOT NULL constraint.
    pub is_not_null: bool,
    /// Precision (total digits) - 0 means unlimited.
    pub precision: u16,
    /// Scale (digits after decimal) - 0 means no fractional part.
    pub scale: u16,
    /// Whether this column has any default value.
    pub has_default: bool,
}

impl<T> NumericBuilder<T> {
    /// Creates a new NUMERIC column builder with no constraints.
    #[inline]
    pub const fn new() -> Self {
        Self {
            _marker: PhantomData,
            is_not_null: false,
            precision: 0,
            scale: 0,
            has_default: false,
        }
    }

    /// Sets the precision and scale for this NUMERIC column.
    ///
    /// - `precision`: Total number of digits (before and after decimal point)
    /// - `scale`: Number of digits after the decimal point
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// #[numeric(precision = 10, scale = 2)]
    /// price: Decimal,  // SQL: price NUMERIC(10, 2)
    /// ```
    #[inline]
    pub const fn precision_scale(self, precision: u16, scale: u16) -> Self {
        Self {
            precision,
            scale,
            ..self
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

    /// Marks this column as having a Rust function to generate default values at runtime.
    #[inline]
    pub const fn has_default_fn(self) -> Self {
        Self {
            has_default: true,
            ..self
        }
    }
}

/// Creates a NUMERIC column builder.
///
/// NUMERIC stores exact numbers with arbitrary precision.
///
/// See: <https://www.postgresql.org/docs/current/datatype-numeric.html#DATATYPE-NUMERIC-DECIMAL>
#[inline]
pub const fn numeric<T>() -> NumericBuilder<T> {
    NumericBuilder::new()
}

/// Alias for NUMERIC (PostgreSQL treats DECIMAL as equivalent).
#[inline]
pub const fn decimal<T>() -> NumericBuilder<T> {
    NumericBuilder::new()
}

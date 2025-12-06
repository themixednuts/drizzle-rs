//! PostgreSQL BIGINT column builder.

use core::marker::PhantomData;

/// Builder for PostgreSQL BIGINT columns.
///
/// BIGINT (or INT8) stores 64-bit signed integers.
/// Range: -9223372036854775808 to 9223372036854775807
///
/// See: <https://www.postgresql.org/docs/current/datatype-numeric.html#DATATYPE-INT>
///
/// # When to use BIGINT vs INTEGER
///
/// Use BIGINT when:
/// - You need to store values larger than 2 billion
/// - You're counting things that might exceed INTEGER range (e.g., unique IDs at scale)
/// - You're storing Unix timestamps in milliseconds
#[derive(Debug, Clone, Copy)]
pub struct BigintBuilder<T> {
    _marker: PhantomData<T>,
    /// Whether this column is the primary key.
    pub is_primary: bool,
    /// Whether this column has a UNIQUE constraint.
    pub is_unique: bool,
    /// Whether this column has a NOT NULL constraint.
    pub is_not_null: bool,
    /// Whether this column has any default value.
    pub has_default: bool,
}

impl<T> BigintBuilder<T> {
    /// Creates a new BIGINT column builder with no constraints.
    #[inline]
    pub const fn new() -> Self {
        Self {
            _marker: PhantomData,
            is_primary: false,
            is_unique: false,
            is_not_null: false,
            has_default: false,
        }
    }

    /// Makes this column the PRIMARY KEY.
    ///
    /// See: <https://www.postgresql.org/docs/current/ddl-constraints.html#DDL-CONSTRAINTS-PRIMARY-KEYS>
    #[inline]
    pub const fn primary(self) -> Self {
        Self {
            is_primary: true,
            is_not_null: true,
            ..self
        }
    }

    /// Adds a UNIQUE constraint to this column.
    #[inline]
    pub const fn unique(self) -> Self {
        Self {
            is_unique: true,
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

    /// Sets a compile-time default value for this column.
    #[inline]
    pub const fn default(self, _value: i64) -> Self {
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

/// Creates a BIGINT column builder.
///
/// BIGINT stores 64-bit signed integers.
///
/// See: <https://www.postgresql.org/docs/current/datatype-numeric.html#DATATYPE-INT>
#[inline]
pub const fn bigint<T>() -> BigintBuilder<T> {
    BigintBuilder::new()
}

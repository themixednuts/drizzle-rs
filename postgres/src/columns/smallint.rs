//! PostgreSQL SMALLINT column builder.

use core::marker::PhantomData;

/// Builder for PostgreSQL SMALLINT columns.
///
/// SMALLINT (or INT2) stores 16-bit signed integers (-32768 to 32767).
///
/// See: <https://www.postgresql.org/docs/current/datatype-numeric.html#DATATYPE-INT>
///
/// # When to use SMALLINT
///
/// Use SMALLINT when:
/// - You're storing small numbers that fit in 16 bits
/// - Storage space is a concern (2 bytes vs 4 for INTEGER)
/// - Examples: age, year, small counts
#[derive(Debug, Clone, Copy)]
pub struct SmallintBuilder<T> {
    _marker: PhantomData<T>,
    /// Whether this column is the primary key.
    pub is_primary: bool,
    /// Whether this column has a UNIQUE constraint.
    pub is_unique: bool,
    /// Whether this column has a NOT NULL constraint.
    pub is_not_null: bool,
    /// Whether this column stores enum discriminants.
    pub is_enum: bool,
    /// Whether this column has any default value.
    pub has_default: bool,
}

impl<T> SmallintBuilder<T> {
    /// Creates a new SMALLINT column builder with no constraints.
    #[inline]
    pub const fn new() -> Self {
        Self {
            _marker: PhantomData,
            is_primary: false,
            is_unique: false,
            is_not_null: false,
            is_enum: false,
            has_default: false,
        }
    }

    /// Makes this column the PRIMARY KEY.
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

    /// Stores Rust enum discriminants as integers.
    #[inline]
    pub const fn r#enum(self) -> Self {
        Self {
            is_enum: true,
            ..self
        }
    }

    /// Sets a compile-time default value for this column.
    #[inline]
    pub const fn default(self, _value: i16) -> Self {
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

/// Creates a SMALLINT column builder.
///
/// SMALLINT stores 16-bit signed integers.
///
/// See: <https://www.postgresql.org/docs/current/datatype-numeric.html#DATATYPE-INT>
#[inline]
pub const fn smallint<T>() -> SmallintBuilder<T> {
    SmallintBuilder::new()
}

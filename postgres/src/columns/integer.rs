//! PostgreSQL INTEGER column builder.

use core::marker::PhantomData;

/// Builder for PostgreSQL INTEGER columns.
///
/// INTEGER (or INT4) stores 32-bit signed integers (-2147483648 to 2147483647).
///
/// See: <https://www.postgresql.org/docs/current/datatype-numeric.html#DATATYPE-INT>
///
/// # Auto-increment
///
/// For auto-increment behavior, use `serial()` instead - it's a shorthand for
/// `INTEGER` with an auto-created sequence.
#[derive(Debug, Clone, Copy)]
pub struct IntegerBuilder<T> {
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

impl<T> IntegerBuilder<T> {
    /// Creates a new INTEGER column builder with no constraints.
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
    ///
    /// See: <https://www.postgresql.org/docs/current/ddl-constraints.html#DDL-CONSTRAINTS-UNIQUE-CONSTRAINTS>
    #[inline]
    pub const fn unique(self) -> Self {
        Self {
            is_unique: true,
            ..self
        }
    }

    /// Adds a NOT NULL constraint to this column.
    ///
    /// See: <https://www.postgresql.org/docs/current/ddl-constraints.html#id-1.5.4.6.6>
    #[inline]
    pub const fn not_null(self) -> Self {
        Self {
            is_not_null: true,
            ..self
        }
    }

    /// Stores Rust enum discriminants as integers.
    ///
    /// The enum type must implement `Into<i32>` and `TryFrom<i32>`.
    #[inline]
    pub const fn r#enum(self) -> Self {
        Self {
            is_enum: true,
            ..self
        }
    }

    /// Sets a compile-time default value for this column.
    ///
    /// See: <https://www.postgresql.org/docs/current/ddl-default.html>
    #[inline]
    pub const fn default(self, _value: i32) -> Self {
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

/// Creates an INTEGER column builder.
///
/// INTEGER stores 32-bit signed integers.
///
/// See: <https://www.postgresql.org/docs/current/datatype-numeric.html#DATATYPE-INT>
#[inline]
pub const fn integer<T>() -> IntegerBuilder<T> {
    IntegerBuilder::new()
}

//! PostgreSQL BYTEA column builder.

use core::marker::PhantomData;

/// Builder for PostgreSQL BYTEA columns.
///
/// BYTEA stores variable-length binary data ("byte array").
///
/// See: <https://www.postgresql.org/docs/current/datatype-binary.html>
///
/// # Storage
///
/// BYTEA uses a hex or escape format for input/output. The `hex` format
/// is more compact and is the default since PostgreSQL 9.0.
#[derive(Debug, Clone, Copy)]
pub struct ByteaBuilder<T> {
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

impl<T> ByteaBuilder<T> {
    /// Creates a new BYTEA column builder with no constraints.
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

    /// Marks this column as having a Rust function to generate default values at runtime.
    #[inline]
    pub const fn has_default_fn(self) -> Self {
        Self {
            has_default: true,
            ..self
        }
    }
}

/// Creates a BYTEA column builder.
///
/// BYTEA stores variable-length binary data.
///
/// See: <https://www.postgresql.org/docs/current/datatype-binary.html>
#[inline]
pub const fn bytea<T>() -> ByteaBuilder<T> {
    ByteaBuilder::new()
}

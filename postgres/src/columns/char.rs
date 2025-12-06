//! PostgreSQL CHAR column builder.

use core::marker::PhantomData;

/// Builder for PostgreSQL CHAR columns.
///
/// CHAR(n) stores fixed-length strings, padded with spaces to the specified length.
///
/// See: <https://www.postgresql.org/docs/current/datatype-character.html>
///
/// # CHAR vs VARCHAR vs TEXT
///
/// - `CHAR(n)` - Fixed length, space-padded, rarely used in modern PostgreSQL
/// - `VARCHAR(n)` - Variable length with limit
/// - `TEXT` - Variable length without limit
///
/// **Note:** According to PostgreSQL docs, there's no performance difference
/// between these types, so use VARCHAR or TEXT unless you specifically need
/// fixed-length storage semantics.
#[derive(Debug, Clone, Copy)]
pub struct CharBuilder<T> {
    _marker: PhantomData<T>,
    /// Whether this column is the primary key.
    pub is_primary: bool,
    /// Whether this column has a UNIQUE constraint.
    pub is_unique: bool,
    /// Whether this column has a NOT NULL constraint.
    pub is_not_null: bool,
    /// Fixed length for CHAR(n).
    pub length: usize,
    /// Whether this column has any default value.
    pub has_default: bool,
}

impl<T> CharBuilder<T> {
    /// Creates a new CHAR column builder with no constraints.
    #[inline]
    pub const fn new() -> Self {
        Self {
            _marker: PhantomData,
            is_primary: false,
            is_unique: false,
            is_not_null: false,
            length: 1,
            has_default: false,
        }
    }

    /// Sets the fixed length for this CHAR column.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// #[char(length = 2)]
    /// country_code: String,  // SQL: country_code CHAR(2)
    /// ```
    #[inline]
    pub const fn length(self, n: usize) -> Self {
        Self { length: n, ..self }
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

    /// Sets a compile-time default value for this column.
    #[inline]
    pub const fn default(self, _value: &'static str) -> Self {
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

/// Creates a CHAR column builder.
///
/// CHAR stores fixed-length strings, space-padded.
///
/// See: <https://www.postgresql.org/docs/current/datatype-character.html>
#[inline]
pub const fn char<T>() -> CharBuilder<T> {
    CharBuilder::new()
}

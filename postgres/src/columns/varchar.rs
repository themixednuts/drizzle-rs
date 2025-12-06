//! PostgreSQL VARCHAR column builder.

use core::marker::PhantomData;

/// Builder for PostgreSQL VARCHAR columns.
///
/// VARCHAR (character varying) stores variable-length strings with an optional maximum length.
///
/// See: <https://www.postgresql.org/docs/current/datatype-character.html>
///
/// # VARCHAR vs TEXT
///
/// In PostgreSQL, VARCHAR (without a length) is identical to TEXT.
/// Use VARCHAR when you need to enforce a maximum length; use TEXT otherwise.
#[derive(Debug, Clone, Copy)]
pub struct VarcharBuilder<T> {
    _marker: PhantomData<T>,
    /// Whether this column is the primary key.
    pub is_primary: bool,
    /// Whether this column has a UNIQUE constraint.
    pub is_unique: bool,
    /// Whether this column has a NOT NULL constraint.
    pub is_not_null: bool,
    /// Maximum length (VARCHAR(n)) or 0 for unlimited.
    pub length: usize,
    /// Whether this column has any default value.
    pub has_default: bool,
}

impl<T> VarcharBuilder<T> {
    /// Creates a new VARCHAR column builder with no length limit.
    #[inline]
    pub const fn new() -> Self {
        Self {
            _marker: PhantomData,
            is_primary: false,
            is_unique: false,
            is_not_null: false,
            length: 0,
            has_default: false,
        }
    }

    /// Sets the maximum length for this VARCHAR column.
    ///
    /// This creates a VARCHAR(n) column that rejects values longer than n characters.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// #[varchar(length = 255)]
    /// email: String,  // SQL: email VARCHAR(255)
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

/// Creates a VARCHAR column builder.
///
/// VARCHAR stores variable-length strings. Use `.length(n)` for VARCHAR(n).
///
/// See: <https://www.postgresql.org/docs/current/datatype-character.html>
///
/// # Example
///
/// ```rust,ignore
/// const COL: VarcharBuilder<Email> = varchar::<Email>().length(255).unique();
/// ```
#[inline]
pub const fn varchar<T>() -> VarcharBuilder<T> {
    VarcharBuilder::new()
}

//! SQLite REAL column builder.

use core::marker::PhantomData;

/// Builder for SQLite REAL columns.
///
/// REAL columns store 8-byte IEEE 754 floating-point numbers (f64).
///
/// See: <https://sqlite.org/datatype3.html#storage_classes_and_datatypes>
#[derive(Debug, Clone, Copy)]
pub struct RealBuilder<T> {
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

impl<T> RealBuilder<T> {
    /// Creates a new REAL column builder with no constraints.
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
    /// Note: Using REAL as a primary key is unusual. Consider INTEGER or TEXT instead.
    ///
    /// See: <https://sqlite.org/lang_createtable.html#the_primary_key>
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
    /// See: <https://sqlite.org/lang_createtable.html#unique_constraints>
    #[inline]
    pub const fn unique(self) -> Self {
        Self {
            is_unique: true,
            ..self
        }
    }

    /// Adds a NOT NULL constraint to this column.
    ///
    /// See: <https://sqlite.org/lang_createtable.html#not_null_constraints>
    #[inline]
    pub const fn not_null(self) -> Self {
        Self {
            is_not_null: true,
            ..self
        }
    }

    /// Sets a compile-time default value for this column.
    ///
    /// See: <https://sqlite.org/lang_createtable.html#the_default_clause>
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// #[real(default = 0.0)]
    /// score: f64,  // SQL: score REAL DEFAULT 0.0
    /// ```
    #[inline]
    pub const fn default(self, _value: f64) -> Self {
        Self {
            has_default: true,
            ..self
        }
    }

    /// Marks this column as having a Rust function to generate default values at runtime.
    ///
    /// See [`TextBuilder::has_default_fn`] for detailed documentation.
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
/// REAL columns store 8-byte IEEE 754 floating-point numbers.
///
/// See: <https://sqlite.org/datatype3.html#storage_classes_and_datatypes>
#[inline]
pub const fn real<T>() -> RealBuilder<T> {
    RealBuilder::new()
}

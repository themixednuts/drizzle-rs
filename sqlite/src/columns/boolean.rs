//! SQLite BOOLEAN column builder.

use core::marker::PhantomData;

/// Builder for SQLite BOOLEAN columns.
///
/// SQLite doesn't have a native BOOLEAN type. Boolean values are stored as
/// INTEGER: 0 for false, 1 for true.
///
/// See: <https://sqlite.org/datatype3.html#boolean_datatype>
///
/// # Storage
///
/// - `false` → INTEGER 0
/// - `true` → INTEGER 1
#[derive(Debug, Clone, Copy)]
pub struct BooleanBuilder<T> {
    _marker: PhantomData<T>,
    /// Whether this column is the primary key (unusual for boolean).
    pub is_primary: bool,
    /// Whether this column has a UNIQUE constraint.
    pub is_unique: bool,
    /// Whether this column has a NOT NULL constraint.
    pub is_not_null: bool,
    /// Whether this column has any default value.
    pub has_default: bool,
}

impl<T> BooleanBuilder<T> {
    /// Creates a new BOOLEAN column builder with no constraints.
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
    /// #[boolean(default = true)]
    /// active: bool,  // SQL: active INTEGER DEFAULT 1
    /// ```
    #[inline]
    pub const fn default(self, _value: bool) -> Self {
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

/// Creates a BOOLEAN column builder.
///
/// Boolean values are stored as INTEGER (0 or 1) in SQLite.
///
/// See: <https://sqlite.org/datatype3.html#boolean_datatype>
#[inline]
pub const fn boolean<T>() -> BooleanBuilder<T> {
    BooleanBuilder::new()
}

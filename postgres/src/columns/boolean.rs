//! PostgreSQL BOOLEAN column builder.

use core::marker::PhantomData;

/// Builder for PostgreSQL BOOLEAN columns.
///
/// PostgreSQL has a native BOOLEAN type (unlike SQLite).
/// Valid values: TRUE, FALSE, or NULL.
///
/// See: <https://www.postgresql.org/docs/current/datatype-boolean.html>
#[derive(Debug, Clone, Copy)]
pub struct BooleanBuilder<T> {
    _marker: PhantomData<T>,
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
            is_not_null: false,
            has_default: false,
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

    /// Sets a compile-time default value for this column.
    ///
    /// See: <https://www.postgresql.org/docs/current/ddl-default.html>
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// #[boolean(default = true)]
    /// active: bool,  // SQL: active BOOLEAN DEFAULT TRUE
    /// ```
    #[inline]
    pub const fn default(self, _value: bool) -> Self {
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

/// Creates a BOOLEAN column builder.
///
/// PostgreSQL has a native BOOLEAN type.
///
/// See: <https://www.postgresql.org/docs/current/datatype-boolean.html>
#[inline]
pub const fn boolean<T>() -> BooleanBuilder<T> {
    BooleanBuilder::new()
}

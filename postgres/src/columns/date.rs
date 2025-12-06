//! PostgreSQL DATE column builder.

use core::marker::PhantomData;

/// Builder for PostgreSQL DATE columns.
///
/// DATE stores calendar dates (year, month, day) without time.
/// Range: 4713 BC to 5874897 AD.
///
/// See: <https://www.postgresql.org/docs/current/datatype-datetime.html>
#[derive(Debug, Clone, Copy)]
pub struct DateBuilder<T> {
    _marker: PhantomData<T>,
    /// Whether this column has a NOT NULL constraint.
    pub is_not_null: bool,
    /// Whether default is CURRENT_DATE.
    pub default_now: bool,
    /// Whether this column has any default value.
    pub has_default: bool,
}

impl<T> DateBuilder<T> {
    /// Creates a new DATE column builder with no constraints.
    #[inline]
    pub const fn new() -> Self {
        Self {
            _marker: PhantomData,
            is_not_null: false,
            default_now: false,
            has_default: false,
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

    /// Uses CURRENT_DATE as the default value.
    ///
    /// This sets the server's current date when a row is inserted.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// #[date(default_now)]
    /// created_date: chrono::NaiveDate,
    /// // SQL: created_date DATE DEFAULT CURRENT_DATE
    /// ```
    #[inline]
    pub const fn default_now(self) -> Self {
        Self {
            default_now: true,
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

/// Creates a DATE column builder.
///
/// DATE stores calendar dates without time.
///
/// See: <https://www.postgresql.org/docs/current/datatype-datetime.html>
#[inline]
pub const fn date<T>() -> DateBuilder<T> {
    DateBuilder::new()
}

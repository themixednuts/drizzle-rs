//! PostgreSQL TIMESTAMP column builder.

use core::marker::PhantomData;

/// Builder for PostgreSQL TIMESTAMP columns.
///
/// TIMESTAMP stores date and time without time zone information.
/// For time zone awareness, use TIMESTAMPTZ instead.
///
/// See: <https://www.postgresql.org/docs/current/datatype-datetime.html>
///
/// # TIMESTAMP vs TIMESTAMPTZ
///
/// - `TIMESTAMP` (without time zone) - stores what you give it verbatim
/// - `TIMESTAMPTZ` (with time zone) - converts to UTC for storage, converts to session zone on retrieval
///
/// **Recommendation:** Use TIMESTAMPTZ for most applications.
#[derive(Debug, Clone, Copy)]
pub struct TimestampBuilder<T> {
    _marker: PhantomData<T>,
    /// Whether this column has a NOT NULL constraint.
    pub is_not_null: bool,
    /// Whether this column includes time zone (TIMESTAMPTZ).
    pub with_timezone: bool,
    /// Whether default is CURRENT_TIMESTAMP.
    pub default_now: bool,
    /// Whether this column has any default value.
    pub has_default: bool,
}

impl<T> TimestampBuilder<T> {
    /// Creates a new TIMESTAMP column builder with no constraints.
    #[inline]
    pub const fn new() -> Self {
        Self {
            _marker: PhantomData,
            is_not_null: false,
            with_timezone: false,
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

    /// Uses TIMESTAMPTZ (timestamp with time zone) instead of TIMESTAMP.
    ///
    /// TIMESTAMPTZ is recommended for most use cases because it handles
    /// time zones correctly.
    ///
    /// See: <https://www.postgresql.org/docs/current/datatype-datetime.html#DATATYPE-TIMEZONES>
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// #[timestamp(with_timezone)]
    /// created_at: chrono::DateTime<chrono::Utc>,
    /// ```
    #[inline]
    pub const fn with_timezone(self) -> Self {
        Self {
            with_timezone: true,
            ..self
        }
    }

    /// Uses CURRENT_TIMESTAMP as the default value.
    ///
    /// This sets the server's current timestamp when a row is inserted.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// #[timestamp(with_timezone, default_now)]
    /// created_at: chrono::DateTime<chrono::Utc>,
    /// // SQL: created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP
    /// ```
    #[inline]
    pub const fn default_now(self) -> Self {
        Self {
            default_now: true,
            has_default: true,
            ..self
        }
    }

    /// Marks this column as having a Rust function to generate timestamp values at runtime.
    ///
    /// This generates timestamps in Rust before insert, not in PostgreSQL.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// #[timestamp(with_timezone, default_fn = chrono::Utc::now)]
    /// created_at: chrono::DateTime<chrono::Utc>,
    /// ```
    #[inline]
    pub const fn has_default_fn(self) -> Self {
        Self {
            has_default: true,
            ..self
        }
    }
}

/// Creates a TIMESTAMP column builder.
///
/// TIMESTAMP stores date and time. Use `.with_timezone()` for TIMESTAMPTZ.
///
/// See: <https://www.postgresql.org/docs/current/datatype-datetime.html>
#[inline]
pub const fn timestamp<T>() -> TimestampBuilder<T> {
    TimestampBuilder::new()
}

/// Creates a TIMESTAMPTZ column builder (shorthand for timestamp().with_timezone()).
///
/// TIMESTAMPTZ stores date and time with time zone awareness.
///
/// See: <https://www.postgresql.org/docs/current/datatype-datetime.html>
#[inline]
pub const fn timestamptz<T>() -> TimestampBuilder<T> {
    TimestampBuilder {
        _marker: PhantomData,
        is_not_null: false,
        with_timezone: true,
        default_now: false,
        has_default: false,
    }
}

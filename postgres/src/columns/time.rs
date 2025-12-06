//! PostgreSQL TIME column builder.

use core::marker::PhantomData;

/// Builder for PostgreSQL TIME columns.
///
/// TIME stores time of day without date or time zone.
/// Range: 00:00:00 to 24:00:00 with microsecond precision.
///
/// See: <https://www.postgresql.org/docs/current/datatype-datetime.html>
///
/// # TIME vs TIMETZ
///
/// - `TIME` (without time zone) - stores time only
/// - `TIMETZ` (with time zone) - stores time with zone offset
///
/// **Note:** TIMETZ is rarely useful because it stores offset, not the zone name.
/// For most applications, use TIME and handle zones in your application.
#[derive(Debug, Clone, Copy)]
pub struct TimeBuilder<T> {
    _marker: PhantomData<T>,
    /// Whether this column has a NOT NULL constraint.
    pub is_not_null: bool,
    /// Whether this column includes time zone (TIMETZ).
    pub with_timezone: bool,
    /// Whether default is CURRENT_TIME.
    pub default_now: bool,
    /// Whether this column has any default value.
    pub has_default: bool,
}

impl<T> TimeBuilder<T> {
    /// Creates a new TIME column builder with no constraints.
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

    /// Uses TIMETZ (time with time zone) instead of TIME.
    ///
    /// See: <https://www.postgresql.org/docs/current/datatype-datetime.html#DATATYPE-TIMEZONES>
    #[inline]
    pub const fn with_timezone(self) -> Self {
        Self {
            with_timezone: true,
            ..self
        }
    }

    /// Uses CURRENT_TIME as the default value.
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

/// Creates a TIME column builder.
///
/// TIME stores time of day. Use `.with_timezone()` for TIMETZ.
///
/// See: <https://www.postgresql.org/docs/current/datatype-datetime.html>
#[inline]
pub const fn time<T>() -> TimeBuilder<T> {
    TimeBuilder::new()
}

/// Creates a TIMETZ column builder (shorthand for time().with_timezone()).
///
/// TIMETZ stores time of day with time zone offset.
///
/// See: <https://www.postgresql.org/docs/current/datatype-datetime.html>
#[inline]
pub const fn timetz<T>() -> TimeBuilder<T> {
    TimeBuilder {
        _marker: PhantomData,
        is_not_null: false,
        with_timezone: true,
        default_now: false,
        has_default: false,
    }
}

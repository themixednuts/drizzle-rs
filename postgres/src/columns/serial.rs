//! PostgreSQL SERIAL column builder.

use core::marker::PhantomData;

/// Builder for PostgreSQL SERIAL columns.
///
/// SERIAL is a convenience notation for creating auto-incrementing integer columns.
/// It's equivalent to:
/// ```sql
/// CREATE SEQUENCE tablename_colname_seq;
/// CREATE TABLE tablename (colname INTEGER NOT NULL DEFAULT nextval('tablename_colname_seq'));
/// ```
///
/// See: <https://www.postgresql.org/docs/current/datatype-numeric.html#DATATYPE-SERIAL>
///
/// # SERIAL vs BIGSERIAL
///
/// - `SERIAL` = INTEGER (32-bit) with auto-increment
/// - `BIGSERIAL` = BIGINT (64-bit) with auto-increment
///
/// Use BIGSERIAL for tables that might have billions of rows.
#[derive(Debug, Clone, Copy)]
pub struct SerialBuilder<T> {
    _marker: PhantomData<T>,
    /// Whether this column is the primary key.
    pub is_primary: bool,
    /// Whether this column has a UNIQUE constraint.
    pub is_unique: bool,
    /// Whether this column has a NOT NULL constraint (SERIAL is implicitly NOT NULL).
    pub is_not_null: bool,
    /// Whether this is BIGSERIAL instead of SERIAL.
    pub is_bigserial: bool,
}

impl<T> SerialBuilder<T> {
    /// Creates a new SERIAL column builder.
    #[inline]
    pub const fn new() -> Self {
        Self {
            _marker: PhantomData,
            is_primary: false,
            is_unique: false,
            is_not_null: true, // SERIAL is implicitly NOT NULL
            is_bigserial: false,
        }
    }

    /// Makes this column the PRIMARY KEY.
    ///
    /// This is the most common use of SERIAL - as an auto-incrementing primary key.
    ///
    /// See: <https://www.postgresql.org/docs/current/ddl-constraints.html#DDL-CONSTRAINTS-PRIMARY-KEYS>
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// #[serial(primary)]
    /// id: i32,  // SQL: id SERIAL PRIMARY KEY
    /// ```
    #[inline]
    pub const fn primary(self) -> Self {
        Self {
            is_primary: true,
            is_not_null: true, // PRIMARY KEY implies NOT NULL
            ..self
        }
    }
}

/// Creates a SERIAL column builder.
///
/// SERIAL is INTEGER with auto-increment (uses a sequence).
///
/// See: <https://www.postgresql.org/docs/current/datatype-numeric.html#DATATYPE-SERIAL>
///
/// # Example
///
/// ```rust,ignore
/// #[serial(primary)]
/// id: i32,  // Auto-incrementing INTEGER primary key
/// ```
#[inline]
pub const fn serial<T>() -> SerialBuilder<T> {
    SerialBuilder::new()
}

/// Creates a BIGSERIAL column builder.
///
/// BIGSERIAL is BIGINT with auto-increment (uses a sequence).
///
/// See: <https://www.postgresql.org/docs/current/datatype-numeric.html#DATATYPE-SERIAL>
///
/// # Example
///
/// ```rust,ignore
/// #[bigserial(primary)]
/// id: i64,  // Auto-incrementing BIGINT primary key
/// ```
#[inline]
pub const fn bigserial<T>() -> SerialBuilder<T> {
    SerialBuilder {
        _marker: PhantomData,
        is_primary: false,
        is_unique: false,
        is_not_null: true, // BIGSERIAL is implicitly NOT NULL
        is_bigserial: true,
    }
}

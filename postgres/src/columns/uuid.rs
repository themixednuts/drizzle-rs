//! PostgreSQL UUID column builder.

use core::marker::PhantomData;

/// Builder for PostgreSQL UUID columns.
///
/// PostgreSQL has a native UUID type that stores 128-bit UUIDs in 16 bytes.
/// This is more efficient than storing UUIDs as TEXT (36 characters).
///
/// See: <https://www.postgresql.org/docs/current/datatype-uuid.html>
///
/// # UUID Generation
///
/// PostgreSQL can generate UUIDs server-side using `gen_random_uuid()` (requires pgcrypto
/// or built-in with PostgreSQL 13+), or you can generate them in Rust.
#[derive(Debug, Clone, Copy)]
pub struct UuidBuilder<T> {
    _marker: PhantomData<T>,
    /// Whether this column is the primary key.
    pub is_primary: bool,
    /// Whether this column has a UNIQUE constraint.
    pub is_unique: bool,
    /// Whether this column has a NOT NULL constraint.
    pub is_not_null: bool,
    /// Whether this column uses server-side random UUID generation.
    pub default_random: bool,
    /// Whether this column has any default value.
    pub has_default: bool,
}

impl<T> UuidBuilder<T> {
    /// Creates a new UUID column builder with no constraints.
    #[inline]
    pub const fn new() -> Self {
        Self {
            _marker: PhantomData,
            is_primary: false,
            is_unique: false,
            is_not_null: false,
            default_random: false,
            has_default: false,
        }
    }

    /// Makes this column the PRIMARY KEY.
    ///
    /// UUIDs make excellent primary keys for distributed systems where you
    /// can't rely on sequential IDs.
    ///
    /// See: <https://www.postgresql.org/docs/current/ddl-constraints.html#DDL-CONSTRAINTS-PRIMARY-KEYS>
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// #[uuid(primary, default_fn = uuid::Uuid::new_v4)]
    /// id: uuid::Uuid,
    /// ```
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

    /// Uses PostgreSQL's `gen_random_uuid()` for default values.
    ///
    /// This generates UUIDs server-side. Requires PostgreSQL 13+ or pgcrypto extension.
    ///
    /// See: <https://www.postgresql.org/docs/current/functions-uuid.html>
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// #[uuid(primary, default_random)]
    /// id: uuid::Uuid,  // SQL: id UUID PRIMARY KEY DEFAULT gen_random_uuid()
    /// ```
    #[inline]
    pub const fn default_random(self) -> Self {
        Self {
            default_random: true,
            has_default: true,
            ..self
        }
    }

    /// Marks this column as having a Rust function to generate UUIDs at runtime.
    ///
    /// This generates UUIDs in Rust before insert, not in PostgreSQL.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// #[uuid(primary, default_fn = uuid::Uuid::new_v4)]
    /// id: uuid::Uuid,  // UUIDv4 generated in Rust
    ///
    /// #[uuid(primary, default_fn = uuid::Uuid::now_v7)]
    /// id: uuid::Uuid,  // UUIDv7 (time-ordered) generated in Rust
    /// ```
    #[inline]
    pub const fn has_default_fn(self) -> Self {
        Self {
            has_default: true,
            ..self
        }
    }
}

/// Creates a UUID column builder.
///
/// PostgreSQL's native UUID type (128-bit, 16 bytes).
///
/// See: <https://www.postgresql.org/docs/current/datatype-uuid.html>
#[inline]
pub const fn uuid<T>() -> UuidBuilder<T> {
    UuidBuilder::new()
}

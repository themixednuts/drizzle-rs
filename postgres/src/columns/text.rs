//! PostgreSQL TEXT column builder.

use core::marker::PhantomData;

/// Builder for PostgreSQL TEXT columns.
///
/// TEXT stores variable-length character strings with no length limit.
/// This is PostgreSQL's most flexible string type.
///
/// See: <https://www.postgresql.org/docs/current/datatype-character.html>
///
/// # Comparison with VARCHAR
///
/// In PostgreSQL, TEXT and VARCHAR (without limit) are essentially identical.
/// Use TEXT when you don't need a length constraint; it's simpler.
#[derive(Debug, Clone, Copy)]
pub struct TextBuilder<T> {
    _marker: PhantomData<T>,
    /// Whether this column is the primary key.
    pub is_primary: bool,
    /// Whether this column has a UNIQUE constraint.
    pub is_unique: bool,
    /// Whether this column has a NOT NULL constraint.
    pub is_not_null: bool,
    /// Whether this column stores enum values as text.
    pub is_enum: bool,
    /// Whether this column uses a native PostgreSQL ENUM type.
    pub is_pgenum: bool,
    /// Whether this column stores JSON data.
    pub is_json: bool,
    /// Whether this column has any default value.
    pub has_default: bool,
}

impl<T> TextBuilder<T> {
    /// Creates a new TEXT column builder with no constraints.
    #[inline]
    pub const fn new() -> Self {
        Self {
            _marker: PhantomData,
            is_primary: false,
            is_unique: false,
            is_not_null: false,
            is_enum: false,
            is_pgenum: false,
            is_json: false,
            has_default: false,
        }
    }

    /// Makes this column the PRIMARY KEY.
    ///
    /// See: <https://www.postgresql.org/docs/current/ddl-constraints.html#DDL-CONSTRAINTS-PRIMARY-KEYS>
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
    /// See: <https://www.postgresql.org/docs/current/ddl-constraints.html#DDL-CONSTRAINTS-UNIQUE-CONSTRAINTS>
    #[inline]
    pub const fn unique(self) -> Self {
        Self {
            is_unique: true,
            ..self
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

    /// Stores Rust enum values as their string representation.
    ///
    /// Unlike native PostgreSQL ENUMs, this stores the variant name as TEXT.
    /// Use `pgenum()` for native PostgreSQL ENUM types.
    #[inline]
    pub const fn r#enum(self) -> Self {
        Self {
            is_enum: true,
            ..self
        }
    }

    /// Uses a native PostgreSQL ENUM type.
    ///
    /// PostgreSQL supports native ENUM types with database-level type checking.
    /// The enum type must be created before the table using `CREATE TYPE`.
    ///
    /// See: <https://www.postgresql.org/docs/current/datatype-enum.html>
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// #[derive(PostgresEnum)]
    /// enum Status { Active, Inactive }
    ///
    /// #[PostgresTable]
    /// struct Users {
    ///     #[text(pgenum)]
    ///     status: Status,  // Uses PostgreSQL 'status' ENUM type
    /// }
    /// ```
    #[inline]
    pub const fn pgenum(self) -> Self {
        Self {
            is_pgenum: true,
            ..self
        }
    }

    /// Stores JSON-serializable data as TEXT.
    ///
    /// For better performance with JSON operations, consider JSONB instead.
    ///
    /// See: <https://www.postgresql.org/docs/current/datatype-json.html>
    #[inline]
    pub const fn json(self) -> Self {
        Self {
            is_json: true,
            ..self
        }
    }

    /// Sets a compile-time default value for this column.
    ///
    /// See: <https://www.postgresql.org/docs/current/ddl-default.html>
    #[inline]
    pub const fn default(self, _value: &'static str) -> Self {
        Self {
            has_default: true,
            ..self
        }
    }

    /// Marks this column as having a Rust function to generate default values at runtime.
    ///
    /// The function is called by the insert builder when no value is provided.
    /// The default is generated in Rust, **not** in the PostgreSQL database.
    #[inline]
    pub const fn has_default_fn(self) -> Self {
        Self {
            has_default: true,
            ..self
        }
    }
}

/// Creates a TEXT column builder.
///
/// TEXT stores variable-length character strings with no limit.
///
/// See: <https://www.postgresql.org/docs/current/datatype-character.html>
#[inline]
pub const fn text<T>() -> TextBuilder<T> {
    TextBuilder::new()
}

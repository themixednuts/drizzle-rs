//! SQLite INTEGER column builder.

use core::marker::PhantomData;

/// Builder for SQLite INTEGER columns.
///
/// INTEGER columns store signed integers up to 8 bytes (64-bit).
/// SQLite uses a variable-length encoding, so small values use less space.
///
/// See: <https://sqlite.org/datatype3.html#storage_classes_and_datatypes>
///
/// # Special: INTEGER PRIMARY KEY
///
/// An `INTEGER PRIMARY KEY` column is special in SQLite - it becomes an alias for the ROWID,
/// providing auto-increment behavior without the AUTOINCREMENT keyword.
///
/// See: <https://sqlite.org/lang_createtable.html#rowid>
#[derive(Debug, Clone, Copy)]
pub struct IntegerBuilder<T> {
    _marker: PhantomData<T>,
    /// Whether this column is the primary key.
    pub is_primary: bool,
    /// Whether this column has AUTOINCREMENT.
    pub is_autoincrement: bool,
    /// Whether this column has a UNIQUE constraint.
    pub is_unique: bool,
    /// Whether this column has a NOT NULL constraint.
    pub is_not_null: bool,
    /// Whether this column stores enum discriminants.
    pub is_enum: bool,
    /// Whether this column has any default value.
    pub has_default: bool,
}

impl<T> IntegerBuilder<T> {
    /// Creates a new INTEGER column builder with no constraints.
    #[inline]
    pub const fn new() -> Self {
        Self {
            _marker: PhantomData,
            is_primary: false,
            is_autoincrement: false,
            is_unique: false,
            is_not_null: false,
            is_enum: false,
            has_default: false,
        }
    }

    /// Makes this column the PRIMARY KEY.
    ///
    /// An `INTEGER PRIMARY KEY` in SQLite is special:
    /// - It becomes an alias for the internal ROWID
    /// - Values auto-increment even without AUTOINCREMENT keyword
    /// - Maximum value is 9223372036854775807 (i64::MAX)
    ///
    /// See: <https://sqlite.org/lang_createtable.html#rowid>
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// #[integer(primary)]
    /// id: i32,  // SQL: id INTEGER PRIMARY KEY
    /// ```
    #[inline]
    pub const fn primary(self) -> Self {
        Self {
            is_primary: true,
            is_not_null: true,
            ..self
        }
    }

    /// Enables AUTOINCREMENT for this INTEGER PRIMARY KEY.
    ///
    /// **Warning:** AUTOINCREMENT is rarely needed! It prevents ROWID reuse
    /// and is slightly slower. Regular `INTEGER PRIMARY KEY` already auto-increments.
    ///
    /// Use AUTOINCREMENT only when you need to guarantee that ROWIDs are never reused,
    /// even after rows are deleted.
    ///
    /// See: <https://sqlite.org/autoinc.html>
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// #[integer(primary, autoincrement)]
    /// id: i32,  // SQL: id INTEGER PRIMARY KEY AUTOINCREMENT
    /// ```
    #[inline]
    pub const fn autoincrement(self) -> Self {
        Self {
            is_autoincrement: true,
            is_primary: true, // AUTOINCREMENT requires PRIMARY KEY
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

    /// Stores Rust enum discriminants as integers.
    ///
    /// The enum type must implement:
    /// - `Into<i32>` or similar (for serialization)
    /// - `TryFrom<i32>` (for deserialization)
    ///
    /// This is more compact than TEXT storage but less readable in the database.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// #[derive(SQLiteEnum)]
    /// enum Priority { Low = 1, Medium = 2, High = 3 }
    ///
    /// #[SQLiteTable]
    /// struct Tasks {
    ///     #[integer(enum)]
    ///     priority: Priority,  // Stored as 1, 2, or 3
    /// }
    /// ```
    #[inline]
    pub const fn r#enum(self) -> Self {
        Self {
            is_enum: true,
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
    /// #[integer(default = 0)]
    /// count: i32,  // SQL: count INTEGER DEFAULT 0
    /// ```
    #[inline]
    pub const fn default(self, _value: i64) -> Self {
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

/// Creates an INTEGER column builder.
///
/// INTEGER columns store signed integers up to 64-bit.
///
/// See: <https://sqlite.org/datatype3.html#storage_classes_and_datatypes>
#[inline]
pub const fn integer<T>() -> IntegerBuilder<T> {
    IntegerBuilder::new()
}

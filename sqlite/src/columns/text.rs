//! SQLite TEXT column builder.

use core::marker::PhantomData;

/// Builder for SQLite TEXT columns.
///
/// TEXT columns store variable-length UTF-8 character strings with no size limit.
/// SQLite's TEXT affinity accepts any string data.
///
/// See: <https://sqlite.org/datatype3.html#storage_classes_and_datatypes>
///
/// # Generated Usage
///
/// The macro generates builder calls from your field attributes:
///
/// ```rust,ignore
/// #[SQLiteTable]
/// struct Users {
///     #[text(primary)]           // text::<UsersId>().primary()
///     id: String,
///     #[text(unique)]            // text::<UsersEmail>().unique()
///     email: String,
///     #[text(default = "guest")] // text::<UsersName>().default("guest")
///     name: String,
/// }
/// ```
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
            is_json: false,
            has_default: false,
        }
    }

    /// Makes this column the PRIMARY KEY.
    ///
    /// In SQLite, a TEXT PRIMARY KEY:
    /// - Must contain unique values
    /// - Cannot contain NULL (implicitly NOT NULL)
    /// - Only one primary key per table is allowed
    /// - Unlike INTEGER PRIMARY KEY, TEXT PKs are **not** aliases for ROWID
    ///
    /// See: <https://sqlite.org/lang_createtable.html#the_primary_key>
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// #[text(primary)]
    /// id: String,
    /// ```
    #[inline]
    pub const fn primary(self) -> Self {
        Self {
            is_primary: true,
            is_not_null: true, // PRIMARY KEY implies NOT NULL
            ..self
        }
    }

    /// Adds a UNIQUE constraint to this column.
    ///
    /// A UNIQUE constraint ensures all values in this column are distinct.
    /// NULL values are allowed and are considered distinct from each other
    /// (multiple NULLs are permitted).
    ///
    /// See: <https://sqlite.org/lang_createtable.html#unique_constraints>
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// #[text(unique)]
    /// email: String,
    /// ```
    #[inline]
    pub const fn unique(self) -> Self {
        Self {
            is_unique: true,
            ..self
        }
    }

    /// Adds a NOT NULL constraint to this column.
    ///
    /// This constraint prevents NULL values from being inserted.
    /// Note: This is typically inferred from the Rust type - `String` implies NOT NULL,
    /// while `Option<String>` allows NULL.
    ///
    /// See: <https://sqlite.org/lang_createtable.html#not_null_constraints>
    #[inline]
    pub const fn not_null(self) -> Self {
        Self {
            is_not_null: true,
            ..self
        }
    }

    /// Stores Rust enum values as their string representation.
    ///
    /// The enum type must implement:
    /// - `Into<&str>` (for serialization to database)
    /// - `TryFrom<&str>` (for deserialization from database)
    ///
    /// SQLite doesn't have native enum types, so this stores the variant name as TEXT.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// #[derive(SQLiteEnum)]
    /// enum Status { Active, Inactive }
    ///
    /// #[SQLiteTable]
    /// struct Users {
    ///     #[text(enum)]
    ///     status: Status,  // Stored as "Active" or "Inactive"
    /// }
    /// ```
    #[inline]
    pub const fn r#enum(self) -> Self {
        Self {
            is_enum: true,
            ..self
        }
    }

    /// Stores JSON-serializable data as TEXT.
    ///
    /// Requires the `serde` feature. The type must implement `Serialize` and `Deserialize`.
    /// Data is stored as a JSON string in the TEXT column.
    ///
    /// See: <https://sqlite.org/json1.html>
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// #[derive(Serialize, Deserialize)]
    /// struct Metadata { tags: Vec<String> }
    ///
    /// #[SQLiteTable]
    /// struct Posts {
    ///     #[text(json)]
    ///     metadata: Metadata,  // Stored as '{"tags":["rust","sqlite"]}'
    /// }
    /// ```
    #[inline]
    pub const fn json(self) -> Self {
        Self {
            is_json: true,
            ..self
        }
    }

    /// Sets a compile-time default value for this column.
    ///
    /// This default is used in the SQL CREATE TABLE statement.
    /// When inserting without specifying this column, SQLite uses this value.
    ///
    /// See: <https://sqlite.org/lang_createtable.html#the_default_clause>
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// #[text(default = "pending")]
    /// status: String,  // SQL: status TEXT DEFAULT 'pending'
    /// ```
    #[inline]
    pub const fn default(self, _value: &'static str) -> Self {
        Self {
            has_default: true,
            ..self
        }
    }

    /// Marks this column as having a Rust function to generate default values at runtime.
    ///
    /// The actual function is specified in the attribute (e.g., `#[text(default_fn = Uuid::new_v4)]`)
    /// and called by the insert builder when no value is provided.
    /// The default is generated in Rust, **not** in the SQL database.
    /// This is useful for UUIDs, timestamps, or other dynamically generated values.
    ///
    /// **Note:** This does not affect `drizzle_kit` migrations - it's runtime-only.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// #[text(default_fn = Uuid::new_v4)]
    /// id: String,  // Generates UUID in Rust before insert
    /// ```
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
/// TEXT columns store variable-length UTF-8 character strings.
///
/// See: <https://sqlite.org/datatype3.html#storage_classes_and_datatypes>
///
/// # Example
///
/// ```rust,ignore
/// const COLUMN: TextBuilder<UsersName> = text::<UsersName>().unique();
/// ```
#[inline]
pub const fn text<T>() -> TextBuilder<T> {
    TextBuilder::new()
}

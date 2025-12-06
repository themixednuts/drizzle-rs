//! SQLite BLOB column builder.

use core::marker::PhantomData;

/// Builder for SQLite BLOB columns.
///
/// BLOB columns store binary data exactly as input, with no encoding.
/// Useful for UUIDs, binary files, encrypted data, etc.
///
/// See: <https://sqlite.org/datatype3.html#storage_classes_and_datatypes>
///
/// # UUID Storage
///
/// UUIDs are often stored as BLOBs (16 bytes) rather than TEXT (36 characters)
/// for better space efficiency and comparison performance.
#[derive(Debug, Clone, Copy)]
pub struct BlobBuilder<T> {
    _marker: PhantomData<T>,
    /// Whether this column is the primary key.
    pub is_primary: bool,
    /// Whether this column has a UNIQUE constraint.
    pub is_unique: bool,
    /// Whether this column has a NOT NULL constraint.
    pub is_not_null: bool,
    /// Whether this column has any default value.
    pub has_default: bool,
    /// Whether this column stores JSON data as binary.
    pub is_json: bool,
}

impl<T> BlobBuilder<T> {
    /// Creates a new BLOB column builder with no constraints.
    #[inline]
    pub const fn new() -> Self {
        Self {
            _marker: PhantomData,
            is_primary: false,
            is_unique: false,
            is_not_null: false,
            has_default: false,
            is_json: false,
        }
    }

    /// Makes this column the PRIMARY KEY.
    ///
    /// BLOB primary keys are useful for UUID-based primary keys where you want
    /// efficient binary storage.
    ///
    /// See: <https://sqlite.org/lang_createtable.html#the_primary_key>
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// #[blob(primary, default_fn = uuid::Uuid::new_v4)]
    /// id: uuid::Uuid,  // 16-byte binary UUID as primary key
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

    /// Marks this column as storing JSON data as binary.
    ///
    /// JSON can be stored as BLOB for efficient binary encoding (e.g., CBOR, MessagePack).
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// #[blob(json)]
    /// config: Option<UserConfig>,  // Serialized JSON as binary
    /// ```
    #[inline]
    pub const fn json(self) -> Self {
        Self {
            is_json: true,
            ..self
        }
    }

    /// Marks this column as having a Rust function to generate default values at runtime.
    ///
    /// This is commonly used for UUID generation:
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// #[blob(primary, default_fn = uuid::Uuid::new_v4)]
    /// id: uuid::Uuid,  // Generates UUIDv4 before insert
    ///
    /// #[blob(primary, default_fn = uuid::Uuid::now_v7)]
    /// id: uuid::Uuid,  // Generates UUIDv7 (time-ordered) before insert
    /// ```
    #[inline]
    pub const fn has_default_fn(self) -> Self {
        Self {
            has_default: true,
            ..self
        }
    }
}

/// Creates a BLOB column builder.
///
/// BLOB columns store raw binary data with no encoding.
///
/// See: <https://sqlite.org/datatype3.html#storage_classes_and_datatypes>
#[inline]
pub const fn blob<T>() -> BlobBuilder<T> {
    BlobBuilder::new()
}

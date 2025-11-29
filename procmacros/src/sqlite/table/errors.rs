//! Shared error messages for SQLite macro generation.
//!
//! Centralizes error strings to ensure consistency across driver implementations
//! and simplify maintenance.

/// Error messages for JSON field configuration
pub(crate) mod json {
    pub const INVALID_COLUMN_TYPE: &str = "JSON fields must use either TEXT or BLOB column types.\n\
         \n\
         - TEXT storage: Human-readable JSON string format\n\
         - BLOB storage: Binary JSON format (more efficient)\n\
         \n\
         Example: #[text(json)] or #[blob(json)]";

    pub const SERDE_REQUIRED: &str = "JSON fields require the 'serde' feature to be enabled.\n\
         \n\
         Add to Cargo.toml:\n\
         drizzle = { version = \"...\", features = [\"serde\"] }";
}

/// Error messages for UUID field configuration
#[allow(dead_code)]
pub(crate) mod uuid {
    pub const INVALID_COLUMN_TYPE: &str = "UUID fields must use BLOB or TEXT column types.\n\
         \n\
         - BLOB storage: Efficient 16-byte binary format (recommended)\n\
         - TEXT storage: Human-readable string format\n\
         \n\
         Example: #[blob] uuid: Uuid or #[text] uuid: Uuid";
}

/// Error messages for enum field configuration
pub(crate) mod enums {
    pub const INVALID_COLUMN_TYPE: &str = "Enum fields are only supported with TEXT or INTEGER column types.\n\
         \n\
         - TEXT storage: Stores variant names (e.g., 'Active', 'Pending')\n\
         - INTEGER storage: Stores discriminant values (0, 1, 2, ...)\n\
         \n\
         Example: #[text(enum)] or #[integer(enum)]";
}

/// Error messages for type conversion
#[allow(dead_code)]
pub(crate) mod conversion {
    pub const REFERENCE_TYPE_UNSUPPORTED: &str = "Reference types (&str, &[u8]) are not supported.\n\
         \n\
         Use owned types instead:\n\
         - &str -> String\n\
         - &[u8] -> Vec<u8>";

    /// Generate a field conversion error message
    pub fn required_field(field_name: &str) -> String {
        format!("Error converting required field `{}`", field_name)
    }
}

//! Shared error messages for PostgreSQL macro generation.
//!
//! Centralizes error strings to ensure consistency across driver implementations
//! and simplify maintenance.

#![allow(dead_code)]

/// Error messages for JSON field configuration
pub(crate) mod json {
    pub const INVALID_COLUMN_TYPE: &str = "JSON fields must use either JSON or JSONB column types.\n\
         \n\
         - JSON: Standard JSON storage\n\
         - JSONB: Binary JSON format (more efficient for queries)\n\
         \n\
         Example: #[json] or #[jsonb]";

    pub const SERDE_REQUIRED: &str = "JSON fields require the 'serde' feature to be enabled.\n\
         \n\
         Add to Cargo.toml:\n\
         drizzle = { version = \"...\", features = [\"serde\"] }";
}

/// Error messages for UUID field configuration
#[allow(dead_code)]
pub(crate) mod uuid {
    pub const INVALID_COLUMN_TYPE: &str = "UUID fields must use the UUID column type.\n\
         \n\
         PostgreSQL has native UUID support.\n\
         \n\
         Example: #[uuid] id: Uuid";
}

/// Error messages for enum field configuration
pub(crate) mod enums {
    pub const INVALID_COLUMN_TYPE: &str = "Enum fields are only supported with TEXT, INTEGER, or native ENUM column types.\n\
         \n\
         - TEXT storage: Stores variant names (e.g., 'Active', 'Pending')\n\
         - INTEGER storage: Stores discriminant values (0, 1, 2, ...)\n\
         - Native ENUM: PostgreSQL native enum type (most efficient)\n\
         \n\
         Example: #[text(enum)], #[integer(enum)], or #[r#enum(MyEnum)]";
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

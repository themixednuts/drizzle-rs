//! Shared error messages for `PostgreSQL` macro generation.
//!
//! Centralizes error strings to ensure consistency across driver implementations
//! and simplify maintenance.

#![allow(dead_code)]

/// Error messages for JSON field configuration
pub mod json {
    pub const INVALID_COLUMN_TYPE: &str = "JSON fields must use either JSON or JSONB column types.\n\
         \n\
         - JSON: Standard JSON storage\n\
         - JSONB: Binary JSON format (more efficient for queries)\n\
         \n\
         Example: #[column(json)] or #[column(jsonb)]";

    pub const SERDE_REQUIRED: &str = "JSON fields require the 'serde' feature to be enabled.\n\
         \n\
         Add to Cargo.toml:\n\
         drizzle = { version = \"...\", features = [\"serde\"] }";
}

/// Error messages for UUID field configuration
#[allow(dead_code)]
pub mod uuid {
    pub const INVALID_COLUMN_TYPE: &str = "UUID fields must use the UUID column type.\n\
         \n\
         PostgreSQL has native UUID support.\n\
         \n\
         Example: pub id: Uuid";
}

/// Error messages for enum field configuration
pub mod enums {
    pub const INVALID_COLUMN_TYPE: &str = "Enum fields must use `#[column(enum)]` with a type that derives `PostgresEnum`.\n\
         \n\
         - Default: native PostgreSQL ENUM type\n\
         - With integer repr (e.g. `#[repr(i32)]`): INTEGER storage\n\
         \n\
         Example: #[column(enum)] pub status: Status";
}

/// Error messages for type conversion
#[allow(dead_code)]
pub mod conversion {
    pub const REFERENCE_TYPE_UNSUPPORTED: &str = "Reference types (&str, &[u8]) are not supported.\n\
         \n\
         Use owned types instead:\n\
         - &str -> String\n\
         - &[u8] -> Vec<u8>";

    /// Generate a field conversion error message
    pub fn required_field(field_name: &str) -> String {
        format!("Error converting required field `{field_name}`")
    }
}

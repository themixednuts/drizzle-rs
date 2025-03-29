// Remove incorrect doc include
// #![doc = include_str!("../README.md")]

//! # Drizzle-rs
//! A type-safe SQL query builder and ORM library for Rust,
//! inspired by Drizzle ORM (TypeScript).

// Core Driver Traits & Error
pub use drivers::{Connection, DbRow, DriverError, PreparedStatement, Transaction};

// Core Drizzle struct
pub mod core;
pub use core::Drizzle;

// Core Proc Macros
pub use procmacros::{FromRow, SQLiteEnum, SQLiteTable, drizzle, schema};

// SQLite specific types (like SQLiteValue)
#[cfg(feature = "rusqlite")]
pub mod sqlite;

// Feature-gated driver Connection implementations
// Example for Rusqlite:
#[cfg(feature = "libsql")]
pub use drivers::libsql::LibsqlConnection; // Assuming struct name
#[cfg(feature = "libsql-rusqlite")]
pub use drivers::libsql_rusqlite::LibsqlRusqliteConnection;
#[cfg(feature = "rusqlite")]
pub use drivers::rusqlite::RusqliteConnection; // Assuming struct name

/// A comprehensive prelude that brings all commonly used items into scope.
/// Users can import everything needed with a single `use drizzle_rs::prelude::*;` statement.
pub mod prelude {
    // Core Drizzle struct
    pub use crate::core::Drizzle;

    // Core Driver traits & Error
    pub use drivers::{Connection, DbRow, DriverError, PreparedStatement, Transaction};

    // SQLite Value Type
    pub use drivers::SQLiteValue;

    // Core Proc Macros
    pub use procmacros::{FromRow, SQLiteEnum, SQLiteTable, drizzle, schema};

    // Feature-gated driver connection structs
    #[cfg(feature = "libsql")]
    pub use drivers::libsql::LibsqlConnection; // Assuming struct name
    #[cfg(feature = "libsql-rusqlite")]
    pub use drivers::libsql_rusqlite::LibsqlRusqliteConnection;
    #[cfg(feature = "rusqlite")]
    pub use drivers::rusqlite::RusqliteConnection; // Assuming struct name

    // --- Query Builder Exports ---

    // Core traits and types
    pub use querybuilder::core::schema_traits::*;
    pub use querybuilder::core::traits::*;
    pub use querybuilder::core::{IntoValue, SQL, SQLParam, ToSQL};

    // Common expression functions
    pub use querybuilder::core::expressions::conditions::*;

    // Common macros
    pub use querybuilder::{and, columns, or};

    // SQLite specific query builder components (assuming only sqlite for now)
    #[cfg(feature = "rusqlite")]
    pub use querybuilder::sqlite::common::SQLiteTableType;
    #[cfg(feature = "rusqlite")]
    pub use querybuilder::sqlite::query_builder::{
        Columns, DeleteBuilder, InsertBuilder, JoinType, QueryBuilder, SQLiteQueryBuilder, Select,
        SortDirection, UpdateBuilder, alias,
    };

    // Re-export uuid types if feature enabled
    #[cfg(feature = "uuid")]
    pub use uuid::Uuid;

    // Re-export serde types if feature enabled (optional, maybe not needed in prelude)
    // #[cfg(feature = "serde_json")]
    // pub use serde::{Serialize, Deserialize};
    // #[cfg(feature = "serde_json")]
    // pub use serde_json::Value as JsonValue;
}

// Conditionally export SQLite-related items from querybuilder
// These are useful shortcuts but depend on the sqlite feature of querybuilder
#[cfg(feature = "rusqlite")]
pub mod sqlite_specifics {
    pub use querybuilder::sqlite::SQLiteColumn;
    pub use querybuilder::sqlite::common::{IntoSQLiteValue, SQLiteEnum, SQLiteTableType};
    pub use querybuilder::sqlite::query_builder::{
        Columns, DeleteBuilder, InsertBuilder, QueryBuilder, SelectBuilder, SortDirection,
        UpdateBuilder, alias,
    };
    // pub use querybuilder::sqlite::SQLiteDialect; // Comment out - likely defined elsewhere or not exported
    pub use querybuilder::sqlite::SQLiteQueryBuilder;
}

// Example of how the Drizzle struct might be used directly
// (This might be internal or part of a higher-level API)
pub struct DrizzleInstance<Conn: Connection> {
    connection: Conn,
    // Potentially store schema information here if needed at runtime
}

impl<Conn: Connection> DrizzleInstance<Conn> {
    pub fn new(connection: Conn) -> Self {
        Self { connection }
    }

    // Example method - actual query execution would likely use the query builder
    pub fn execute(&self, sql: &str, params: &[Conn::Value]) -> Result<usize, DriverError> {
        self.connection.run_statement(sql, params)
    }
}

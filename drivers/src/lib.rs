pub mod rusqlite;
// Re-export commonly used items
#[cfg(feature = "rusqlite")]
pub use rusqlite::*;

use std::error::Error;
/// Connection trait to abstract database connections
/// This allows the library to work with different connection types
/// like direct connections, connection pools, etc.
pub trait Connection {
    type RowType;
    type Value;
    type QueryResult;

    /// Execute a SQL query with parameters and return rows
    fn query(&self, sql: &str, params: &[Self::Value])
    -> Result<Self::QueryResult, Box<dyn Error>>;

    /// Execute a SQL statement with parameters and return affected rows
    fn execute(&self, sql: &str, params: &[Self::Value]) -> Result<usize, Box<dyn Error>>;

    /// Begin a transaction
    fn begin_transaction(&self) -> Result<(), Box<dyn Error>>;

    /// Commit a transaction
    fn commit(&self) -> Result<(), Box<dyn Error>>;

    /// Rollback a transaction
    fn rollback(&self) -> Result<(), Box<dyn Error>>;
}

/// Drizzle is the main entry point for the ORM
/// It holds a reference to a connection and optionally a schema
pub struct Drizzle<'a, C: Connection, S = ()> {
    pub conn: &'a C,
    pub schema: S,
}

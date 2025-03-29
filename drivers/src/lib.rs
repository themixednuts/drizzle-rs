#[cfg(feature = "libsql")]
pub mod libsql;
#[cfg(feature = "libsql-rusqlite")]
pub mod libsql_rusqlite;
#[cfg(feature = "rusqlite")]
pub mod rusqlite; // Add module for libsql // Add module for libsql-rusqlite
// Re-export commonly used items based on features
#[cfg(feature = "libsql")]
pub use crate::libsql::*;
#[cfg(feature = "libsql-rusqlite")]
pub use crate::libsql_rusqlite::*;
#[cfg(feature = "rusqlite")]
pub use crate::rusqlite::*;

// Remove the re-export from querybuilder
// pub use querybuilder::sqlite::common::SQLiteValue;
use std::borrow::Cow;
use thiserror::Error;

// --- SQLiteValue Definition --- Moved from querybuilder
#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub enum SQLiteValue<'a> {
    Integer(i64),
    Text(Cow<'a, str>),
    Blob(Cow<'a, [u8]>),
    Real(f64),
    Null,
}

impl<'a> Default for SQLiteValue<'a> {
    fn default() -> Self {
        Self::Integer(Default::default())
    }
}

impl<'a> std::fmt::Display for SQLiteValue<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Basic display implementation for debugging/logging
        match self {
            SQLiteValue::Integer(i) => write!(f, "{}", i),
            SQLiteValue::Text(s) => write!(f, "\"{}\"", s),
            SQLiteValue::Blob(b) => write!(f, "BLOB({} bytes)", b.len()),
            SQLiteValue::Real(r) => write!(f, "{}", r),
            SQLiteValue::Null => write!(f, "NULL"),
        }
    }
}

impl<'a> From<&'a str> for SQLiteValue<'a> {
    fn from(value: &'a str) -> Self {
        Self::Text(Cow::Borrowed(value))
    }
}

impl<'a, T: AsRef<str> + 'a> From<&'a T> for SQLiteValue<'a> {
    fn from(value: &'a T) -> Self {
        Self::Text(Cow::Borrowed(value.as_ref()))
    }
}

impl<'a> From<String> for SQLiteValue<'a> {
    fn from(value: String) -> Self {
        Self::Text(Cow::Owned(value))
    }
}

impl<'a> From<&'a [u8]> for SQLiteValue<'a> {
    fn from(value: &'a [u8]) -> Self {
        Self::Blob(Cow::Borrowed(value))
    }
}

impl<'a> From<Vec<u8>> for SQLiteValue<'a> {
    fn from(value: Vec<u8>) -> Self {
        Self::Blob(Cow::Owned(value))
    }
}

impl<'a> From<f64> for SQLiteValue<'a> {
    fn from(value: f64) -> Self {
        Self::Real(value)
    }
}

impl<'a> From<i64> for SQLiteValue<'a> {
    fn from(value: i64) -> Self {
        Self::Integer(value)
    }
}

impl<'a> From<bool> for SQLiteValue<'a> {
    fn from(value: bool) -> Self {
        Self::Integer(if value { 1 } else { 0 })
    }
}

// Implement From for other integer types
impl<'a> From<i32> for SQLiteValue<'a> {
    fn from(value: i32) -> Self {
        Self::Integer(value as i64)
    }
}

impl<'a> From<i16> for SQLiteValue<'a> {
    fn from(value: i16) -> Self {
        Self::Integer(value as i64)
    }
}

impl<'a> From<i8> for SQLiteValue<'a> {
    fn from(value: i8) -> Self {
        Self::Integer(value as i64)
    }
}

impl<'a> From<u32> for SQLiteValue<'a> {
    fn from(value: u32) -> Self {
        Self::Integer(value as i64)
    }
}

impl<'a> From<u16> for SQLiteValue<'a> {
    fn from(value: u16) -> Self {
        Self::Integer(value as i64)
    }
}

impl<'a> From<u8> for SQLiteValue<'a> {
    fn from(value: u8) -> Self {
        Self::Integer(value as i64)
    }
}

impl<'a> From<usize> for SQLiteValue<'a> {
    fn from(value: usize) -> Self {
        Self::Integer(value as i64)
    }
}

// Support Option<T>
impl<'a, T: Into<SQLiteValue<'a>>> From<Option<T>> for SQLiteValue<'a> {
    fn from(value: Option<T>) -> Self {
        match value {
            Some(inner) => inner.into(),
            None => Self::Null,
        }
    }
}

// UUID Support
#[cfg(feature = "uuid")]
impl<'a> From<uuid::Uuid> for SQLiteValue<'a> {
    fn from(value: uuid::Uuid) -> Self {
        Self::Text(Cow::Owned(value.to_string()))
    }
}

// JSON Support Removed - To be handled in querybuilder via ToSQL/IntoSQLiteValue
// #[cfg(feature = "serde_json")]
// impl<'a, T: serde::Serialize> From<querybuilder::core::Json<T>> for SQLiteValue<'a> {
//     fn from(value: querybuilder::core::Json<T>) -> Self {
//         match serde_json::to_string(&value.0) {
//             Ok(s) => Self::Text(Cow::Owned(s)),
//             Err(_) => Self::Null, // Or handle error appropriately
//         }
//     }
// }

// --- End SQLiteValue Definition ---

#[derive(Error, Debug)]
pub enum DriverError {
    #[error("Feature not enabled for the selected driver")]
    FeatureNotEnabled,
    #[error("Database connection error: {0}")]
    Connection(String),
    #[error("Query execution error: {0}")]
    Query(String),
    #[error("Result mapping error: {0}")]
    Mapping(String),
    #[error("Transaction error: {0}")]
    Transaction(String),
    #[error("Prepared statement error: {0}")]
    Statement(String),
    #[error("Type mismatch: {0}")]
    TypeMismatch(String),
    #[error("Underlying driver error: {0}")]
    Driver(Box<dyn std::error::Error + Send + Sync + 'static>),
}

/// Represents a database row.
/// Implementors should provide ways to access column values by index or name.
pub trait DbRow {
    /// Get the raw SQLiteValue from the row by column index.
    // Change signature to return owned SQLiteValue, removing FromSql bound.
    // The lifetime is now 'static as the row implementation should own the data.
    fn get(&self, index: usize) -> Result<SQLiteValue<'static>, DriverError>;

    // Optional: Add get_by_name later if needed
    // fn get_by_name<T: rusqlite::types::FromSql>(&self, name: &str) -> Result<T, DriverError>;
}

/// Represents a prepared statement, ready for execution.
pub trait PreparedStatement<'stmt>: Sized {
    // Row lifetime is bound to the statement lifetime 'stmt
    type Row: DbRow;
    type Value; // Parameters
    type QueryResult; // Result collection (e.g., Vec<Self::Row>)
    type Error: std::error::Error + Send + Sync + 'static;

    /// Executes the prepared statement with parameters, returning affected rows.
    fn run(&mut self, params: &[Self::Value]) -> Result<usize, DriverError>;

    /// Executes the prepared statement with parameters, returning rows.
    fn query(&mut self, params: &[Self::Value]) -> Result<Self::QueryResult, DriverError>;
    // Add query_one, query_optional etc. if needed
}

/// Represents an active database transaction.
/// Methods here mirror `Connection` but operate within the transaction context.
pub trait Transaction<'tx>: Sized {
    // Row lifetime is bound to the transaction lifetime 'tx
    type Row: DbRow;
    type Value;
    type QueryResult;
    type Error: std::error::Error + Send + Sync + 'static;
    // Prepared statement created within tx must live at most as long as tx ('tx: 'stmt)
    type Prepared<'stmt>: PreparedStatement<
            'stmt,
            QueryResult = Self::QueryResult,
            Error = Self::Error,
            Row = Self::Row,
        >
    where
        Self: 'stmt,
        'tx: 'stmt;

    /// Executes a raw SQL statement within the transaction.
    fn run_statement(&mut self, sql: &str, params: &[Self::Value]) -> Result<usize, DriverError>;

    /// Executes a raw SQL query within the transaction.
    fn query_statement(
        &mut self,
        sql: &str,
        params: &[Self::Value],
    ) -> Result<Self::QueryResult, DriverError>;

    /// Prepares a SQL statement for repeated execution within the transaction.
    fn prepare<'stmt>(&'stmt mut self, sql: &str) -> Result<Self::Prepared<'stmt>, DriverError>
    where
        Self: 'stmt;

    /// Commits the transaction.
    fn commit(self) -> Result<(), DriverError>;

    /// Rolls back the transaction.
    fn rollback(self) -> Result<(), DriverError>;
}

/// Connection trait to abstract database connections.
pub trait Connection: Sized {
    type Value;
    // Row lifetime is bound to the connection lifetime 'conn
    type Row: DbRow;
    type QueryResult;
    type Error: std::error::Error + Send + Sync + 'static;
    // Transaction lifetime 'tx must live at most as long as connection 'conn ('conn: 'tx)
    type Transaction<'tx>: Transaction<'tx, QueryResult = Self::QueryResult, Error = Self::Error, Row = Self::Row>
    where
        Self: 'tx;
    // Prepared statement lifetime 'stmt must live at most as long as connection 'conn ('conn: 'stmt)
    type Prepared<'stmt>: PreparedStatement<
            'stmt,
            QueryResult = Self::QueryResult,
            Error = Self::Error,
            Row = Self::Row,
        >
    where
        Self: 'stmt;

    /// Executes a raw SQL statement, returning affected rows.
    fn run_statement(&self, sql: &str, params: &[Self::Value]) -> Result<usize, DriverError>;

    /// Executes a raw SQL query, returning rows or results.
    fn query_statement(
        &self,
        sql: &str,
        params: &[Self::Value],
    ) -> Result<Self::QueryResult, DriverError>;

    /// Prepares a SQL statement for repeated execution.
    fn prepare<'stmt>(&'stmt self, sql: &str) -> Result<Self::Prepared<'stmt>, DriverError>
    where
        Self: 'stmt;

    /// Begins a transaction, returning a transaction handle.
    fn begin_transaction<'tx>(&'tx mut self) -> Result<Self::Transaction<'tx>, DriverError>
    where
        Self: 'tx;
}

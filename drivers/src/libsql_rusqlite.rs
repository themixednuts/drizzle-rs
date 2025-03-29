#![cfg(feature = "libsql-rusqlite")]

use super::{
    Connection,
    DbRow,
    DriverError,
    PreparedStatement,
    SQLiteValue, // Assuming SQLiteValue works for params for now
    Transaction,
};
use libsql_rusqlite::types::{Value as LibsqlRusqliteValue, ValueRef};
use std::borrow::Cow;
use std::marker::PhantomData;

// Helper function to map SQLiteValue to libsql_rusqlite::Value
// Clones owned data (Text, Blob) to ensure correct lifetime for libsql_rusqlite
fn map_params_to_values<'a>(
    params: &'a [SQLiteValue<'a>],
) -> Result<Vec<LibsqlRusqliteValue>, DriverError> {
    params
        .iter()
        .map(|p| match p {
            SQLiteValue::Null => Ok(LibsqlRusqliteValue::Null),
            SQLiteValue::Integer(i) => Ok(LibsqlRusqliteValue::Integer(*i)),
            SQLiteValue::Real(f) => Ok(LibsqlRusqliteValue::Real(*f)),
            SQLiteValue::Text(s) => Ok(LibsqlRusqliteValue::Text(s.to_string())), // Clone Cow -> String
            SQLiteValue::Blob(b) => Ok(LibsqlRusqliteValue::Blob(b.to_vec())), // Clone Cow -> Vec<u8>
        })
        .collect::<Result<Vec<_>, _>>()
        // Map potential error during collection (though our mapping is infallible here)
        .map_err(|_e: std::convert::Infallible| {
            // Prefix unused variable _e
            DriverError::Query("Infallible error during param mapping?".to_string())
        })
}

// --- Wrapper Structs ---

/// Wrapper around `libsql_rusqlite::Connection` to implement the `Connection` trait.
pub struct LibsqlRusqliteConnection<'conn> {
    conn: libsql_rusqlite::Connection,
    _marker: PhantomData<&'conn ()>, // To potentially tie lifetime if needed later
}

impl<'conn> LibsqlRusqliteConnection<'conn> {
    pub fn new(conn: libsql_rusqlite::Connection) -> Self {
        Self {
            conn,
            _marker: PhantomData,
        }
    }
}

/// Wrapper around `libsql_rusqlite::Row` that owns its data.
pub struct LibsqlRusqliteRow {
    // Hold owned values
    values: Vec<SQLiteValue<'static>>,
}

/// Helper to map libsql_rusqlite::ValueRef to our owned SQLiteValue
fn map_value_ref_to_sqlite_value(value_ref: ValueRef) -> Result<SQLiteValue<'static>, DriverError> {
    match value_ref {
        ValueRef::Null => Ok(SQLiteValue::Null),
        ValueRef::Integer(i) => Ok(SQLiteValue::Integer(i)),
        ValueRef::Real(f) => Ok(SQLiteValue::Real(f)),
        ValueRef::Text(t_bytes) => match std::str::from_utf8(t_bytes) {
            Ok(s) => Ok(SQLiteValue::Text(Cow::Owned(s.to_string()))),
            Err(e) => Err(DriverError::Mapping(format!(
                "Invalid UTF-8 sequence in TEXT column: {}",
                e
            ))),
        },
        ValueRef::Blob(b) => Ok(SQLiteValue::Blob(Cow::Owned(b.to_vec()))),
    }
}

// No lifetime needed for LibsqlRusqliteRow impl as it owns data
impl DbRow for LibsqlRusqliteRow {
    /// Get the raw SQLiteValue from the row by column index.
    fn get(&self, index: usize) -> Result<SQLiteValue<'static>, DriverError> {
        self.values
            .get(index)
            .cloned() // Clone the SQLiteValue<'static>
            .ok_or_else(|| {
                DriverError::Mapping(format!(r#"Index {} out of bounds for row"#, index))
            }) // Use raw string literal
    }
}

/// Prepared statement wrapper
pub struct LibsqlRusqlitePreparedStatement<'stmt> {
    stmt: libsql_rusqlite::Statement<'stmt>,
    // Removed 'conn lifetime marker as it's not needed now
}

// Only 'stmt lifetime is relevant here
impl<'stmt> PreparedStatement<'stmt> for LibsqlRusqlitePreparedStatement<'stmt> {
    // LibsqlRusqliteRow has no lifetime now
    type Row = LibsqlRusqliteRow;
    // Value lifetime can likely be 'static now if params are always mapped/cloned
    // Keeping 'conn for now, but could potentially be simplified.
    type Value = SQLiteValue<'static>;
    type QueryResult = Vec<Self::Row>;
    type Error = libsql_rusqlite::Error;

    fn run(&mut self, params: &[Self::Value]) -> Result<usize, DriverError> {
        let libsql_rusqlite_values = map_params_to_values(params)?;
        self.stmt
            .execute(libsql_rusqlite::params_from_iter(
                libsql_rusqlite_values.iter(),
            ))
            .map_err(|e| DriverError::Statement(e.to_string()))
    }

    fn query(&mut self, params: &[Self::Value]) -> Result<Self::QueryResult, DriverError> {
        let libsql_rusqlite_values = map_params_to_values(params)?;

        // Get column count *before* the mutable borrow for query
        let num_cols = self.stmt.column_count();

        let mut rows_iter = self
            .stmt
            .query(libsql_rusqlite::params_from_iter(
                libsql_rusqlite_values.iter(),
            ))
            .map_err(|e| DriverError::Statement(e.to_string()))?;

        let mut result_vec = Vec::new();
        while let Some(row_result) = rows_iter
            .next()
            .map_err(|e| DriverError::Query(e.to_string()))?
        {
            let row = row_result; // row is libsql_rusqlite::Row
            // Use the num_cols obtained from the statement
            let mut values = Vec::with_capacity(num_cols);
            for i in 0..num_cols {
                let value_ref = row
                    .get_ref(i)
                    .map_err(|e| DriverError::Query(e.to_string()))?;
                let sqlite_val = map_value_ref_to_sqlite_value(value_ref)
                    .map_err(|e| DriverError::Mapping(e.to_string()))?; // Map DriverError from helper
                values.push(sqlite_val);
            }
            result_vec.push(LibsqlRusqliteRow { values });
        }

        Ok(result_vec)
    }
}

/// Transaction wrapper
pub struct LibsqlRusqliteTransaction<'conn> {
    tx: libsql_rusqlite::Transaction<'conn>,
}

impl<'conn> Transaction<'conn> for LibsqlRusqliteTransaction<'conn> {
    // Value can likely be 'static, but keep 'conn for consistency with Connection for now
    type Value = SQLiteValue<'static>;
    // LibsqlRusqliteRow has no lifetime
    type Row = LibsqlRusqliteRow;
    type QueryResult = Vec<Self::Row>;
    type Error = libsql_rusqlite::Error;
    // Prepared statement now only has 'stmt lifetime
    type Prepared<'stmt>
        = LibsqlRusqlitePreparedStatement<'stmt>
    where
        Self: 'stmt,
        'conn: 'stmt; // Keep 'conn: 'stmt

    fn run_statement(&mut self, sql: &str, params: &[Self::Value]) -> Result<usize, DriverError> {
        let libsql_rusqlite_values = map_params_to_values(params)?;
        self.tx
            .execute(
                sql,
                libsql_rusqlite::params_from_iter(libsql_rusqlite_values.iter()),
            )
            .map_err(|e| DriverError::Transaction(e.to_string()))
    }

    fn query_statement(
        &mut self,
        sql: &str,
        params: &[Self::Value],
    ) -> Result<Self::QueryResult, DriverError> {
        let libsql_rusqlite_values = map_params_to_values(params)?;
        let mut stmt = self
            .tx
            .prepare(sql)
            .map_err(|e| DriverError::Transaction(e.to_string()))?;

        // Get column count from the statement itself
        let num_cols = stmt.column_count();

        let mut rows_iter = stmt
            .query(libsql_rusqlite::params_from_iter(
                libsql_rusqlite_values.iter(),
            ))
            .map_err(|e| DriverError::Transaction(e.to_string()))?;

        let mut result_vec = Vec::new();
        while let Some(row_result) = rows_iter
            .next()
            .map_err(|e| DriverError::Query(e.to_string()))?
        {
            let row = row_result; // row is libsql_rusqlite::Row
            // Use the num_cols obtained from the statement
            let mut values = Vec::with_capacity(num_cols);
            for i in 0..num_cols {
                let value_ref = row
                    .get_ref(i)
                    .map_err(|e| DriverError::Query(e.to_string()))?;
                let sqlite_val = map_value_ref_to_sqlite_value(value_ref)
                    .map_err(|e| DriverError::Mapping(e.to_string()))?; // Map DriverError from helper
                values.push(sqlite_val);
            }
            result_vec.push(LibsqlRusqliteRow { values });
        }

        Ok(result_vec)
    }

    // Add the lifetime bound to match the trait declaration
    fn prepare<'stmt>(&'stmt mut self, sql: &str) -> Result<Self::Prepared<'stmt>, DriverError>
    where
        Self: 'stmt, // Add the missing bound
    {
        let stmt = self
            .tx
            .prepare(sql)
            .map_err(|e| DriverError::Transaction(e.to_string()))?;
        Ok(LibsqlRusqlitePreparedStatement { stmt })
    }

    fn commit(self) -> Result<(), DriverError> {
        self.tx
            .commit()
            .map_err(|e| DriverError::Transaction(e.to_string()))
    }

    fn rollback(self) -> Result<(), DriverError> {
        self.tx
            .rollback()
            .map_err(|e| DriverError::Transaction(e.to_string()))
    }
}

// --- Connection Implementation ---

impl<'conn> Connection for LibsqlRusqliteConnection<'conn> {
    // Value can likely be 'static, but keep 'conn for consistency for now
    type Value = SQLiteValue<'static>;
    // LibsqlRusqliteRow has no lifetime
    type Row = LibsqlRusqliteRow;
    type QueryResult = Vec<Self::Row>;
    type Error = libsql_rusqlite::Error;
    // Transaction wrapper lifetime tied to connection
    type Transaction<'tx>
        = LibsqlRusqliteTransaction<'tx>
    where
        Self: 'tx;
    // Prepared statement now only has 'stmt lifetime
    type Prepared<'stmt>
        = LibsqlRusqlitePreparedStatement<'stmt>
    where
        Self: 'stmt;

    fn run_statement(&self, sql: &str, params: &[Self::Value]) -> Result<usize, DriverError> {
        let libsql_rusqlite_values = map_params_to_values(params)?;
        self.conn
            .execute(
                sql,
                libsql_rusqlite::params_from_iter(libsql_rusqlite_values.iter()),
            )
            .map_err(|e| DriverError::Query(e.to_string()))
    }

    fn query_statement(
        &self,
        sql: &str,
        params: &[Self::Value],
    ) -> Result<Self::QueryResult, DriverError> {
        let libsql_rusqlite_values = map_params_to_values(params)?;
        let mut stmt = self
            .conn
            .prepare(sql)
            .map_err(|e| DriverError::Statement(e.to_string()))?;

        // Get column count from the statement itself
        let num_cols = stmt.column_count();

        let mut rows_iter = stmt
            .query(libsql_rusqlite::params_from_iter(
                libsql_rusqlite_values.iter(),
            ))
            .map_err(|e| DriverError::Query(e.to_string()))?;

        let mut result_vec = Vec::new();
        while let Some(row_result) = rows_iter
            .next()
            .map_err(|e| DriverError::Query(e.to_string()))?
        {
            let row = row_result; // row is libsql_rusqlite::Row
            // Use the num_cols obtained from the statement
            let mut values = Vec::with_capacity(num_cols);
            for i in 0..num_cols {
                let value_ref = row
                    .get_ref(i)
                    .map_err(|e| DriverError::Query(e.to_string()))?;
                let sqlite_val = map_value_ref_to_sqlite_value(value_ref)
                    .map_err(|e| DriverError::Mapping(e.to_string()))?; // Map DriverError from helper
                values.push(sqlite_val);
            }
            result_vec.push(LibsqlRusqliteRow { values });
        }

        Ok(result_vec)
    }

    // Add the lifetime bound to match the trait declaration
    fn prepare<'stmt>(&'stmt self, sql: &str) -> Result<Self::Prepared<'stmt>, DriverError>
    where
        Self: 'stmt, // Add the missing bound
    {
        let stmt = self
            .conn
            .prepare(sql)
            .map_err(|e| DriverError::Statement(e.to_string()))?;
        Ok(LibsqlRusqlitePreparedStatement { stmt })
    }

    // Add the lifetime bound to match the trait declaration
    fn begin_transaction<'tx>(&'tx mut self) -> Result<Self::Transaction<'tx>, DriverError>
    where
        Self: 'tx, // Add the missing bound
    {
        let tx = self
            .conn
            .transaction()
            .map_err(|e| DriverError::Transaction(e.to_string()))?;
        Ok(LibsqlRusqliteTransaction { tx })
    }
}

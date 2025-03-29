#![cfg(feature = "libsql")]

use super::{Connection, DbRow, DriverError, PreparedStatement, SQLiteValue, Transaction};

use libsql::{
    Connection as NativeLibsqlConnection, Row as LibsqlRowTrait, Value as LibsqlValue,
    ValueRef as LibsqlValueRef, params_from_iter,
};
use std::borrow::Cow;
use std::cell::RefCell;
use std::marker::PhantomData;

// --- Helper Functions ---

fn map_params_to_values<'a>(
    params: &'a [SQLiteValue<'a>],
) -> Result<Vec<LibsqlValue>, DriverError> {
    params
        .iter()
        .map(|p| match p {
            SQLiteValue::Null => Ok(LibsqlValue::Null),
            SQLiteValue::Integer(i) => Ok(LibsqlValue::Integer(*i)),
            SQLiteValue::Real(f) => Ok(LibsqlValue::Real(*f)),
            SQLiteValue::Text(s) => Ok(LibsqlValue::Text(s.to_string())), // Clone Cow -> String
            SQLiteValue::Blob(b) => Ok(LibsqlValue::Blob(b.to_vec())),    // Clone Cow -> Vec<u8>
        })
        .collect::<Result<Vec<_>, _>>()
}

fn map_value_ref_to_sqlite_value(
    value_ref: LibsqlValueRef,
) -> Result<SQLiteValue<'static>, DriverError> {
    match value_ref {
        LibsqlValueRef::Null => Ok(SQLiteValue::Null),
        LibsqlValueRef::Integer(i) => Ok(SQLiteValue::Integer(i)),
        LibsqlValueRef::Real(f) => Ok(SQLiteValue::Real(f)),
        LibsqlValueRef::Text(t_bytes) => {
            // Need to convert &[u8] to String for Text
            String::from_utf8(t_bytes.to_vec())
                .map(|s| SQLiteValue::Text(Cow::Owned(s)))
                .map_err(|e| DriverError::Mapping(format!("UTF-8 error converting text: {}", e)))
        }
        LibsqlValueRef::Blob(b_bytes) => Ok(SQLiteValue::Blob(Cow::Owned(b_bytes.to_vec()))),
    }
}

// --- Wrapper Structs ---

pub struct LibsqlConnection {
    conn: RefCell<NativeLibsqlConnection>, // Use RefCell for interior mutability
}

impl LibsqlConnection {
    pub fn new(conn: NativeLibsqlConnection) -> Self {
        Self {
            conn: RefCell::new(conn),
        }
    }
}

#[derive(Debug)]
pub struct LibsqlRow {
    // Store owned values, similar to RusqliteRow
    values: Vec<SQLiteValue<'static>>,
}

pub struct LibsqlPreparedStatement<'stmt> {
    stmt: libsql::Statement<'stmt>,
    _marker: PhantomData<&'stmt ()>,
}

pub struct LibsqlTransaction<'conn> {
    tx: libsql::Transaction<'conn>,
}

// --- Trait Implementations ---

impl DbRow for LibsqlRow {
    fn get(&self, index: usize) -> Result<SQLiteValue<'static>, DriverError> {
        self.values
            .get(index)
            .cloned()
            .ok_or_else(|| DriverError::Mapping(format!("Index {} out of bounds", index)))
    }
}

impl<'stmt> PreparedStatement<'stmt> for LibsqlPreparedStatement<'stmt> {
    type Row = LibsqlRow;
    type Value = SQLiteValue<'stmt>;
    type QueryResult = Vec<Self::Row>;
    type Error = DriverError;

    fn run(&mut self, params: &[Self::Value]) -> Result<usize, DriverError> {
        let libsql_values = map_params_to_values(params)?;
        let changes = self
            .stmt
            .execute(params_from_iter(libsql_values))
            .map_err(|e| DriverError::Statement(e.to_string()))?;
        Ok(changes as usize)
    }

    fn query(&mut self, params: &[Self::Value]) -> Result<Self::QueryResult, DriverError> {
        let libsql_values = map_params_to_values(params)?;
        let num_cols = self.stmt.column_count();
        let mut rows = self
            .stmt
            .query(params_from_iter(libsql_values))
            .map_err(|e| DriverError::Statement(e.to_string()))?;

        let mut result_vec = Vec::new();
        while let Some(row_result) = rows.next().map_err(|e| DriverError::Query(e.to_string()))? {
            let row = row_result;
            let mut values = Vec::with_capacity(num_cols);
            for i in 0..num_cols {
                let value_ref = row
                    .get_value_ref(i)
                    .map_err(|e| DriverError::Query(e.to_string()))?;
                let sqlite_val = map_value_ref_to_sqlite_value(value_ref)?;
                values.push(sqlite_val);
            }
            result_vec.push(LibsqlRow { values });
        }
        Ok(result_vec)
    }
}

impl<'conn> Transaction<'conn> for LibsqlTransaction<'conn> {
    type Row = LibsqlRow;
    type Value = SQLiteValue<'conn>;
    type QueryResult = Vec<Self::Row>;
    type Error = DriverError;
    type Prepared<'stmt>
        = LibsqlPreparedStatement<'stmt>
    where
        Self: 'stmt,
        'conn: 'stmt;

    fn run_statement(&mut self, sql: &str, params: &[Self::Value]) -> Result<usize, DriverError> {
        let libsql_values = map_params_to_values(params)?;
        let changes = self
            .tx
            .execute(sql, params_from_iter(libsql_values))
            .map_err(|e| DriverError::Transaction(e.to_string()))?;
        Ok(changes as usize)
    }

    fn query_statement(
        &mut self,
        sql: &str,
        params: &[Self::Value],
    ) -> Result<Self::QueryResult, DriverError> {
        let libsql_values = map_params_to_values(params)?;
        let mut stmt = self
            .tx
            .prepare(sql)
            .map_err(|e| DriverError::Transaction(e.to_string()))?;
        let num_cols = stmt.column_count();
        let mut rows = stmt
            .query(params_from_iter(libsql_values))
            .map_err(|e| DriverError::Transaction(e.to_string()))?;

        let mut result_vec = Vec::new();
        while let Some(row_result) = rows.next().map_err(|e| DriverError::Query(e.to_string()))? {
            let row = row_result;
            let mut values = Vec::with_capacity(num_cols);
            for i in 0..num_cols {
                let value_ref = row
                    .get_value_ref(i)
                    .map_err(|e| DriverError::Query(e.to_string()))?;
                let sqlite_val = map_value_ref_to_sqlite_value(value_ref)?;
                values.push(sqlite_val);
            }
            result_vec.push(LibsqlRow { values });
        }
        Ok(result_vec)
    }

    fn prepare<'stmt>(&'stmt mut self, sql: &str) -> Result<Self::Prepared<'stmt>, DriverError>
    where
        Self: 'stmt,
    {
        let stmt = self
            .tx
            .prepare(sql)
            .map_err(|e| DriverError::Transaction(e.to_string()))?;
        Ok(LibsqlPreparedStatement {
            stmt,
            _marker: PhantomData,
        })
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

impl Connection for LibsqlConnection {
    type Value = SQLiteValue<'static>;
    type Row = LibsqlRow;
    type QueryResult = Vec<Self::Row>;
    type Error = DriverError;
    type Transaction<'tx>
        = LibsqlTransaction<'tx>
    where
        Self: 'tx;
    type Prepared<'stmt>
        = LibsqlPreparedStatement<'stmt>
    where
        Self: 'stmt;

    fn run_statement(&self, sql: &str, params: &[Self::Value]) -> Result<usize, DriverError> {
        let libsql_values = map_params_to_values(params)?;
        let conn = self.conn.borrow();
        let changes = conn
            .execute(sql, params_from_iter(libsql_values))
            .map_err(|e| DriverError::Query(e.to_string()))?;
        Ok(changes as usize)
    }

    fn query_statement(
        &self,
        sql: &str,
        params: &[Self::Value],
    ) -> Result<Self::QueryResult, DriverError> {
        let libsql_values = map_params_to_values(params)?;
        let conn = self.conn.borrow();
        let mut stmt = conn
            .prepare(sql)
            .map_err(|e| DriverError::Statement(e.to_string()))?;
        let num_cols = stmt.column_count();
        let mut rows = stmt
            .query(params_from_iter(libsql_values))
            .map_err(|e| DriverError::Query(e.to_string()))?;

        let mut result_vec = Vec::new();
        while let Some(row_result) = rows.next().map_err(|e| DriverError::Query(e.to_string()))? {
            let row = row_result;
            let mut values = Vec::with_capacity(num_cols);
            for i in 0..num_cols {
                let value_ref = row
                    .get_value_ref(i)
                    .map_err(|e| DriverError::Query(e.to_string()))?;
                let sqlite_val = map_value_ref_to_sqlite_value(value_ref)?;
                values.push(sqlite_val);
            }
            result_vec.push(LibsqlRow { values });
        }
        Ok(result_vec)
    }

    fn prepare<'stmt>(&'stmt self, sql: &str) -> Result<Self::Prepared<'stmt>, DriverError>
    where
        Self: 'stmt,
    {
        let conn = self.conn.borrow();
        let stmt = conn
            .prepare(sql)
            .map_err(|e| DriverError::Statement(e.to_string()))?;
        Ok(LibsqlPreparedStatement {
            stmt,
            _marker: PhantomData,
        })
    }

    fn begin_transaction<'tx>(&'tx self) -> Result<Self::Transaction<'tx>, DriverError>
    where
        Self: 'tx,
    {
        let mut conn_mut = self.conn.borrow_mut();
        let tx = conn_mut
            .transaction()
            .map_err(|e| DriverError::Transaction(e.to_string()))?;
        Ok(LibsqlTransaction { tx })
    }
}

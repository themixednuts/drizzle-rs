/// Simplified test helpers that eliminate code duplication across database drivers
///
/// This module provides macros and helpers that make it easy to write tests that work
/// across rusqlite, turso, and libsql without duplicating code.

/// Macro to set up database connection with proper async/sync handling
#[macro_export]
macro_rules! setup_test_db {
    () => {{
        #[cfg(feature = "rusqlite")]
        let conn = crate::common::setup_db();
        #[cfg(any(feature = "turso", feature = "libsql"))]
        let conn = crate::common::setup_db().await;
        #[cfg(any(feature = "turso", feature = "libsql", feature = "rusqlite"))]
        conn
    }};
}

/// Macro to execute drizzle operations with proper async/sync handling
#[macro_export]
macro_rules! drizzle_exec {
    ($operation:expr) => {{
        #[cfg(feature = "rusqlite")]
        let result = $operation.unwrap();
        #[cfg(any(feature = "turso", feature = "libsql"))]
        let result = $operation.await.unwrap();
        #[cfg(any(feature = "turso", feature = "libsql", feature = "rusqlite"))]
        result
    }};
}

/// Macro to execute drizzle operations that can fail with proper async/sync handling  
#[macro_export]
macro_rules! drizzle_try {
    ($operation:expr) => {{
        #[cfg(feature = "rusqlite")]
        let result = $operation;
        #[cfg(any(feature = "turso", feature = "libsql"))]
        let result = $operation.await;
        #[cfg(any(feature = "turso", feature = "libsql", feature = "rusqlite"))]
        result
    }};
}

/// Macro to prepare SQL statements with proper async/sync handling
#[macro_export]
macro_rules! prepare_stmt {
    ($conn:expr, $query:expr) => {{
        #[cfg(feature = "rusqlite")]
        let stmt = $conn.prepare($query).unwrap();
        #[cfg(any(feature = "turso", feature = "libsql"))]
        let stmt = $conn.prepare($query).await.unwrap();
        #[cfg(any(feature = "turso", feature = "libsql", feature = "rusqlite"))]
        stmt
    }};
}

/// Macro to execute raw SQL with proper async/sync handling
#[macro_export]
macro_rules! exec_sql {
    ($conn:expr, $query:expr, $params:expr) => {{
        #[cfg(feature = "rusqlite")]
        let result = $conn.execute($query, $params).unwrap();
        #[cfg(any(feature = "turso", feature = "libsql"))]
        let result = $conn.execute($query, $params).await.unwrap();
        #[cfg(any(feature = "turso", feature = "libsql", feature = "rusqlite"))]
        result
    }};
}

/// Macro to query a single row with proper async/sync handling
#[macro_export]
macro_rules! query_row {
    ($stmt:expr, $params:expr, $mapper:expr) => {{
        #[cfg(feature = "rusqlite")]
        let result = $stmt
            .query_row($params, |row| -> Result<(), rusqlite::Error> {
                $mapper(row);
                Ok(())
            })
            .unwrap();
        #[cfg(any(feature = "turso", feature = "libsql"))]
        let result = {
            let mut rows = $stmt.query($params).await.unwrap();
            if let Some(row) = rows.next().await.unwrap() {
                $mapper(&row)
            } else {
                panic!("No rows returned");
            }
        };
        #[cfg(any(feature = "turso", feature = "libsql", feature = "rusqlite"))]
        result
    }};
}

/// Helper macro for database parameters
#[macro_export]
macro_rules! db_params {
    () => {{
        #[cfg(feature = "rusqlite")]
        let params = [];
        #[cfg(feature = "turso")]
        let params = ();
        #[cfg(feature = "libsql")]
        let params = ();
        #[cfg(any(feature = "turso", feature = "libsql", feature = "rusqlite"))]
        params
    }};
    ($($param:expr),*) => {{
        #[cfg(feature = "rusqlite")]
        let params = rusqlite::params![$($param),*];
        #[cfg(feature = "turso")]
        let params = turso::params![$($param),*];
        #[cfg(feature = "libsql")]
        let params = libsql::params![$($param),*];
        #[cfg(any(feature = "turso", feature = "libsql", feature = "rusqlite"))]
        params
    }};
}

/// Helper to extract values from database rows in a unified way
pub struct RowHelper;

impl RowHelper {
    /// Extract string value from row at index
    #[cfg(feature = "rusqlite")]
    pub fn get_string(row: &rusqlite::Row, index: usize) -> String {
        row.get::<_, String>(index).unwrap()
    }

    #[cfg(feature = "libsql")]
    pub fn get_string(row: &libsql::Row, index: usize) -> String {
        row.get_value(index as i32)
            .unwrap()
            .as_text()
            .unwrap()
            .to_string()
    }
    #[cfg(feature = "turso")]
    pub fn get_string(row: &turso::Row, index: usize) -> String {
        row.get_value(index).unwrap().as_text().unwrap().to_string()
    }

    /// Extract integer value from row at index
    #[cfg(feature = "rusqlite")]
    pub fn get_i32(row: &rusqlite::Row, index: usize) -> i32 {
        row.get::<_, i32>(index).unwrap()
    }

    #[cfg(feature = "libsql")]
    pub fn get_i32(row: &libsql::Row, index: usize) -> i32 {
        row.get_value(index as i32)
            .unwrap()
            .as_integer()
            .unwrap()
            .clone() as i32
    }
    #[cfg(feature = "turso")]
    pub fn get_i32(row: &turso::Row, index: usize) -> i32 {
        row.get_value(index).unwrap().as_integer().unwrap().clone() as i32
    }

    /// Extract i64 value from row at index
    #[cfg(feature = "rusqlite")]
    pub fn get_i64(row: &rusqlite::Row, index: usize) -> i64 {
        row.get::<_, i64>(index).unwrap()
    }

    #[cfg(feature = "libsql")]
    pub fn get_i64(row: &libsql::Row, index: usize) -> i64 {
        row.get_value(index as i32)
            .unwrap()
            .as_integer()
            .unwrap()
            .clone()
    }
    #[cfg(feature = "turso")]
    pub fn get_i64(row: &turso::Row, index: usize) -> i64 {
        row.get_value(index).unwrap().as_integer().unwrap().clone()
    }

    /// Extract f64 value from row at index
    #[cfg(feature = "rusqlite")]
    pub fn get_f64(row: &rusqlite::Row, index: usize) -> f64 {
        row.get::<_, f64>(index).unwrap()
    }

    #[cfg(feature = "libsql")]
    pub fn get_f64(row: &libsql::Row, index: usize) -> f64 {
        row.get_value(index as i32)
            .unwrap()
            .as_real()
            .unwrap()
            .clone()
    }
    #[cfg(feature = "turso")]
    pub fn get_f64(row: &turso::Row, index: usize) -> f64 {
        row.get_value(index).unwrap().as_real().unwrap().clone()
    }

    /// Extract bool value from row at index
    #[cfg(feature = "rusqlite")]
    pub fn get_bool(row: &rusqlite::Row, index: usize) -> bool {
        row.get::<_, bool>(index).unwrap()
    }

    #[cfg(feature = "libsql")]
    pub fn get_bool(row: &libsql::Row, index: usize) -> bool {
        row.get_value(index as i32)
            .unwrap()
            .as_integer()
            .map(|&v| v != 0)
            .unwrap()
    }
    #[cfg(feature = "turso")]
    pub fn get_bool(row: &turso::Row, index: usize) -> bool {
        row.get_value(index)
            .unwrap()
            .as_integer()
            .map(|&v| v != 0)
            .unwrap()
    }

    /// Extract Vec<u8> value from row at index
    #[cfg(feature = "rusqlite")]
    pub fn get_blob(row: &rusqlite::Row, index: usize) -> Vec<u8> {
        row.get::<_, Vec<u8>>(index).unwrap()
    }

    #[cfg(feature = "libsql")]
    pub fn get_blob(row: &libsql::Row, index: usize) -> Vec<u8> {
        row.get_value(index as i32)
            .unwrap()
            .as_blob()
            .unwrap()
            .clone()
    }
    #[cfg(feature = "turso")]
    pub fn get_blob(row: &turso::Row, index: usize) -> Vec<u8> {
        row.get_value(index).unwrap().as_blob().unwrap().clone()
    }
}

/// Macro to extract values from rows uniformly
#[macro_export]
macro_rules! row_get {
    ($row:expr, $index:expr, String) => {
        crate::common::helpers::RowHelper::get_string($row, $index)
    };
    ($row:expr, $index:expr, i32) => {
        crate::common::helpers::RowHelper::get_i32($row, $index)
    };
    ($row:expr, $index:expr, i64) => {
        crate::common::helpers::RowHelper::get_i64($row, $index)
    };
    ($row:expr, $index:expr, f64) => {
        crate::common::helpers::RowHelper::get_f64($row, $index)
    };
    ($row:expr, $index:expr, bool) => {
        crate::common::helpers::RowHelper::get_bool($row, $index)
    };
    ($row:expr, $index:expr, Vec<u8>) => {
        crate::common::helpers::RowHelper::get_blob($row, $index)
    };
}

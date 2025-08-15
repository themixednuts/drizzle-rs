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

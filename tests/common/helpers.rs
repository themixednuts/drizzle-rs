#[cfg(feature = "rusqlite")]
pub mod rusqlite_setup {
    use drizzle_rs::rusqlite::Drizzle;
    use rusqlite::Connection;

    pub fn setup_db<S: Default + drizzle_rs::core::SQLSchemaImpl>() -> (Drizzle<S>, S) {
        let conn = Connection::open_in_memory().expect("Failed to create in-memory database");
        let schema = S::default();
        let (db, schema) = Drizzle::new(conn, schema);
        db.create().expect("Failed to create schema tables");
        (db, schema)
    }
}

#[cfg(feature = "libsql")]
pub mod libsql_setup {
    use drizzle_rs::libsql::Drizzle;
    use libsql::{Builder, Connection};

    pub async fn setup_db<S: Default + drizzle_rs::core::SQLSchemaImpl>() -> (Drizzle<S>, S) {
        let db = Builder::new_local(":memory:")
            .build()
            .await
            .expect("build db");
        let conn = db.connect().expect("connect to db");
        let schema = S::default();
        let (db, schema) = Drizzle::new(conn, schema);
        db.create().await.expect("Failed to create schema tables");
        (db, schema)
    }
}

#[cfg(feature = "turso")]
pub mod turso_setup {
    use drizzle_rs::turso::Drizzle;
    use turso::Builder;

    pub async fn setup_db<S: Default + drizzle_rs::core::SQLSchemaImpl>() -> (Drizzle<S>, S) {
        let db = Builder::new_local(":memory:")
            .build()
            .await
            .expect("build db");
        let conn = db.connect().expect("connect to db");
        let schema = S::default();
        let (db, schema) = Drizzle::new(conn, schema);
        db.create().await.expect("Failed to create schema tables");
        (db, schema)
    }
}

// test_all_drivers macro replaced with drivers_test proc macro in drizzle-macros crate

// Driver-specific macros are now injected by test_all_drivers! macro

// prepare_stmt macro is no longer needed - use direct drizzle queries instead

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
    /// Extract string value from row at index - uses function overloading per driver
    #[cfg(feature = "rusqlite")]
    pub fn get_string_rusqlite(row: &rusqlite::Row, index: usize) -> String {
        row.get::<_, String>(index).unwrap()
    }

    #[cfg(feature = "libsql")]
    pub fn get_string_libsql(row: &libsql::Row, index: usize) -> String {
        row.get_value(index as i32)
            .unwrap()
            .as_text()
            .unwrap()
            .to_string()
    }

    #[cfg(feature = "turso")]
    pub fn get_string_turso(row: &turso::Row, index: usize) -> String {
        row.get_value(index).unwrap().as_text().unwrap().to_string()
    }

    /// Extract integer value from row at index
    #[cfg(feature = "rusqlite")]
    pub fn get_i32_rusqlite(row: &rusqlite::Row, index: usize) -> i32 {
        row.get::<_, i32>(index).unwrap()
    }

    #[cfg(feature = "libsql")]
    pub fn get_i32_libsql(row: &libsql::Row, index: usize) -> i32 {
        row.get_value(index as i32)
            .unwrap()
            .as_integer()
            .unwrap()
            .clone() as i32
    }

    #[cfg(feature = "turso")]
    pub fn get_i32_turso(row: &turso::Row, index: usize) -> i32 {
        row.get_value(index).unwrap().as_integer().unwrap().clone() as i32
    }

    /// Extract i64 value from row at index
    #[cfg(feature = "rusqlite")]
    pub fn get_i64_rusqlite(row: &rusqlite::Row, index: usize) -> i64 {
        row.get::<_, i64>(index).unwrap()
    }

    #[cfg(feature = "libsql")]
    pub fn get_i64_libsql(row: &libsql::Row, index: usize) -> i64 {
        row.get_value(index as i32)
            .unwrap()
            .as_integer()
            .unwrap()
            .clone()
    }

    #[cfg(feature = "turso")]
    pub fn get_i64_turso(row: &turso::Row, index: usize) -> i64 {
        row.get_value(index).unwrap().as_integer().unwrap().clone()
    }

    /// Extract f64 value from row at index
    #[cfg(feature = "rusqlite")]
    pub fn get_f64_rusqlite(row: &rusqlite::Row, index: usize) -> f64 {
        row.get::<_, f64>(index).unwrap()
    }

    #[cfg(feature = "libsql")]
    pub fn get_f64_libsql(row: &libsql::Row, index: usize) -> f64 {
        row.get_value(index as i32)
            .unwrap()
            .as_real()
            .unwrap()
            .clone()
    }

    #[cfg(feature = "turso")]
    pub fn get_f64_turso(row: &turso::Row, index: usize) -> f64 {
        row.get_value(index).unwrap().as_real().unwrap().clone()
    }

    /// Extract bool value from row at index
    #[cfg(feature = "rusqlite")]
    pub fn get_bool_rusqlite(row: &rusqlite::Row, index: usize) -> bool {
        row.get::<_, bool>(index).unwrap()
    }

    #[cfg(feature = "libsql")]
    pub fn get_bool_libsql(row: &libsql::Row, index: usize) -> bool {
        row.get_value(index as i32)
            .unwrap()
            .as_integer()
            .map(|&v| v != 0)
            .unwrap()
    }

    #[cfg(feature = "turso")]
    pub fn get_bool_turso(row: &turso::Row, index: usize) -> bool {
        row.get_value(index)
            .unwrap()
            .as_integer()
            .map(|&v| v != 0)
            .unwrap()
    }

    /// Extract Vec<u8> value from row at index
    #[cfg(feature = "rusqlite")]
    pub fn get_blob_rusqlite(row: &rusqlite::Row, index: usize) -> Vec<u8> {
        row.get::<_, Vec<u8>>(index).unwrap()
    }

    #[cfg(feature = "libsql")]
    pub fn get_blob_libsql(row: &libsql::Row, index: usize) -> Vec<u8> {
        row.get_value(index as i32)
            .unwrap()
            .as_blob()
            .unwrap()
            .clone()
    }

    #[cfg(feature = "turso")]
    pub fn get_blob_turso(row: &turso::Row, index: usize) -> Vec<u8> {
        row.get_value(index).unwrap().as_blob().unwrap().clone()
    }
}

/// Macro to extract values from rows uniformly
#[macro_export]
macro_rules! row_get {
    ($row:expr, $index:expr, String) => {{
        #[cfg(feature = "rusqlite")]
        {
            crate::common::helpers::RowHelper::get_string_rusqlite($row, $index)
        }
        #[cfg(all(feature = "libsql", not(feature = "rusqlite")))]
        {
            crate::common::helpers::RowHelper::get_string_libsql($row, $index)
        }
        #[cfg(all(feature = "turso", not(feature = "rusqlite"), not(feature = "libsql")))]
        {
            crate::common::helpers::RowHelper::get_string_turso($row, $index)
        }
    }};
    ($row:expr, $index:expr, i32) => {{
        #[cfg(feature = "rusqlite")]
        {
            crate::common::helpers::RowHelper::get_i32_rusqlite($row, $index)
        }
        #[cfg(all(feature = "libsql", not(feature = "rusqlite")))]
        {
            crate::common::helpers::RowHelper::get_i32_libsql($row, $index)
        }
        #[cfg(all(feature = "turso", not(feature = "rusqlite"), not(feature = "libsql")))]
        {
            crate::common::helpers::RowHelper::get_i32_turso($row, $index)
        }
    }};
    ($row:expr, $index:expr, i64) => {{
        #[cfg(feature = "rusqlite")]
        {
            crate::common::helpers::RowHelper::get_i64_rusqlite($row, $index)
        }
        #[cfg(all(feature = "libsql", not(feature = "rusqlite")))]
        {
            crate::common::helpers::RowHelper::get_i64_libsql($row, $index)
        }
        #[cfg(all(feature = "turso", not(feature = "rusqlite"), not(feature = "libsql")))]
        {
            crate::common::helpers::RowHelper::get_i64_turso($row, $index)
        }
    }};
    ($row:expr, $index:expr, f64) => {{
        #[cfg(feature = "rusqlite")]
        {
            crate::common::helpers::RowHelper::get_f64_rusqlite($row, $index)
        }
        #[cfg(all(feature = "libsql", not(feature = "rusqlite")))]
        {
            crate::common::helpers::RowHelper::get_f64_libsql($row, $index)
        }
        #[cfg(all(feature = "turso", not(feature = "rusqlite"), not(feature = "libsql")))]
        {
            crate::common::helpers::RowHelper::get_f64_turso($row, $index)
        }
    }};
    ($row:expr, $index:expr, bool) => {{
        #[cfg(feature = "rusqlite")]
        {
            crate::common::helpers::RowHelper::get_bool_rusqlite($row, $index)
        }
        #[cfg(all(feature = "libsql", not(feature = "rusqlite")))]
        {
            crate::common::helpers::RowHelper::get_bool_libsql($row, $index)
        }
        #[cfg(all(feature = "turso", not(feature = "rusqlite"), not(feature = "libsql")))]
        {
            crate::common::helpers::RowHelper::get_bool_turso($row, $index)
        }
    }};
    ($row:expr, $index:expr, Vec<u8>) => {{
        #[cfg(feature = "rusqlite")]
        {
            crate::common::helpers::RowHelper::get_blob_rusqlite($row, $index)
        }
        #[cfg(all(feature = "libsql", not(feature = "rusqlite")))]
        {
            crate::common::helpers::RowHelper::get_blob_libsql($row, $index)
        }
        #[cfg(all(feature = "turso", not(feature = "rusqlite"), not(feature = "libsql")))]
        {
            crate::common::helpers::RowHelper::get_blob_turso($row, $index)
        }
    }};
}

#[cfg(feature = "rusqlite")]
pub mod rusqlite_setup {
    use drizzle::rusqlite::Drizzle;
    use rusqlite::Connection;

    pub fn setup_db<S: Default + drizzle::core::SQLSchemaImpl>() -> (Drizzle<S>, S) {
        let conn = Connection::open_in_memory().expect("Failed to create in-memory database");
        let schema = S::default();
        let (db, schema) = Drizzle::new(conn, schema);
        db.create().expect("Failed to create schema tables");
        (db, schema)
    }
}

#[cfg(feature = "libsql")]
pub mod libsql_setup {
    use drizzle::libsql::Drizzle;
    use libsql::{Builder, Connection};

    pub async fn setup_db<S: Default + drizzle::core::SQLSchemaImpl>() -> (Drizzle<S>, S) {
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
    use drizzle::turso::Drizzle;
    use turso::Builder;

    pub async fn setup_db<S: Default + drizzle::core::SQLSchemaImpl>() -> (Drizzle<S>, S) {
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

#[cfg(feature = "postgres-sync")]
pub mod postgres_sync_setup {
    use drizzle::postgres_sync::Drizzle;
    use postgres::{Client, NoTls};
    use std::ops::{Deref, DerefMut};
    use std::process::Command;
    use std::sync::Once;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::thread;
    use std::time::Duration;

    static DOCKER_STARTED: Once = Once::new();
    static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

    fn get_database_url() -> String {
        std::env::var("DATABASE_URL").unwrap_or_else(|_| {
            "host=localhost user=postgres password=postgres dbname=drizzle_test".to_string()
        })
    }

    fn ensure_postgres_running() {
        DOCKER_STARTED.call_once(|| {
            let database_url = get_database_url();

            // Try to connect first
            if Client::connect(&database_url, NoTls).is_ok() {
                println!("✅ PostgreSQL already running");
                return;
            }

            println!("🐳 Starting PostgreSQL via Docker Compose...");

            // Start docker compose
            let status = Command::new("docker")
                .args(["compose", "up", "-d", "postgres"])
                .status();

            match status {
                Ok(s) if s.success() => {
                    // Wait for PostgreSQL to be ready
                    println!("⏳ Waiting for PostgreSQL to be ready...");
                    for i in 0..30 {
                        thread::sleep(Duration::from_secs(1));
                        if Client::connect(&database_url, NoTls).is_ok() {
                            println!("✅ PostgreSQL is ready! (took {}s)", i + 1);
                            return;
                        }
                    }
                    panic!("PostgreSQL failed to start within 30 seconds");
                }
                Ok(_) => {
                    eprintln!("⚠️  Docker Compose failed. Make sure Docker is running.");
                    eprintln!("   You can manually start with: docker compose up -d postgres");
                }
                Err(e) => {
                    eprintln!("⚠️  Could not run docker compose: {}", e);
                    eprintln!("   Make sure Docker is installed and running.");
                }
            }
        });
    }

    /// Generate a unique schema name for this test
    fn generate_schema_name() -> String {
        let counter = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
        let thread_id = format!("{:?}", thread::current().id());
        // Extract just the number from ThreadId(X)
        let thread_num: String = thread_id.chars().filter(|c| c.is_ascii_digit()).collect();
        format!("test_{}_{}", thread_num, counter)
    }

    /// Wrapper around Drizzle that automatically cleans up its schema on drop.
    /// This enables parallel test execution by giving each test its own isolated schema.
    pub struct TestDb<S> {
        pub db: Drizzle<S>,
        schema_name: String,
    }

    impl<S> Deref for TestDb<S> {
        type Target = Drizzle<S>;
        fn deref(&self) -> &Self::Target {
            &self.db
        }
    }

    impl<S> DerefMut for TestDb<S> {
        fn deref_mut(&mut self) -> &mut Self::Target {
            &mut self.db
        }
    }

    impl<S> Drop for TestDb<S> {
        fn drop(&mut self) {
            // Open a new connection to drop the schema (original is owned by Drizzle)
            if let Ok(mut client) = Client::connect(&get_database_url(), NoTls) {
                let drop_sql = format!("DROP SCHEMA IF EXISTS \"{}\" CASCADE", self.schema_name);
                if let Err(e) = client.batch_execute(&drop_sql) {
                    eprintln!("⚠️  Failed to drop test schema {}: {}", self.schema_name, e);
                }
            }
        }
    }

    pub fn setup_db<S: Default + drizzle::core::SQLSchemaImpl>() -> (TestDb<S>, S) {
        // Ensure PostgreSQL is running (auto-starts via Docker if needed)
        ensure_postgres_running();

        let database_url = get_database_url();
        let schema_name = generate_schema_name();

        let mut client =
            Client::connect(&database_url, NoTls).expect("Failed to connect to PostgreSQL");

        // Create isolated schema for this test and set search_path
        let setup_sql = format!(
            "DROP SCHEMA IF EXISTS \"{}\" CASCADE; CREATE SCHEMA \"{}\"; SET search_path TO \"{}\"",
            schema_name, schema_name, schema_name
        );
        client
            .batch_execute(&setup_sql)
            .expect("Failed to create test schema");

        let schema = S::default();
        let (mut db, schema) = Drizzle::new(client, schema);
        db.create().expect("Failed to create schema tables");

        let test_db = TestDb { db, schema_name };
        (test_db, schema)
    }
}

#[cfg(feature = "tokio-postgres")]
pub mod tokio_postgres_setup {
    use drizzle::tokio_postgres::Drizzle;
    use std::ops::{Deref, DerefMut};
    use std::process::Command;
    use std::sync::Once;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::thread;
    use std::time::Duration;
    use tokio_postgres::NoTls;

    static DOCKER_STARTED: Once = Once::new();
    static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

    fn get_database_url() -> String {
        std::env::var("DATABASE_URL").unwrap_or_else(|_| {
            "host=localhost user=postgres password=postgres dbname=drizzle_test".to_string()
        })
    }

    /// Check if postgres is reachable (runs on a separate thread with its own runtime)
    fn check_postgres_available(database_url: &str) -> bool {
        let url = database_url.to_string();
        thread::spawn(move || {
            let rt = match tokio::runtime::Runtime::new() {
                Ok(rt) => rt,
                Err(_) => return false,
            };
            rt.block_on(async move { tokio_postgres::connect(&url, NoTls).await.is_ok() })
        })
        .join()
        .unwrap_or(false)
    }

    fn ensure_postgres_running() {
        DOCKER_STARTED.call_once(|| {
            let database_url = get_database_url();

            // Try to connect using tokio-postgres on separate thread
            if check_postgres_available(&database_url) {
                println!("✅ PostgreSQL already running");
                return;
            }

            println!("🐳 Starting PostgreSQL via Docker Compose...");

            let status = Command::new("docker")
                .args(["compose", "up", "-d", "postgres"])
                .status();

            match status {
                Ok(s) if s.success() => {
                    println!("⏳ Waiting for PostgreSQL to be ready...");
                    for i in 0..30 {
                        thread::sleep(Duration::from_secs(1));
                        if check_postgres_available(&database_url) {
                            println!("✅ PostgreSQL is ready! (took {}s)", i + 1);
                            return;
                        }
                    }
                    panic!("PostgreSQL failed to start within 30 seconds");
                }
                Ok(_) => {
                    eprintln!("⚠️  Docker Compose failed. Make sure Docker is running.");
                    eprintln!("   You can manually start with: docker compose up -d postgres");
                }
                Err(e) => {
                    eprintln!("⚠️  Could not run docker compose: {}", e);
                    eprintln!("   Make sure Docker is installed and running.");
                }
            }
        });
    }

    fn generate_schema_name() -> String {
        let counter = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
        let thread_id = format!("{:?}", thread::current().id());
        let thread_num: String = thread_id.chars().filter(|c| c.is_ascii_digit()).collect();
        format!("test_async_{}_{}", thread_num, counter)
    }

    /// Wrapper around Drizzle that automatically cleans up its schema on drop.
    pub struct TestDb<S> {
        pub db: Drizzle<S>,
        schema_name: String,
    }

    impl<S> Deref for TestDb<S> {
        type Target = Drizzle<S>;
        fn deref(&self) -> &Self::Target {
            &self.db
        }
    }

    impl<S> DerefMut for TestDb<S> {
        fn deref_mut(&mut self) -> &mut Self::Target {
            &mut self.db
        }
    }

    impl<S> Drop for TestDb<S> {
        fn drop(&mut self) {
            let schema_name = self.schema_name.clone();
            let database_url = get_database_url();

            // Spawn a thread with its own tokio runtime for async cleanup
            let _ = thread::spawn(move || {
                let rt = tokio::runtime::Runtime::new().expect("Failed to create cleanup runtime");
                rt.block_on(async move {
                    if let Ok((client, connection)) =
                        tokio_postgres::connect(&database_url, NoTls).await
                    {
                        // Spawn connection handler (fire and forget)
                        tokio::spawn(async move {
                            let _ = connection.await;
                        });

                        let drop_sql = format!("DROP SCHEMA IF EXISTS \"{}\" CASCADE", schema_name);
                        if let Err(e) = client.batch_execute(&drop_sql).await {
                            eprintln!("⚠️  Failed to drop test schema {}: {}", schema_name, e);
                        }
                    }
                });
            })
            .join();
        }
    }

    pub async fn setup_db<S: Default + drizzle::core::SQLSchemaImpl>() -> (TestDb<S>, S) {
        // Ensure PostgreSQL is running (auto-starts via Docker if needed)
        ensure_postgres_running();

        let database_url = get_database_url();
        let schema_name = generate_schema_name();

        // Connect using tokio-postgres
        let (client, connection) = tokio_postgres::connect(&database_url, NoTls)
            .await
            .expect("Failed to connect to PostgreSQL");

        // Spawn the connection handler
        tokio::spawn(async move {
            if let Err(e) = connection.await {
                eprintln!("PostgreSQL connection error: {}", e);
            }
        });

        // Create isolated schema for this test and set search_path
        let setup_sql = format!(
            "DROP SCHEMA IF EXISTS \"{}\" CASCADE; CREATE SCHEMA \"{}\"; SET search_path TO \"{}\"",
            schema_name, schema_name, schema_name
        );
        client
            .batch_execute(&setup_sql)
            .await
            .expect("Failed to create test schema");

        let schema = S::default();
        let (mut db, schema) = Drizzle::new(client, schema);
        db.create().await.expect("Failed to create schema tables");

        let test_db = TestDb { db, schema_name };
        (test_db, schema)
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

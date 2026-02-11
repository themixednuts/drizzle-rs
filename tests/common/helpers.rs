// =============================================================================
// Test Failure Report Infrastructure
// =============================================================================

const BOX_WIDTH: usize = 80;
const CONTENT_WIDTH: usize = BOX_WIDTH - 4; // Account for "│ " prefix and " │" suffix

/// Captured SQL statement with optional error
#[derive(Clone, Debug)]
pub struct CapturedStatement {
    pub sql: String,
    pub error: Option<String>,
}

/// Calculate display width accounting for special characters
fn display_width(s: &str) -> usize {
    s.chars()
        .map(|c| {
            match c {
                '\t' => 4,      // Tabs count as 4 spaces
                '✓' | '✗' => 1, // These are rendered as single width in most terminals
                _ if c.is_ascii() => 1,
                _ => 2, // Assume non-ASCII chars are double-width (conservative)
            }
        })
        .sum()
}

/// Expand tabs to spaces
fn expand_tabs(s: &str) -> String {
    s.replace('\t', "    ")
}

/// Wrap text to fit within a given width
fn wrap_text(text: &str, width: usize) -> Vec<String> {
    let text = expand_tabs(text);
    let mut lines = Vec::new();

    for line in text.lines() {
        if line.is_empty() {
            lines.push(String::new());
            continue;
        }

        if display_width(line) <= width {
            lines.push(line.to_string());
        } else {
            let mut current_line = String::new();
            let mut current_width = 0;

            for word in line.split_inclusive(' ') {
                let word_width = display_width(word);

                if current_width + word_width <= width {
                    current_line.push_str(word);
                    current_width += word_width;
                } else if current_line.is_empty() {
                    // Word is longer than width, force split
                    let mut chars = word.chars().peekable();
                    while chars.peek().is_some() {
                        let mut chunk = String::new();
                        let mut chunk_width = 0;
                        while let Some(&c) = chars.peek() {
                            let c_width = if c == '\t' {
                                4
                            } else if c.is_ascii() {
                                1
                            } else {
                                2
                            };
                            if chunk_width + c_width > width {
                                break;
                            }
                            chunk.push(chars.next().unwrap());
                            chunk_width += c_width;
                        }
                        if !chunk.is_empty() {
                            lines.push(chunk);
                        }
                    }
                } else {
                    lines.push(current_line.trim_end().to_string());
                    current_line = word.to_string();
                    current_width = word_width;
                }
            }

            if !current_line.is_empty() {
                lines.push(current_line.trim_end().to_string());
            }
        }
    }

    if lines.is_empty() {
        lines.push(String::new());
    }

    lines
}

/// Format a line with proper box drawing
fn box_line(content: &str, prefix: &str) -> String {
    let content = expand_tabs(content);
    let prefix_width = display_width(prefix);
    let content_width = display_width(&content);
    let total_used = prefix_width + content_width;
    let padding = CONTENT_WIDTH.saturating_sub(total_used);
    format!("│ {}{}{} │\n", prefix, content, " ".repeat(padding))
}

/// Format a section header
fn section_header(title: &str) -> String {
    // Total inner width is BOX_WIDTH - 2 (for the ├ and ┤)
    // Format: ├─ TITLE ─────...─────┤
    // So: 1 (─) + 1 (space) + title + 1 (space) + remaining dashes = BOX_WIDTH - 2
    let inner_width = BOX_WIDTH - 2;
    let title_width = display_width(title);
    let used = 1 + 1 + title_width + 1; // "─ TITLE "
    let dashes = inner_width.saturating_sub(used);
    format!("├─ {} {}┤\n", title, "─".repeat(dashes))
}

/// Format the top border
fn top_border() -> String {
    format!("╔{}╗\n", "═".repeat(BOX_WIDTH - 2))
}

/// Format the bottom border
fn bottom_border() -> String {
    format!("╚{}╝\n", "═".repeat(BOX_WIDTH - 2))
}

/// Format an empty line within the box
fn empty_box_line() -> String {
    format!("│{}│\n", " ".repeat(BOX_WIDTH - 2))
}

/// Generate a structured failure report for any driver
pub fn failure_report(
    driver_name: &str,
    test_name: &str,
    error: &dyn std::fmt::Display,
    expected: Option<&str>,
    actual: Option<&str>,
    failed_operation: Option<&str>,
    schema_ddl: &[String],
    statements: &[CapturedStatement],
) -> String {
    let mut report = String::new();

    // Header
    let header = format!("{} TEST FAILURE REPORT", driver_name.to_uppercase());
    let header_width = display_width(&header);
    let header_padding = (BOX_WIDTH - 2 - header_width) / 2;
    let header_padding_right = BOX_WIDTH - 2 - header_width - header_padding;

    report.push('\n');
    report.push_str(&top_border());
    report.push_str(&format!(
        "║{}{}{}║\n",
        " ".repeat(header_padding),
        header,
        " ".repeat(header_padding_right)
    ));
    report.push_str(&bottom_border());
    report.push('\n');

    // Test identification section
    report.push_str(&top_border());
    report.push_str(&section_header("TEST"));
    report.push_str(&box_line(test_name, "Name:   "));
    report.push_str(&box_line(driver_name, "Driver: "));
    report.push_str(&bottom_border());
    report.push('\n');

    // Error section
    report.push_str(&top_border());
    report.push_str(&section_header("ERROR"));
    let error_text = format!("{}", error);
    let error_lines = wrap_text(&error_text, CONTENT_WIDTH);
    for line in error_lines {
        report.push_str(&box_line(&line, ""));
    }
    report.push_str(&bottom_border());
    report.push('\n');

    // Expected vs Actual (if provided)
    if expected.is_some() || actual.is_some() {
        report.push_str(&top_border());
        report.push_str(&section_header("COMPARISON"));
        if let Some(exp) = expected {
            let exp_lines = wrap_text(exp, CONTENT_WIDTH - 10);
            for (i, line) in exp_lines.iter().enumerate() {
                if i == 0 {
                    report.push_str(&box_line(line, "Expected: "));
                } else {
                    report.push_str(&box_line(line, "          "));
                }
            }
        }
        if let Some(act) = actual {
            let act_lines = wrap_text(act, CONTENT_WIDTH - 10);
            for (i, line) in act_lines.iter().enumerate() {
                if i == 0 {
                    report.push_str(&box_line(line, "Actual:   "));
                } else {
                    report.push_str(&box_line(line, "          "));
                }
            }
        }
        report.push_str(&bottom_border());
        report.push('\n');
    }

    // Failed operation section (if provided)
    if let Some(op) = failed_operation {
        report.push_str(&top_border());
        report.push_str(&section_header("FAILED OPERATION"));
        let op_lines = wrap_text(op, CONTENT_WIDTH - 2);
        for line in op_lines {
            report.push_str(&box_line(&line, "  "));
        }
        report.push_str(&bottom_border());
        report.push('\n');
    }

    // Schema DDL section
    report.push_str(&top_border());
    report.push_str(&section_header("SCHEMA DDL"));
    if schema_ddl.is_empty() {
        report.push_str(&box_line("(no DDL statements captured)", ""));
    } else {
        for (i, ddl) in schema_ddl.iter().enumerate() {
            report.push_str(&box_line(&format!("[{}]", i + 1), ""));
            for line in ddl.lines() {
                let expanded = expand_tabs(line);
                let wrapped = wrap_text(&expanded, CONTENT_WIDTH - 2);
                for wrap_line in wrapped {
                    report.push_str(&box_line(&wrap_line, "  "));
                }
            }
            if i < schema_ddl.len() - 1 {
                report.push_str(&empty_box_line());
            }
        }
    }
    report.push_str(&bottom_border());
    report.push('\n');

    // Executed statements section
    report.push_str(&top_border());
    report.push_str(&section_header("EXECUTED STATEMENTS"));
    if statements.is_empty() {
        report.push_str(&box_line("(no statements executed)", ""));
    } else {
        for (i, stmt) in statements.iter().enumerate() {
            let status = if stmt.error.is_some() { "✗" } else { "✓" };
            report.push_str(&box_line(&format!("[{}]", i + 1), &format!("{} ", status)));
            for line in stmt.sql.lines() {
                let expanded = expand_tabs(line);
                let wrapped = wrap_text(&expanded, CONTENT_WIDTH - 4);
                for wrap_line in wrapped {
                    report.push_str(&box_line(&wrap_line, "    "));
                }
            }
            if i < statements.len() - 1 {
                report.push_str(&empty_box_line());
            }
        }
    }
    report.push_str(&bottom_border());
    report.push('\n');

    report
}

/// Test database wrapper that captures execution context for failure reports
pub mod test_db {
    use super::{CapturedStatement, failure_report};
    use std::cell::RefCell;
    use std::ops::{Deref, DerefMut};

    /// Generic test database wrapper
    pub struct TestDb<D> {
        pub db: D,
        pub driver_name: String,
        pub schema_ddl: Vec<String>,
        pub statements: RefCell<Vec<CapturedStatement>>,
    }

    impl<D> Deref for TestDb<D> {
        type Target = D;
        fn deref(&self) -> &Self::Target {
            &self.db
        }
    }

    impl<D> DerefMut for TestDb<D> {
        fn deref_mut(&mut self) -> &mut Self::Target {
            &mut self.db
        }
    }

    impl<D> TestDb<D> {
        pub fn new(db: D, driver_name: impl Into<String>, schema_ddl: Vec<String>) -> Self {
            let ddl_statements: Vec<CapturedStatement> = schema_ddl
                .iter()
                .map(|sql| CapturedStatement {
                    sql: sql.clone(),
                    error: None,
                })
                .collect();

            Self {
                db,
                driver_name: driver_name.into(),
                schema_ddl,
                statements: RefCell::new(ddl_statements),
            }
        }

        /// Record a SQL statement execution
        pub fn record(&self, sql: impl Into<String>, error: Option<String>) {
            self.statements.borrow_mut().push(CapturedStatement {
                sql: sql.into(),
                error,
            });
        }

        /// Generate a failure report
        pub fn report(
            &self,
            test_name: &str,
            error: &dyn std::fmt::Display,
            expected: Option<&str>,
            actual: Option<&str>,
            failed_operation: Option<&str>,
        ) -> String {
            failure_report(
                &self.driver_name,
                test_name,
                error,
                expected,
                actual,
                failed_operation,
                &self.schema_ddl,
                &self.statements.borrow(),
            )
        }

        /// Panic with a formatted failure report
        pub fn fail(
            &self,
            test_name: &str,
            error: &dyn std::fmt::Display,
            expected: Option<&str>,
            actual: Option<&str>,
        ) -> ! {
            panic!("{}", self.report(test_name, error, expected, actual, None));
        }

        /// Panic with a formatted failure report including the failed operation
        pub fn fail_with_op(
            &self,
            test_name: &str,
            error: &dyn std::fmt::Display,
            failed_operation: &str,
        ) -> ! {
            panic!(
                "{}",
                self.report(test_name, error, None, None, Some(failed_operation))
            );
        }
    }
}

// =============================================================================
// Driver-specific setup modules
// =============================================================================

#[cfg(feature = "rusqlite")]
pub mod rusqlite_setup {
    use super::test_db::TestDb;
    use drizzle::sqlite::rusqlite::Drizzle;
    use rusqlite::Connection;

    pub fn setup_db<S: Default + drizzle::core::SQLSchemaImpl>() -> (TestDb<Drizzle<S>>, S) {
        let conn = Connection::open_in_memory().expect("Failed to create in-memory database");
        let schema = S::default();
        let schema_ddl: Vec<_> = schema
            .create_statements()
            .expect("create statements")
            .collect();
        let (db, schema) = Drizzle::new(conn, schema);

        if let Err(e) = db.create() {
            let test_db = TestDb::new(db, "rusqlite", schema_ddl);
            test_db.fail(
                "schema_creation",
                &e,
                Some("Schema created successfully"),
                None,
            );
        }

        let test_db = TestDb::new(db, "rusqlite", schema_ddl);
        (test_db, schema)
    }
}

#[cfg(feature = "libsql")]
pub mod libsql_setup {
    use super::test_db::TestDb;
    use drizzle::sqlite::libsql::Drizzle;
    use libsql::Builder;

    pub async fn setup_db<S: Default + drizzle::core::SQLSchemaImpl>() -> (TestDb<Drizzle<S>>, S) {
        let db = Builder::new_local(":memory:")
            .build()
            .await
            .expect("build db");
        let conn = db.connect().expect("connect to db");
        let schema = S::default();
        let schema_ddl: Vec<_> = schema
            .create_statements()
            .expect("create statements")
            .collect();
        let (db, schema) = Drizzle::new(conn, schema);

        if let Err(e) = db.create().await {
            let test_db = TestDb::new(db, "libsql", schema_ddl);
            test_db.fail(
                "schema_creation",
                &e,
                Some("Schema created successfully"),
                None,
            );
        }

        let test_db = TestDb::new(db, "libsql", schema_ddl);
        (test_db, schema)
    }
}

#[cfg(feature = "turso")]
pub mod turso_setup {
    use super::test_db::TestDb;
    use drizzle::sqlite::turso::Drizzle;
    use turso::Builder;

    pub async fn setup_db<S: Default + drizzle::core::SQLSchemaImpl>() -> (TestDb<Drizzle<S>>, S) {
        let db = Builder::new_local(":memory:")
            .build()
            .await
            .expect("build db");
        let conn = db.connect().expect("connect to db");
        let schema = S::default();
        let schema_ddl: Vec<_> = schema
            .create_statements()
            .expect("create statements")
            .collect();
        let (db, schema) = Drizzle::new(conn, schema);

        if let Err(e) = db.create().await {
            let test_db = TestDb::new(db, "turso", schema_ddl);
            test_db.fail(
                "schema_creation",
                &e,
                Some("Schema created successfully"),
                None,
            );
        }

        let test_db = TestDb::new(db, "turso", schema_ddl);
        (test_db, schema)
    }
}

#[cfg(feature = "postgres-sync")]
pub mod postgres_sync_setup {
    use super::{CapturedStatement, failure_report};
    use drizzle::postgres::sync::Drizzle;
    use postgres::{Client, NoTls};
    use std::cell::RefCell;
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
                println!("PostgreSQL already running");
                return;
            }

            println!("Starting PostgreSQL via Docker Compose...");

            // Start docker compose
            let status = Command::new("docker")
                .args(["compose", "up", "-d", "postgres"])
                .status();

            match status {
                Ok(s) if s.success() => {
                    // Wait for PostgreSQL to be ready
                    println!("Waiting for PostgreSQL to be ready...");
                    for i in 0..30 {
                        thread::sleep(Duration::from_secs(1));
                        if Client::connect(&database_url, NoTls).is_ok() {
                            println!("PostgreSQL is ready! (took {}s)", i + 1);
                            return;
                        }
                    }
                    panic!("PostgreSQL failed to start within 30 seconds");
                }
                Ok(_) => {
                    eprintln!("Docker Compose failed. Make sure Docker is running.");
                    eprintln!("You can manually start with: docker compose up -d postgres");
                }
                Err(e) => {
                    eprintln!("Could not run docker compose: {}", e);
                    eprintln!("Make sure Docker is installed and running.");
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
    pub struct TestDb<S> {
        pub db: Drizzle<S>,
        schema_name: String,
        schema_ddl: Vec<String>,
        statements: RefCell<Vec<CapturedStatement>>,
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

    impl<S> TestDb<S> {
        pub fn record(&self, sql: impl Into<String>, error: Option<String>) {
            self.statements.borrow_mut().push(CapturedStatement {
                sql: sql.into(),
                error,
            });
        }

        pub fn report(
            &self,
            test_name: &str,
            error: &dyn std::fmt::Display,
            expected: Option<&str>,
            actual: Option<&str>,
            failed_operation: Option<&str>,
        ) -> String {
            failure_report(
                "postgres-sync",
                test_name,
                error,
                expected,
                actual,
                failed_operation,
                &self.schema_ddl,
                &self.statements.borrow(),
            )
        }

        pub fn fail(
            &self,
            test_name: &str,
            error: &dyn std::fmt::Display,
            expected: Option<&str>,
            actual: Option<&str>,
        ) -> ! {
            panic!("{}", self.report(test_name, error, expected, actual, None));
        }

        pub fn fail_with_op(
            &self,
            test_name: &str,
            error: &dyn std::fmt::Display,
            failed_operation: &str,
        ) -> ! {
            panic!(
                "{}",
                self.report(test_name, error, None, None, Some(failed_operation))
            );
        }
    }

    impl<S> Drop for TestDb<S> {
        fn drop(&mut self) {
            // Open a new connection to drop the schema (original is owned by Drizzle)
            if let Ok(mut client) = Client::connect(&get_database_url(), NoTls) {
                let drop_sql = format!("DROP SCHEMA IF EXISTS \"{}\" CASCADE", self.schema_name);
                if let Err(e) = client.batch_execute(&drop_sql) {
                    eprintln!("Failed to drop test schema {}: {}", self.schema_name, e);
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
        let schema_ddl: Vec<_> = schema
            .create_statements()
            .expect("create statements")
            .collect();
        let (mut db, schema) = Drizzle::new(client, schema);

        let ddl_statements: Vec<CapturedStatement> = schema_ddl
            .iter()
            .map(|sql| CapturedStatement {
                sql: sql.clone(),
                error: None,
            })
            .collect();

        if let Err(e) = db.create() {
            let test_db = TestDb {
                db,
                schema_name,
                schema_ddl,
                statements: RefCell::new(ddl_statements),
            };
            test_db.fail(
                "schema_creation",
                &e,
                Some("Schema created successfully"),
                None,
            );
        }

        let test_db = TestDb {
            db,
            schema_name,
            schema_ddl,
            statements: RefCell::new(ddl_statements),
        };
        (test_db, schema)
    }
}

#[cfg(feature = "tokio-postgres")]
pub mod tokio_postgres_setup {
    use super::{CapturedStatement, failure_report};
    use drizzle::postgres::tokio::Drizzle;
    use std::cell::RefCell;
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
                println!("PostgreSQL already running");
                return;
            }

            println!("Starting PostgreSQL via Docker Compose...");

            let status = Command::new("docker")
                .args(["compose", "up", "-d", "postgres"])
                .status();

            match status {
                Ok(s) if s.success() => {
                    println!("Waiting for PostgreSQL to be ready...");
                    for i in 0..30 {
                        thread::sleep(Duration::from_secs(1));
                        if check_postgres_available(&database_url) {
                            println!("PostgreSQL is ready! (took {}s)", i + 1);
                            return;
                        }
                    }
                    panic!("PostgreSQL failed to start within 30 seconds");
                }
                Ok(_) => {
                    eprintln!("Docker Compose failed. Make sure Docker is running.");
                    eprintln!("You can manually start with: docker compose up -d postgres");
                }
                Err(e) => {
                    eprintln!("Could not run docker compose: {}", e);
                    eprintln!("Make sure Docker is installed and running.");
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
        schema_ddl: Vec<String>,
        statements: RefCell<Vec<CapturedStatement>>,
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

    impl<S> TestDb<S> {
        pub fn record(&self, sql: impl Into<String>, error: Option<String>) {
            self.statements.borrow_mut().push(CapturedStatement {
                sql: sql.into(),
                error,
            });
        }

        pub fn report(
            &self,
            test_name: &str,
            error: &dyn std::fmt::Display,
            expected: Option<&str>,
            actual: Option<&str>,
            failed_operation: Option<&str>,
        ) -> String {
            failure_report(
                "tokio-postgres",
                test_name,
                error,
                expected,
                actual,
                failed_operation,
                &self.schema_ddl,
                &self.statements.borrow(),
            )
        }

        pub fn fail(
            &self,
            test_name: &str,
            error: &dyn std::fmt::Display,
            expected: Option<&str>,
            actual: Option<&str>,
        ) -> ! {
            panic!("{}", self.report(test_name, error, expected, actual, None));
        }

        pub fn fail_with_op(
            &self,
            test_name: &str,
            error: &dyn std::fmt::Display,
            failed_operation: &str,
        ) -> ! {
            panic!(
                "{}",
                self.report(test_name, error, None, None, Some(failed_operation))
            );
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
                            eprintln!("Failed to drop test schema {}: {}", schema_name, e);
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
        let schema_ddl: Vec<_> = schema
            .create_statements()
            .expect("create statements")
            .collect();
        let (mut db, schema) = Drizzle::new(client, schema);

        let ddl_statements: Vec<CapturedStatement> = schema_ddl
            .iter()
            .map(|sql| CapturedStatement {
                sql: sql.clone(),
                error: None,
            })
            .collect();

        if let Err(e) = db.create().await {
            let test_db = TestDb {
                db,
                schema_name,
                schema_ddl,
                statements: RefCell::new(ddl_statements),
            };
            test_db.fail(
                "schema_creation",
                &e,
                Some("Schema created successfully"),
                None,
            );
        }

        let test_db = TestDb {
            db,
            schema_name,
            schema_ddl,
            statements: RefCell::new(ddl_statements),
        };
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

#[cfg(test)]
mod tests {
    use drizzle_rs::prelude::*;
    use drizzle_rs::{SQLiteTable, drizzle};
    use rusqlite::Connection as RusqliteConnection;
    use std::borrow::Cow;

    // Define test tables for schema
    #[SQLiteTable(name = "users")]
    struct Users {
        #[integer(primary)]
        id: i64,
        #[text]
        name: String,
        #[text]
        email: String,
    }

    #[SQLiteTable(name = "posts")]
    struct Posts {
        #[integer(primary)]
        id: i64,
        #[integer(references = Users::id)]
        user_id: i64,
        #[text]
        title: String,
        #[text]
        content: String,
    }

    #[test]
    fn test_drizzle_macro_without_schema() {
        // Create an in-memory rusqlite connection
        let conn = RusqliteConnection::open_in_memory().unwrap();

        // Create a table
        conn.execute(
            "CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT, email TEXT)",
            [],
        )
        .unwrap();

        // Use the drizzle! macro without schema (original form)
        let drizzle = drizzle!(conn);

        // Test execute using the Drizzle instance
        let result = drizzle.execute(
            "INSERT INTO users (name, email) VALUES (?, ?)",
            &[
                SQLiteValue::Text(Cow::Owned("John".to_string())),
                SQLiteValue::Text(Cow::Owned("john@example.com".to_string())),
            ],
        );
        assert!(result.is_ok());

        // Test that the data was inserted by querying with the original connection
        let result = conn.query_row(
            "SELECT COUNT(*) FROM users WHERE name = 'John'",
            [],
            |row| row.get::<_, i64>(0),
        );
        assert_eq!(result.unwrap(), 1);
    }

    #[test]
    fn test_drizzle_macro_with_schema() {
        // Create an in-memory rusqlite connection
        let conn = RusqliteConnection::open_in_memory().unwrap();

        // Create tables
        conn.execute(
            "CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT, email TEXT)",
            [],
        )
        .unwrap();

        conn.execute(
            "CREATE TABLE posts (id INTEGER PRIMARY KEY, user_id INTEGER, title TEXT, content TEXT)",
            [],
        ).unwrap();

        // Insert some test data
        conn.execute(
            "INSERT INTO users (id, name, email) VALUES (1, 'John', 'john@example.com')",
            [],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO posts (id, user_id, title, content) VALUES (1, 1, 'First Post', 'Hello World')",
            [],
        ).unwrap();

        // Use the drizzle! macro with schema
        let db = drizzle!(conn, [Users, Posts]);

        // Test query building with SELECT
        let users = db
            .from::<Users>()
            .select(columns!(Users::id, Users::name, Users::email))
            .all()
            .unwrap();
        assert_eq!(users.len(), 1);

        // Test query building with WHERE clause
        let user = db
            .from::<Users>()
            .select(columns!(Users::id, Users::name, Users::email))
            .r#where(eq(Users::id, 1))
            .first()
            .unwrap();
        assert_eq!(user.name, "John");
        assert_eq!(user.email, "john@example.com");

        // Test query building with JOIN
        let joined_results = db
            .from::<Users>()
            .join::<Posts>(eq(Users::id, Posts::user_id))
            .select(columns!(Users::id, Users::name, Posts::title))
            .where_(eq(Posts::user_id, 1))
            .all()
            .unwrap();

        assert_eq!(joined_results.len(), 1);

        // Test INSERT with query building
        let new_user_id = db
            .insert_into::<Users>()
            .values([(Users::name, "Jane"), (Users::email, "jane@example.com")])
            .returning([Users::id])
            .first()
            .unwrap()
            .id;

        // Verify insertion worked
        let jane = db
            .from::<Users>()
            .select(columns!(Users::id, Users::name, Users::email))
            .r#where(eq(Users::name, "Jane"))
            .first()
            .unwrap();
        assert_eq!(jane.id, new_user_id);
        assert_eq!(jane.email, "jane@example.com");

        // Test UPDATE
        db.update::<Users>()
            .set(Users::email, "jane.doe@example.com")
            .r#where(eq(Users::id, new_user_id))
            .run()
            .unwrap();

        // Verify update worked
        let updated_jane = db
            .from::<Users>()
            .select(columns!(Users::id, Users::name, Users::email))
            .r#where(eq(Users::id, new_user_id))
            .first()
            .unwrap();
        assert_eq!(updated_jane.email, "jane.doe@example.com");

        // Test DELETE
        db.delete_from::<Users>()
            .r#where(eq(Users::id, new_user_id))
            .run()
            .unwrap();

        // Verify deletion worked
        let jane_count = db
            .from::<Users>()
            .select(columns!(Users::id, Users::name, Users::email))
            .r#where(eq(Users::id, new_user_id))
            .all()
            .unwrap();
        assert_eq!(jane_count.len(), 0);
    }

    #[test]
    fn test_drizzle_prepare() {
        // Create an in-memory rusqlite connection
        let conn = RusqliteConnection::open_in_memory().unwrap();

        // Create a table
        conn.execute(
            "CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT, email TEXT)",
            [],
        )
        .unwrap();

        // Use the drizzle! macro without schema (original form)
        let drizzle = drizzle!(conn);

        // Test prepare
        let result = drizzle.prepare("INSERT INTO users (name, email) VALUES (?, ?)");
        assert!(result.is_ok());

        // Use the prepared statement
        let mut stmt = result.unwrap();
        let run_result = stmt.run(&[
            SQLiteValue::Text(Cow::Owned("John".to_string())),
            SQLiteValue::Text(Cow::Owned("john@example.com".to_string())),
        ]);
        assert!(run_result.is_ok());

        // Verify the data was inserted
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM users WHERE name = 'John'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn test_drizzle_transaction() {
        // Create an in-memory rusqlite connection
        let conn = RusqliteConnection::open_in_memory().unwrap();

        // Create a table
        conn.execute(
            "CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT, email TEXT)",
            [],
        )
        .unwrap();

        // Use the drizzle! macro without schema (original form)
        let mut drizzle = drizzle!(conn);

        // Test transaction with commit
        let result = drizzle.transaction::<_, _, DriverError>(|tx| {
            // Execute a query within the transaction
            tx.run_statement(
                "INSERT INTO users (name, email) VALUES (?, ?)",
                &[
                    SQLiteValue::Text(Cow::Owned("Transaction User".to_string())),
                    SQLiteValue::Text(Cow::Owned("tx@example.com".to_string())),
                ],
            )?;

            // Return success to commit the transaction
            Ok(42) // Just a test value to verify the result
        });

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);

        // Verify the transaction was committed
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM users WHERE name = 'Transaction User'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn test_drizzle_transaction_rollback() {
        // Create an in-memory rusqlite connection
        let conn = RusqliteConnection::open_in_memory().unwrap();

        // Create a table
        conn.execute(
            "CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT, email TEXT)",
            [],
        )
        .unwrap();

        // Use the drizzle! macro without schema (original form)
        let mut drizzle = drizzle!(conn);

        // Test transaction with rollback
        let result = drizzle.transaction::<_, _, DriverError>(|tx| {
            // Execute a query within the transaction
            tx.run_statement(
                "INSERT INTO users (name, email) VALUES (?, ?)",
                &[
                    SQLiteValue::Text(Cow::Owned("Rollback User".to_string())),
                    SQLiteValue::Text(Cow::Owned("rollback@example.com".to_string())),
                ],
            )?;

            // Return an error to rollback the transaction
            Err(DriverError::Query("Simulated error".to_string()))
        });

        assert!(result.is_err());

        // Verify the transaction was rolled back
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM users WHERE name = 'Rollback User'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 0); // Should be 0 since the transaction was rolled back
    }
}

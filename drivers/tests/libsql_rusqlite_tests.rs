#![cfg(feature = "libsql-rusqlite")]

use drivers::libsql_rusqlite::LibsqlRusqliteConnection;
use drivers::{Connection, DbRow, DriverError, PreparedStatement, SQLiteValue, Transaction};
use libsql_rusqlite::Connection as NativeLibsqlRusqliteConnection;
use std::borrow::Cow;

fn create_in_memory_db() -> LibsqlRusqliteConnection<'static> {
    let conn = NativeLibsqlRusqliteConnection::open_in_memory()
        .expect("Failed to open in-memory database");
    LibsqlRusqliteConnection::new(conn)
}

#[test]
fn test_run_statement_create_table() -> Result<(), DriverError> {
    let db = create_in_memory_db();
    let result = db.run_statement(
        "CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT NOT NULL)",
        &[],
    )?;
    assert_eq!(result, 0); // CREATE TABLE usually reports 0 changes
    Ok(())
}

#[test]
fn test_run_statement_insert() -> Result<(), DriverError> {
    let db = create_in_memory_db();
    db.run_statement(
        "CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT NOT NULL)",
        &[],
    )?;
    let result = db.run_statement(
        "INSERT INTO users (name) VALUES (?)",
        &[SQLiteValue::Text(Cow::Borrowed("Alice"))],
    )?;
    assert_eq!(result, 1); // Should report 1 change
    Ok(())
}

#[test]
fn test_query_statement_select() -> Result<(), DriverError> {
    let db = create_in_memory_db();
    db.run_statement(
        "CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT NOT NULL, age INTEGER)",
        &[],
    )?;
    db.run_statement(
        "INSERT INTO users (name, age) VALUES (?, ?)",
        &[
            SQLiteValue::Text(Cow::Borrowed("Alice")),
            SQLiteValue::Integer(30),
        ],
    )?;
    db.run_statement(
        "INSERT INTO users (name, age) VALUES (?, ?)",
        &[
            SQLiteValue::Text(Cow::Borrowed("Bob")),
            SQLiteValue::Null, // Test NULL value
        ],
    )?;

    let rows = db.query_statement("SELECT id, name, age FROM users ORDER BY id", &[])?;

    assert_eq!(rows.len(), 2);

    // Check first row (Alice)
    assert_eq!(rows[0].get(0)?, SQLiteValue::Integer(1));
    assert_eq!(rows[0].get(1)?, SQLiteValue::Text(Cow::Borrowed("Alice")));
    assert_eq!(rows[0].get(2)?, SQLiteValue::Integer(30));

    // Check second row (Bob)
    assert_eq!(rows[1].get(0)?, SQLiteValue::Integer(2));
    assert_eq!(rows[1].get(1)?, SQLiteValue::Text(Cow::Borrowed("Bob")));
    assert_eq!(rows[1].get(2)?, SQLiteValue::Null);

    Ok(())
}

#[test]
fn test_prepared_statement_insert_run() -> Result<(), DriverError> {
    let db = create_in_memory_db();
    db.run_statement(
        "CREATE TABLE items (id INTEGER PRIMARY KEY, description TEXT)",
        &[],
    )?;

    let sql = "INSERT INTO items (description) VALUES (?)";
    let mut stmt = db.prepare(sql)?;

    let changes1 = stmt.run(&[SQLiteValue::Text(Cow::Borrowed("Item 1"))])?;
    assert_eq!(changes1, 1);

    let changes2 = stmt.run(&[SQLiteValue::Text(Cow::Borrowed("Item 2"))])?;
    assert_eq!(changes2, 1);

    // Verify insertions
    let rows = db.query_statement("SELECT description FROM items ORDER BY id", &[])?;
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].get(0)?, SQLiteValue::Text(Cow::Borrowed("Item 1")));
    assert_eq!(rows[1].get(0)?, SQLiteValue::Text(Cow::Borrowed("Item 2")));

    Ok(())
}

#[test]
fn test_prepared_statement_select_query() -> Result<(), DriverError> {
    let db = create_in_memory_db();
    db.run_statement(
        "CREATE TABLE items (id INTEGER PRIMARY KEY, description TEXT, price REAL)",
        &[],
    )?;
    db.run_statement(
        "INSERT INTO items (description, price) VALUES (?, ?)",
        &[
            SQLiteValue::Text(Cow::Borrowed("Gadget")),
            SQLiteValue::Real(19.99),
        ],
    )?;
    db.run_statement(
        "INSERT INTO items (description, price) VALUES (?, ?)",
        &[
            SQLiteValue::Text(Cow::Borrowed("Widget")),
            SQLiteValue::Real(25.50),
        ],
    )?;

    let sql = "SELECT description, price FROM items WHERE price > ? ORDER BY price";
    let mut stmt = db.prepare(sql)?;

    let rows = stmt.query(&[SQLiteValue::Real(20.0)])?;

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].get(0)?, SQLiteValue::Text(Cow::Borrowed("Widget")));
    assert_eq!(rows[0].get(1)?, SQLiteValue::Real(25.50));

    Ok(())
}

#[test]
fn test_transaction_commit() -> Result<(), DriverError> {
    let mut db = create_in_memory_db();
    db.run_statement(
        "CREATE TABLE accounts (id INTEGER PRIMARY KEY, balance REAL NOT NULL)",
        &[],
    )?;
    db.run_statement(
        "INSERT INTO accounts (id, balance) VALUES (?, ?)",
        &[SQLiteValue::Integer(1), SQLiteValue::Real(100.0)],
    )?;
    db.run_statement(
        "INSERT INTO accounts (id, balance) VALUES (?, ?)",
        &[SQLiteValue::Integer(2), SQLiteValue::Real(50.0)],
    )?;

    // Start transaction
    let mut tx = db.begin_transaction()?;

    // Perform operations within transaction
    tx.run_statement(
        "UPDATE accounts SET balance = balance - ? WHERE id = ?",
        &[SQLiteValue::Real(25.0), SQLiteValue::Integer(1)],
    )?;
    tx.run_statement(
        "UPDATE accounts SET balance = balance + ? WHERE id = ?",
        &[SQLiteValue::Real(25.0), SQLiteValue::Integer(2)],
    )?;

    // Commit transaction
    tx.commit()?;

    // Verify results outside transaction
    let rows = db.query_statement("SELECT id, balance FROM accounts ORDER BY id", &[])?;
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].get(0)?, SQLiteValue::Integer(1));
    assert_eq!(rows[0].get(1)?, SQLiteValue::Real(75.0)); // 100 - 25
    assert_eq!(rows[1].get(0)?, SQLiteValue::Integer(2));
    assert_eq!(rows[1].get(1)?, SQLiteValue::Real(75.0)); // 50 + 25

    Ok(())
}

#[test]
fn test_transaction_rollback() -> Result<(), DriverError> {
    let mut db = create_in_memory_db();
    db.run_statement(
        "CREATE TABLE accounts (id INTEGER PRIMARY KEY, balance REAL NOT NULL)",
        &[],
    )?;
    db.run_statement(
        "INSERT INTO accounts (id, balance) VALUES (?, ?)",
        &[SQLiteValue::Integer(1), SQLiteValue::Real(100.0)],
    )?;

    // Start transaction
    let mut tx = db.begin_transaction()?;

    // Perform operation
    tx.run_statement(
        "UPDATE accounts SET balance = balance - ? WHERE id = ?",
        &[SQLiteValue::Real(25.0), SQLiteValue::Integer(1)],
    )?;

    // Check balance inside transaction (optional)
    let temp_rows = tx.query_statement("SELECT balance FROM accounts WHERE id = 1", &[])?;
    assert_eq!(temp_rows[0].get(0)?, SQLiteValue::Real(75.0));

    // Rollback transaction
    tx.rollback()?;

    // Verify results outside transaction (should be unchanged)
    let rows = db.query_statement("SELECT id, balance FROM accounts ORDER BY id", &[])?;
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].get(0)?, SQLiteValue::Integer(1));
    assert_eq!(rows[0].get(1)?, SQLiteValue::Real(100.0)); // Original balance

    Ok(())
}

#[test]
fn test_data_types_real_blob() -> Result<(), DriverError> {
    let db = create_in_memory_db();
    db.run_statement(
        "CREATE TABLE data (id INTEGER PRIMARY KEY, measurement REAL, image BLOB)",
        &[],
    )?;

    let measurement = 3.14159;
    let image_data: Vec<u8> = vec![0, 1, 2, 3, 4, 5];

    db.run_statement(
        "INSERT INTO data (measurement, image) VALUES (?, ?)",
        &[
            SQLiteValue::Real(measurement),
            SQLiteValue::Blob(Cow::Owned(image_data.clone())),
        ],
    )?;

    let rows = db.query_statement("SELECT measurement, image FROM data WHERE id = 1", &[])?;
    assert_eq!(rows.len(), 1);

    assert_eq!(rows[0].get(0)?, SQLiteValue::Real(measurement));
    match &rows[0].get(1)? {
        SQLiteValue::Blob(blob_data) => {
            // Compare the contents of the blobs, not the Cow wrappers
            assert_eq!(&**blob_data, &image_data);
        }
        _ => panic!("Expected BLOB data"),
    }

    Ok(())
}

use drizzle_rs::prelude::*;
use rusqlite::Connection;

#[SQLiteTable(name = "test_table")]
struct TestTable {
    #[integer(primary)]
    id: i32,
    #[text]
    name: String,
    #[text]
    email: Option<String>,
}

#[SQLiteTable(name = "strict_table", strict)]
struct StrictTable {
    #[integer(primary)]
    id: i32,
    #[text]
    content: String,
}

#[test]
fn table_sql() {
    let conn = Connection::open_in_memory().unwrap();

    let sql = TestTable::SQL;
    assert!(sql.contains("CREATE TABLE"));
    assert!(sql.contains("test_table"));
    assert!(sql.contains("PRIMARY KEY"));

    conn.execute(sql, []).unwrap();
}

#[test]
fn strict_table() {
    let conn = Connection::open_in_memory().unwrap();

    let sql = StrictTable::SQL;
    assert!(sql.contains("STRICT"));
    assert!(sql.contains("strict_table"));

    conn.execute(sql, []).unwrap();
}

#[test]
fn name_attribute() {
    let sql = TestTable::SQL;
    assert!(sql.contains("test_table"));
    assert!(!sql.contains("TestTable"));
}

#[test]
fn column_types() {
    let sql = TestTable::SQL;
    assert!(sql.contains("INTEGER"));
    assert!(sql.contains("TEXT"));
}

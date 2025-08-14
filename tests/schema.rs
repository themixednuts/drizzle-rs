use drizzle_rs::prelude::*;

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
    let sql = TestTable::SQL.sql();
    assert!(sql.contains("CREATE TABLE"));
    assert!(sql.contains("test_table"));
    assert!(sql.contains("PRIMARY KEY"));
}

#[test]
fn strict_table() {
    let sql = StrictTable::SQL.sql();
    assert!(sql.contains("STRICT"));
    assert!(sql.contains("strict_table"));
}

#[test]
fn name_attribute() {
    let sql = TestTable::SQL.sql();
    assert!(sql.contains("test_table"));
    assert!(!sql.contains("TestTable"));
}

#[test]
fn column_types() {
    let sql = TestTable::SQL.sql();
    assert!(sql.contains("INTEGER"));
    assert!(sql.contains("TEXT"));
}

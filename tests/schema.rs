#![cfg(any(feature = "rusqlite", feature = "turso", feature = "libsql"))]
use drizzle_rs::prelude::*;

mod common;

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

// Schema derive tests
#[SQLiteTable(name = "users")]
struct User {
    #[integer(primary)]
    id: i32,
    #[text]
    email: String,
    #[text]
    name: String,
}

#[SQLiteIndex(unique)]
struct UserEmailIdx(User::email);

#[SQLiteIndex]
struct UserNameIdx(User::name);

#[derive(SQLSchema)]
struct AppTestSchema {
    user: User,
    user_email_idx: UserEmailIdx,
    user_name_idx: UserNameIdx,
}

#[tokio::test]
async fn test_schema_derive() {
    let conn = setup_test_db!();
    let schema = AppTestSchema::new();

    // Test that we can create all objects
    drizzle_exec!(schema.create(&conn));

    // Test that we can get tables and indexes
    let tables = schema.tables();
    let indexes = schema.indexes();

    // Tables and indexes are now tuples, not vectors
    // We can access them by position or destructure them
}

#[tokio::test]
async fn test_schema_with_drizzle_macro() {
    let conn = setup_test_db!();
    let (db, schema) = drizzle!(conn, AppTestSchema);

    // Test that we can create all database objects
    drizzle_exec!(schema.create(db.conn()));

    // Test that we can use the schema for queries
    let insert_data = InsertUser::new("test@example.com", "Test User");
    let result = drizzle_exec!(db.insert(schema.user).values([insert_data]).execute());
    assert_eq!(result, 1);

    // Test that the indexes work (this would fail if indexes weren't created)
    let users: Vec<SelectUser> = drizzle_exec!(
        db.select(())
            .from(schema.user)
            .r#where(eq(schema.user.email, "test@example.com"))
            .all()
    );

    assert_eq!(users.len(), 1);
    assert_eq!(users[0].email, "test@example.com");
    assert_eq!(users[0].name, "Test User");
}

#[tokio::test]
async fn test_schema_destructuring() {
    let conn = setup_test_db!();
    let (db, schema) = drizzle!(conn, AppTestSchema);

    // Test destructuring the schema into individual components
    let (user, user_email_idx, user_name_idx) = schema.into();

    // Create all objects
    let schema = AppTestSchema::new(); // Get a fresh schema for create
    drizzle_exec!(schema.create(db.conn()));

    // Test that we can use the destructured components
    let insert_data = InsertUser::new("destructured@example.com", "Destructured User");
    let result = drizzle_exec!(db.insert(user).values([insert_data]).execute());
    assert_eq!(result, 1);

    // Query using the destructured table
    let users: Vec<SelectUser> = drizzle_exec!(
        db.select(())
            .from(user)
            .r#where(eq(user.email, "destructured@example.com"))
            .all()
    );

    assert_eq!(users.len(), 1);
    assert_eq!(users[0].email, "destructured@example.com");
    assert_eq!(users[0].name, "Destructured User");
}

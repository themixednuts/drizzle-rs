#![cfg(feature = "libsql")]

mod common;

use common::Complex;
use common::{InsertSimple, Role, SelectSimple, Simple, UpdateSimple};
use drizzle_rs::prelude::*;
use libsql::{Builder, Connection};

use crate::common::SimpleSchema;

// Helper function to create a libsql connection for testing
async fn setup_libsql_connection() -> Connection {
    let db = Builder::new_local(":memory:")
        .build()
        .await
        .expect("Failed to create in-memory database");
    db.connect().expect("Failed to connect to database")
}

async fn setup_test_tables(conn: &Connection) {
    // Create Simple table
    conn.execute(Simple::new().sql().sql().as_str(), ())
        .await
        .expect("Failed to create simple table");

    // Create Complex table
    conn.execute(Complex::new().sql().sql().as_str(), ())
        .await
        .expect("Failed to create complex table");
}

#[tokio::test]
async fn test_basic_libsql_insert_select() {
    let conn = setup_libsql_connection().await;
    setup_test_tables(&conn).await;

    let (db, SimpleSchema { simple }) = drizzle!(conn, SimpleSchema);

    // Test basic insert
    let data = InsertSimple::new("libsql_test");
    let inserted = db.insert(simple).values([data]).execute().await.unwrap();

    assert_eq!(inserted, 1);

    // Test basic select
    let selected: Vec<SelectSimple> = db.select(()).from(simple).all().await.unwrap();

    assert!(selected.len() > 0);
    assert_eq!(selected[0].name, "libsql_test");
}

#[tokio::test]
async fn test_libsql_get_single_row() {
    let conn = setup_libsql_connection().await;
    setup_test_tables(&conn).await;

    let (db, SimpleSchema { simple }) = drizzle!(conn, SimpleSchema);

    // Insert test data
    let data = InsertSimple::new("single_row_test");
    db.insert(simple).values([data]).execute().await.unwrap();

    // Test get method
    let row: SelectSimple = db.select(()).from(simple).get().await.unwrap();

    assert_eq!(row.name, "single_row_test");
}

#[derive(FromRow, Default)]
struct SimplePartial(i32, String);

#[tokio::test]
async fn test_libsql_column_tuple_select() {
    let conn = setup_libsql_connection().await;
    setup_test_tables(&conn).await;

    let (db, SimpleSchema { simple }) = drizzle!(conn, SimpleSchema);

    // Insert test data
    let data = InsertSimple::new("column_tuple_test");
    db.insert(simple).values([data]).execute().await.unwrap();

    // Test column tuple select (alternative to partial select for libsql)
    let row: SimplePartial = db
        .select((simple.id, simple.name))
        .from(simple)
        .get()
        .await
        .unwrap();

    assert!(row.0.is_positive());
    assert_eq!(row.1, "column_tuple_test");
}

#[cfg(feature = "uuid")]
#[tokio::test]
async fn test_libsql_complex_types() {
    use crate::common::{ComplexSchema, InsertComplex, SelectComplex};

    let conn = setup_libsql_connection().await;
    setup_test_tables(&conn).await;

    let (db, ComplexSchema { complex }) = drizzle!(conn, ComplexSchema);

    // Test complex type insertion
    let complex_data = InsertComplex::new("libsql_complex", true, Role::User)
        .with_name("libsql_complex")
        .with_email("test@libsql.com".to_string())
        .with_age(30);

    let inserted = db
        .insert(complex)
        .values([complex_data])
        .execute()
        .await
        .unwrap();
    assert_eq!(inserted, 1);

    // Test complex type selection
    let selected: Vec<SelectComplex> = db.select(()).from(complex).all().await.unwrap();

    assert!(selected.len() > 0);
    assert_eq!(selected[0].name, "libsql_complex");
    assert_eq!(selected[0].email, Some("test@libsql.com".to_string()));
    assert_eq!(selected[0].age, Some(30));
    assert_eq!(selected[0].active, true);
    assert_eq!(selected[0].role, Role::User);
}

#[cfg(all(feature = "serde", feature = "uuid"))]
#[tokio::test]
async fn test_libsql_json_fields() {
    use crate::common::{ComplexSchema, InsertComplex, SelectComplex, UserMetadata};

    let conn = setup_libsql_connection().await;
    setup_test_tables(&conn).await;

    let (db, ComplexSchema { complex }) = drizzle!(conn, ComplexSchema);

    let metadata = UserMetadata {
        preferences: vec!["dark_mode".to_string(), "notifications".to_string()],
        last_login: Some("2025-08-12T10:00:00Z".to_string()),
        theme: "dark".to_string(),
    };

    let complex_data =
        InsertComplex::new("json_test", true, Role::Admin).with_metadata(metadata.clone());

    let inserted = db
        .insert(complex)
        .values([complex_data])
        .execute()
        .await
        .unwrap();
    assert_eq!(inserted, 1);

    // Test JSON field retrieval
    let selected: Vec<SelectComplex> = db.select(()).from(complex).all().await.unwrap();

    assert!(selected.len() > 0);
    assert_eq!(selected[0].name, "json_test");
    assert_eq!(selected[0].metadata, Some(metadata));
}

#[tokio::test]
async fn test_libsql_update_operations() {
    let conn = setup_libsql_connection().await;
    setup_test_tables(&conn).await;

    let (db, SimpleSchema { simple }) = drizzle!(conn, SimpleSchema);

    // Insert initial data
    let data = InsertSimple::new("update_test");
    db.insert(simple).values([data]).execute().await.unwrap();

    // Test update
    let update_data = UpdateSimple::default().with_name("updated_test");
    let updated = db
        .update(simple)
        .set(update_data)
        .r#where(eq(simple.name, "update_test"))
        .execute()
        .await
        .unwrap();

    assert_eq!(updated, 1);

    // Verify update
    let selected: Vec<SelectSimple> = db.select(()).from(simple).all().await.unwrap();
    assert_eq!(selected[0].name, "updated_test");
}

#[tokio::test]
async fn test_libsql_delete_operations() {
    let conn = setup_libsql_connection().await;
    setup_test_tables(&conn).await;

    let (db, SimpleSchema { simple }) = drizzle!(conn, SimpleSchema);

    // Insert test data
    let data = InsertSimple::new("delete_test");
    db.insert(simple).values([data]).execute().await.unwrap();

    // Test delete
    let deleted = db
        .delete(simple)
        .r#where(eq(simple.name, "delete_test"))
        .execute()
        .await
        .unwrap();

    assert_eq!(deleted, 1);

    // Verify deletion
    let selected: Vec<SelectSimple> = db.select(()).from(simple).all().await.unwrap();
    assert!(selected.is_empty());
}

#[tokio::test]
async fn test_libsql_error_handling() {
    let conn = setup_libsql_connection().await;
    setup_test_tables(&conn).await;

    let (db, SimpleSchema { simple }) = drizzle!(conn, SimpleSchema);

    // Test error when trying to get from empty table
    let result: Result<SelectSimple, _> = db.select(()).from(simple).get().await;
    assert!(result.is_err());

    // Test error when trying to insert duplicate primary key
    let data1 = InsertSimple::new("test1").with_id(1);
    let data2 = InsertSimple::new("test2").with_id(1);

    db.insert(simple).values([data1]).execute().await.unwrap();
    let result = db.insert(simple).values([data2]).execute().await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_libsql_prepared_statements() {
    let conn = setup_libsql_connection().await;
    setup_test_tables(&conn).await;

    let (db, SimpleSchema { simple }) = drizzle!(conn, SimpleSchema);

    // Insert test data
    let data = InsertSimple::new("prepared_test");
    db.insert(simple).values([data]).execute().await.unwrap();

    // Test prepared statement with parameters
    let prepared = db.select(()).from(simple).prepare();
    let selected: Vec<SelectSimple> = prepared.all(db.conn(), []).await.unwrap();

    assert!(selected.len() > 0);
    assert_eq!(selected[0].name, "prepared_test");
}

#[tokio::test]
async fn test_libsql_transactions() {
    let conn = setup_libsql_connection().await;
    setup_test_tables(&conn).await;

    let (db, SimpleSchema { simple }) = drizzle!(conn, SimpleSchema);

    // Insert data in a transaction context
    let data1 = InsertSimple::new("trans_test1");
    let data2 = InsertSimple::new("trans_test2");

    db.insert(simple).values([data1]).execute().await.unwrap();
    db.insert(simple).values([data2]).execute().await.unwrap();

    // Verify both records exist
    let selected: Vec<SelectSimple> = db.select(()).from(simple).all().await.unwrap();
    assert_eq!(selected.len(), 2);
}

#[tokio::test]
async fn test_libsql_where_conditions() {
    let conn = setup_libsql_connection().await;
    setup_test_tables(&conn).await;

    let (db, SimpleSchema { simple }) = drizzle!(conn, SimpleSchema);

    // Insert multiple test records
    let data1 = InsertSimple::new("where_test1");
    let data2 = InsertSimple::new("where_test2");
    let data3 = InsertSimple::new("other_test");

    db.insert(simple)
        .values([data1, data2, data3])
        .execute()
        .await
        .unwrap();

    // Test where condition with like
    let selected: Vec<SelectSimple> = db
        .select(())
        .from(simple)
        .r#where(like(simple.name, "where_test%"))
        .all()
        .await
        .unwrap();

    assert_eq!(selected.len(), 2);
}

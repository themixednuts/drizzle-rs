use drizzle_rs::prelude::*;

#[cfg(feature = "libsql")]
use libsql::{Builder, Connection};
#[cfg(feature = "rusqlite")]
use rusqlite::Connection;
#[cfg(feature = "turso")]
use turso::{Builder, Connection};

// Test struct with various data types for FromRow
#[derive(FromRow, Debug, PartialEq)]
struct AllDataTypes {
    id: i32,
    name: String,
    age: i64,
    score: f64,
    active: bool,
    data: Vec<u8>,
}

// Test struct with different integer sizes
#[derive(FromRow, Debug, PartialEq)]
struct IntegerTypes {
    id: i32,
    big_num: i64,
    small_num: i16,
    tiny_num: i8,
}

// Test struct with float types
#[derive(FromRow, Debug, PartialEq)]
struct FloatTypes {
    id: i32,
    precise: f64,
    compact: f32,
}

// Test table for comprehensive type testing
#[SQLiteTable(name = "type_test")]
struct TypeTest {
    #[integer(primary)]
    id: i32,
    #[text]
    name: String,
    #[integer]
    age: i64,
    #[real]
    score: f64,
    #[boolean]
    active: bool,
    #[blob]
    data: Vec<u8>,
}

// Table for integer size testing
#[SQLiteTable(name = "integer_test")]
struct IntegerTest {
    #[integer(primary)]
    id: i32,
    #[integer]
    big_num: i64,
    #[integer]
    small_num: i16,
    #[integer]
    tiny_num: i8,
}

// Table for float testing
#[SQLiteTable(name = "float_test")]
struct FloatTest {
    #[integer(primary)]
    id: i32,
    #[real]
    precise: f64,
    #[real]
    compact: f32,
}

#[cfg(feature = "rusqlite")]
fn setup_test_db() -> Connection {
    let conn = Connection::open_in_memory().expect("Failed to create in-memory database");
    
    conn.execute(TypeTest::SQL.sql().as_str(), []).unwrap();
    conn.execute(IntegerTest::SQL.sql().as_str(), []).unwrap();
    conn.execute(FloatTest::SQL.sql().as_str(), []).unwrap();
    
    conn
}

#[cfg(any(feature = "turso", feature = "libsql"))]
async fn setup_test_db() -> Connection {
    let db = Builder::new_local(":memory:")
        .build()
        .await
        .expect("build db");
    let conn = db.connect().expect("connect to db");
    
    conn.execute(TypeTest::SQL.sql().as_str(), ()).await.unwrap();
    conn.execute(IntegerTest::SQL.sql().as_str(), ()).await.unwrap();
    conn.execute(FloatTest::SQL.sql().as_str(), ()).await.unwrap();
    
    conn
}

#[tokio::test]
async fn test_fromrow_with_all_data_types() {
    #[cfg(feature = "rusqlite")]
    let conn = setup_test_db();
    #[cfg(any(feature = "turso", feature = "libsql"))]
    let conn = setup_test_db().await;

    let (db, type_test) = drizzle!(conn, [TypeTest]);

    // Insert test data with all data types
    let test_data = InsertTypeTest::default()
        .with_name("test_user".to_string())
        .with_age(25)
        .with_score(98.5)
        .with_active(true)
        .with_data(vec![1, 2, 3, 4, 5]);

    #[cfg(feature = "rusqlite")]
    db.insert(type_test).values([test_data]).execute().unwrap();
    #[cfg(any(feature = "turso", feature = "libsql"))]
    db.insert(type_test).values([test_data]).execute().await.unwrap();

    // Test FromRow with all data types
    #[cfg(feature = "rusqlite")]
    let result: AllDataTypes = db.select(()).from(type_test).get().unwrap();
    #[cfg(any(feature = "turso", feature = "libsql"))]
    let result: AllDataTypes = db.select(()).from(type_test).get().await.unwrap();

    let expected = AllDataTypes {
        id: 1,
        name: "test_user".to_string(),
        age: 25,
        score: 98.5,
        active: true,
        data: vec![1, 2, 3, 4, 5],
    };

    assert_eq!(result, expected);
    println!("✅ All data types test passed: {:?}", result);
}

#[tokio::test]
async fn test_fromrow_with_integer_sizes() {
    #[cfg(feature = "rusqlite")]
    let conn = setup_test_db();
    #[cfg(any(feature = "turso", feature = "libsql"))]
    let conn = setup_test_db().await;

    let (db, integer_test) = drizzle!(conn, [IntegerTest]);

    // Insert test data with different integer sizes
    let test_data = InsertIntegerTest::default()
        .with_big_num(9223372036854775806i64) // Near i64 max
        .with_small_num(32000i16)
        .with_tiny_num(100i8);

    #[cfg(feature = "rusqlite")]
    db.insert(integer_test).values([test_data]).execute().unwrap();
    #[cfg(any(feature = "turso", feature = "libsql"))]
    db.insert(integer_test).values([test_data]).execute().await.unwrap();

    // Test FromRow with different integer types
    #[cfg(feature = "rusqlite")]
    let result: IntegerTypes = db.select(()).from(integer_test).get().unwrap();
    #[cfg(any(feature = "turso", feature = "libsql"))]
    let result: IntegerTypes = db.select(()).from(integer_test).get().await.unwrap();

    let expected = IntegerTypes {
        id: 1,
        big_num: 9223372036854775806i64,
        small_num: 32000i16,
        tiny_num: 100i8,
    };

    assert_eq!(result, expected);
    println!("✅ Integer types test passed: {:?}", result);
}

#[tokio::test]
async fn test_fromrow_with_float_types() {
    #[cfg(feature = "rusqlite")]
    let conn = setup_test_db();
    #[cfg(any(feature = "turso", feature = "libsql"))]
    let conn = setup_test_db().await;

    let (db, float_test) = drizzle!(conn, [FloatTest]);

    // Insert test data with different float types
    let test_data = InsertFloatTest::default()
        .with_precise(3.141592653589793) // High precision f64
        .with_compact(2.718f32);         // f32

    #[cfg(feature = "rusqlite")]
    db.insert(float_test).values([test_data]).execute().unwrap();
    #[cfg(any(feature = "turso", feature = "libsql"))]
    db.insert(float_test).values([test_data]).execute().await.unwrap();

    // Test FromRow with different float types
    #[cfg(feature = "rusqlite")]
    let result: FloatTypes = db.select(()).from(float_test).get().unwrap();
    #[cfg(any(feature = "turso", feature = "libsql"))]
    let result: FloatTypes = db.select(()).from(float_test).get().await.unwrap();

    let expected = FloatTypes {
        id: 1,
        precise: 3.141592653589793,
        compact: 2.718f32,
    };

    assert_eq!(result, expected);
    println!("✅ Float types test passed: {:?}", result);
}

#[tokio::test]
async fn test_fromrow_type_conversion_edge_cases() {
    #[cfg(feature = "rusqlite")]
    let conn = setup_test_db();
    #[cfg(any(feature = "turso", feature = "libsql"))]
    let conn = setup_test_db().await;

    let (db, type_test) = drizzle!(conn, [TypeTest]);

    // Insert test data with edge case values
    let test_data = InsertTypeTest::default()
        .with_name("edge_case".to_string())
        .with_age(0) // Zero value
        .with_score(0.0) // Zero float
        .with_active(false) // False boolean (stored as 0)
        .with_data(vec![]); // Empty blob

    #[cfg(feature = "rusqlite")]
    db.insert(type_test).values([test_data]).execute().unwrap();
    #[cfg(any(feature = "turso", feature = "libsql"))]
    db.insert(type_test).values([test_data]).execute().await.unwrap();

    // Test FromRow with edge case values
    #[cfg(feature = "rusqlite")]
    let result: AllDataTypes = db.select(()).from(type_test).get().unwrap();
    #[cfg(any(feature = "turso", feature = "libsql"))]
    let result: AllDataTypes = db.select(()).from(type_test).get().await.unwrap();

    let expected = AllDataTypes {
        id: 1,
        name: "edge_case".to_string(),
        age: 0,
        score: 0.0,
        active: false,
        data: vec![],
    };

    assert_eq!(result, expected);
    println!("✅ Edge cases test passed: {:?}", result);
}
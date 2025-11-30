//! PostgreSQL ArrayVec tests - tests ArrayString and ArrayVec storage
#![cfg(all(feature = "postgres", feature = "arrayvec"))]

use arrayvec::{ArrayString, ArrayVec};
use drizzle::postgres::QueryBuilder;
use drizzle::prelude::*;
use drizzle_macros::{PostgresSchema, PostgresTable};

// Test table with ArrayString as TEXT column
#[PostgresTable(name = "pg_arraystring_test")]
struct PgArrayStringTest {
    #[serial(primary)]
    id: i32,
    #[text] // ArrayString<16> stored as TEXT
    name: ArrayString<16>,
    #[text]
    description: String,
}

// Test table with ArrayVec<u8, N> as BYTEA column
#[PostgresTable(name = "pg_arrayvec_blob_test")]
struct PgArrayVecBlobTest {
    #[serial(primary)]
    id: i32,
    #[bytea] // ArrayVec<u8, 32> stored as BYTEA
    data: ArrayVec<u8, 32>,
    #[text]
    label: String,
}

// Test table with mixed arrayvec types
#[PostgresTable(name = "pg_mixed_arrayvec_test")]
struct PgMixedArrayVecTest {
    #[serial(primary)]
    id: i32,
    #[text]
    short_name: ArrayString<8>,
    #[text]
    long_name: ArrayString<64>,
    #[bytea]
    small_data: ArrayVec<u8, 16>,
    #[bytea]
    large_data: ArrayVec<u8, 128>,
}

#[derive(PostgresSchema)]
struct PgArrayStringSchema {
    pg_arraystring_test: PgArrayStringTest,
}

#[derive(PostgresSchema)]
struct PgArrayVecBlobSchema {
    pg_arrayvec_blob_test: PgArrayVecBlobTest,
}

#[derive(PostgresSchema)]
struct PgMixedArrayVecSchema {
    pg_mixed_arrayvec_test: PgMixedArrayVecTest,
}

#[test]
fn test_arraystring_insert_sql_generation() {
    let PgArrayStringSchema {
        pg_arraystring_test: table,
    } = PgArrayStringSchema::new();
    let qb = QueryBuilder::new::<PgArrayStringSchema>();

    // Create ArrayString
    let name = ArrayString::<16>::from("Hello").unwrap();
    let insert = InsertPgArrayStringTest::new(name, "test description");

    let sql = qb.insert(table).values([insert]).to_sql();
    let sql_string = sql.sql();

    println!("ArrayString insert SQL: {}", sql_string);

    assert!(sql_string.contains("INSERT INTO"));
    assert!(sql_string.contains(r#""pg_arraystring_test""#));
    assert!(sql_string.contains("$1"));
    assert!(sql_string.contains("$2"));
}

#[test]
fn test_arrayvec_bytea_insert_sql_generation() {
    let PgArrayVecBlobSchema {
        pg_arrayvec_blob_test: table,
    } = PgArrayVecBlobSchema::new();
    let qb = QueryBuilder::new::<PgArrayVecBlobSchema>();

    // Create ArrayVec with some bytes
    let mut data_vec = ArrayVec::<u8, 32>::new();
    data_vec.extend([1, 2, 3, 4, 5]);
    let insert = InsertPgArrayVecBlobTest::new(data_vec, "blob test");

    let sql = qb.insert(table).values([insert]).to_sql();
    let sql_string = sql.sql();

    println!("ArrayVec bytea insert SQL: {}", sql_string);

    assert!(sql_string.contains("INSERT INTO"));
    assert!(sql_string.contains(r#""pg_arrayvec_blob_test""#));
    assert!(sql_string.contains("$1"));
    assert!(sql_string.contains("$2"));
}

#[test]
fn test_arraystring_select_sql_generation() {
    let PgArrayStringSchema {
        pg_arraystring_test: table,
    } = PgArrayStringSchema::new();
    let qb = QueryBuilder::new::<PgArrayStringSchema>();

    let sql = qb
        .select((table.id, table.name, table.description))
        .from(table)
        .to_sql();
    let sql_string = sql.sql();

    println!("ArrayString select SQL: {}", sql_string);

    assert!(sql_string.contains("SELECT"));
    assert!(sql_string.contains(r#""pg_arraystring_test"."id""#));
    assert!(sql_string.contains(r#""pg_arraystring_test"."name""#));
    assert!(sql_string.contains(r#""pg_arraystring_test"."description""#));
    assert!(sql_string.contains("FROM"));
    assert!(sql_string.contains(r#""pg_arraystring_test""#));
}

#[test]
fn test_arrayvec_update_sql_generation() {
    use drizzle_core::expressions::conditions::eq;

    let PgArrayVecBlobSchema {
        pg_arrayvec_blob_test: table,
    } = PgArrayVecBlobSchema::new();
    let qb = QueryBuilder::new::<PgArrayVecBlobSchema>();

    let mut new_data = ArrayVec::<u8, 32>::new();
    new_data.extend([9, 8, 7, 6, 5]);

    let sql = qb
        .update(table)
        .set(UpdatePgArrayVecBlobTest::default().with_data(new_data))
        .r#where(eq(table.id, 1))
        .to_sql();
    let sql_string = sql.sql();

    println!("ArrayVec update SQL: {}", sql_string);

    assert!(sql_string.contains("UPDATE"));
    assert!(sql_string.contains(r#""pg_arrayvec_blob_test""#));
    assert!(sql_string.contains("SET"));
    assert!(sql_string.contains(r#""data" = $1"#));
    assert!(sql_string.contains("WHERE"));
    assert!(sql_string.contains(r#""pg_arrayvec_blob_test"."id" = $2"#));
}

#[test]
fn test_arraystring_empty() {
    let PgArrayStringSchema {
        pg_arraystring_test: table,
    } = PgArrayStringSchema::new();
    let qb = QueryBuilder::new::<PgArrayStringSchema>();

    // Test with empty ArrayString
    let name = ArrayString::<16>::new();
    let insert = InsertPgArrayStringTest::new(name, "empty test");

    let sql = qb.insert(table).values([insert]).to_sql();
    let sql_string = sql.sql();

    println!("Empty ArrayString insert SQL: {}", sql_string);

    assert!(sql_string.contains("INSERT INTO"));
    assert!(sql_string.contains(r#""pg_arraystring_test""#));
}

#[test]
fn test_arrayvec_empty() {
    let PgArrayVecBlobSchema {
        pg_arrayvec_blob_test: table,
    } = PgArrayVecBlobSchema::new();
    let qb = QueryBuilder::new::<PgArrayVecBlobSchema>();

    // Test with empty ArrayVec
    let data_vec = ArrayVec::<u8, 32>::new();
    let insert = InsertPgArrayVecBlobTest::new(data_vec, "empty blob");

    let sql = qb.insert(table).values([insert]).to_sql();
    let sql_string = sql.sql();

    println!("Empty ArrayVec insert SQL: {}", sql_string);

    assert!(sql_string.contains("INSERT INTO"));
    assert!(sql_string.contains(r#""pg_arrayvec_blob_test""#));
}

#[test]
fn test_arraystring_max_capacity() {
    let PgArrayStringSchema {
        pg_arraystring_test: table,
    } = PgArrayStringSchema::new();
    let qb = QueryBuilder::new::<PgArrayStringSchema>();

    // Test with ArrayString at maximum capacity (16 chars)
    let name = ArrayString::<16>::from("1234567890123456").unwrap();
    let insert = InsertPgArrayStringTest::new(name, "max capacity");

    let sql = qb.insert(table).values([insert]).to_sql();
    let sql_string = sql.sql();

    println!("Max capacity ArrayString insert SQL: {}", sql_string);

    assert!(sql_string.contains("INSERT INTO"));
    assert!(sql_string.contains(r#""pg_arraystring_test""#));
}

#[test]
fn test_arrayvec_max_capacity() {
    let PgArrayVecBlobSchema {
        pg_arrayvec_blob_test: table,
    } = PgArrayVecBlobSchema::new();
    let qb = QueryBuilder::new::<PgArrayVecBlobSchema>();

    // Test with ArrayVec at maximum capacity (32 bytes)
    let mut data_vec = ArrayVec::<u8, 32>::new();
    for i in 0..32 {
        data_vec.push(i as u8);
    }
    let insert = InsertPgArrayVecBlobTest::new(data_vec, "max capacity");

    let sql = qb.insert(table).values([insert]).to_sql();
    let sql_string = sql.sql();

    println!("Max capacity ArrayVec insert SQL: {}", sql_string);

    assert!(sql_string.contains("INSERT INTO"));
    assert!(sql_string.contains(r#""pg_arrayvec_blob_test""#));
}

#[test]
fn test_schema_create_table_sql() {
    let schema = PgArrayStringSchema::new();
    let create_sql = schema.pg_arraystring_test.sql().sql();

    println!("ArrayString table CREATE SQL: {}", create_sql);

    assert!(create_sql.contains(r#"CREATE TABLE "pg_arraystring_test""#));
    assert!(create_sql.contains(r#""id" SERIAL PRIMARY KEY"#));
    assert!(create_sql.contains(r#""name" TEXT"#));
    assert!(create_sql.contains(r#""description" TEXT"#));
}

#[test]
fn test_arrayvec_schema_create_table_sql() {
    let schema = PgArrayVecBlobSchema::new();
    let create_sql = schema.pg_arrayvec_blob_test.sql().sql();

    println!("ArrayVec table CREATE SQL: {}", create_sql);

    assert!(create_sql.contains(r#"CREATE TABLE "pg_arrayvec_blob_test""#));
    assert!(create_sql.contains(r#""id" SERIAL PRIMARY KEY"#));
    assert!(create_sql.contains(r#""data" BYTEA"#));
    assert!(create_sql.contains(r#""label" TEXT"#));
}

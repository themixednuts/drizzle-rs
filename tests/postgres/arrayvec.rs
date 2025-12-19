//! PostgreSQL ArrayVec tests - tests ArrayString and ArrayVec storage

#![cfg(all(
    feature = "arrayvec",
    any(feature = "postgres-sync", feature = "tokio-postgres")
))]

use arrayvec::{ArrayString, ArrayVec};
use drizzle::core::expressions::*;
use drizzle::postgres::prelude::*;
use drizzle_macros::{PostgresFromRow, PostgresSchema, PostgresTable, postgres_test};

#[PostgresTable(name = "pg_arraystring_test")]
struct PgArrayStringTest {
    #[column(primary, serial)]
    id: i32,
    name: ArrayString<16>,
    description: String,
}

#[PostgresTable(name = "pg_arrayvec_blob_test")]
struct PgArrayVecBlobTest {
    #[column(primary, serial)]
    id: i32,
    data: ArrayVec<u8, 32>,
    label: String,
}

#[derive(PostgresSchema)]
struct PgArrayStringSchema {
    table: PgArrayStringTest,
}

#[derive(PostgresSchema)]
struct PgArrayVecBlobSchema {
    table: PgArrayVecBlobTest,
}

#[derive(Debug, PostgresFromRow)]
struct ArrayStringResult {
    id: i32,
    name: ArrayString<16>,
    description: String,
}

#[derive(Debug, PostgresFromRow)]
struct ArrayVecBlobResult {
    id: i32,
    data: ArrayVec<u8, 32>,
    label: String,
}

postgres_test!(arraystring_insert_and_select, PgArrayStringSchema, {
    let PgArrayStringSchema { table } = schema;

    let name = ArrayString::<16>::from("Hello").unwrap();
    let stmt = db
        .insert(table)
        .values([InsertPgArrayStringTest::new(name, "test description")]);
    drizzle_exec!(stmt.execute());

    let stmt = db.select(()).from(table);
    let results: Vec<ArrayStringResult> = drizzle_exec!(stmt.all());

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].name.as_str(), "Hello");
    assert_eq!(results[0].description, "test description");
});

postgres_test!(arrayvec_blob_insert_and_select, PgArrayVecBlobSchema, {
    let PgArrayVecBlobSchema { table } = schema;

    let mut data = ArrayVec::<u8, 32>::new();
    data.extend([1, 2, 3, 4, 5]);

    let stmt = db
        .insert(table)
        .values([InsertPgArrayVecBlobTest::new(data.clone(), "blob test")]);
    drizzle_exec!(stmt.execute());

    let stmt = db.select(()).from(table);
    let results: Vec<ArrayVecBlobResult> = drizzle_exec!(stmt.all());

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].data.as_slice(), &[1, 2, 3, 4, 5]);
    assert_eq!(results[0].label, "blob test");
});

postgres_test!(arraystring_empty, PgArrayStringSchema, {
    let PgArrayStringSchema { table } = schema;

    let name = ArrayString::<16>::new();
    let stmt = db
        .insert(table)
        .values([InsertPgArrayStringTest::new(name, "empty name")]);
    drizzle_exec!(stmt.execute());

    let stmt = db.select(()).from(table);
    let results: Vec<ArrayStringResult> = drizzle_exec!(stmt.all());

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].name.as_str(), "");
});

postgres_test!(arrayvec_empty, PgArrayVecBlobSchema, {
    let PgArrayVecBlobSchema { table } = schema;

    let data = ArrayVec::<u8, 32>::new();
    let stmt = db
        .insert(table)
        .values([InsertPgArrayVecBlobTest::new(data, "empty blob")]);
    drizzle_exec!(stmt.execute());

    let stmt = db.select(()).from(table);
    let results: Vec<ArrayVecBlobResult> = drizzle_exec!(stmt.all());

    assert_eq!(results.len(), 1);
    assert!(results[0].data.is_empty());
});

postgres_test!(arraystring_max_capacity, PgArrayStringSchema, {
    let PgArrayStringSchema { table } = schema;

    // 16 character string - max capacity
    let name = ArrayString::<16>::from("1234567890123456").unwrap();
    let stmt = db
        .insert(table)
        .values([InsertPgArrayStringTest::new(name, "max capacity")]);
    drizzle_exec!(stmt.execute());

    let stmt = db.select(()).from(table);
    let results: Vec<ArrayStringResult> = drizzle_exec!(stmt.all());

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].name.as_str(), "1234567890123456");
});

postgres_test!(arrayvec_max_capacity, PgArrayVecBlobSchema, {
    let PgArrayVecBlobSchema { table } = schema;

    // 32 bytes - max capacity
    let mut data = ArrayVec::<u8, 32>::new();
    for i in 0..32 {
        data.push(i as u8);
    }

    let stmt = db
        .insert(table)
        .values([InsertPgArrayVecBlobTest::new(data.clone(), "max capacity")]);
    drizzle_exec!(stmt.execute());

    let stmt = db.select(()).from(table);
    let results: Vec<ArrayVecBlobResult> = drizzle_exec!(stmt.all());

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].data.len(), 32);
    for i in 0..32 {
        assert_eq!(results[0].data[i], i as u8);
    }
});

postgres_test!(arrayvec_update, PgArrayVecBlobSchema, {
    let PgArrayVecBlobSchema { table } = schema;

    let mut initial = ArrayVec::<u8, 32>::new();
    initial.extend([1, 2, 3]);

    let stmt = db
        .insert(table)
        .values([InsertPgArrayVecBlobTest::new(initial, "to update")]);
    drizzle_exec!(stmt.execute());

    let mut updated = ArrayVec::<u8, 32>::new();
    updated.extend([9, 8, 7, 6, 5]);

    let stmt = db
        .update(table)
        .set(UpdatePgArrayVecBlobTest::default().with_data(updated.clone()))
        .r#where(eq(table.label, "to update"));
    drizzle_exec!(stmt.execute());

    let stmt = db.select(()).from(table);
    let results: Vec<ArrayVecBlobResult> = drizzle_exec!(stmt.all());

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].data.as_slice(), &[9, 8, 7, 6, 5]);
});

#[PostgresTable(name = "pg_array_nullable_test")]
struct PgArrayNullableTest {
    #[column(primary, serial)]
    id: i32,
    name: Option<ArrayString<16>>,
    data: Option<ArrayVec<u8, 32>>,
}

#[derive(PostgresSchema)]
struct PgArrayNullableSchema {
    table: PgArrayNullableTest,
}

#[derive(Debug, PostgresFromRow)]
struct ArrayNullableResult {
    id: i32,
    name: Option<ArrayString<16>>,
    data: Option<ArrayVec<u8, 32>>,
}

postgres_test!(array_nullable_test, PgArrayNullableSchema, {
    let PgArrayNullableSchema { table } = schema;

    // Test inserting Some values
    let name = ArrayString::<16>::from("Some Name").unwrap();
    let mut data = ArrayVec::<u8, 32>::new();
    data.extend([10, 20, 30]);

    let stmt = db.insert(table).values([InsertPgArrayNullableTest::new()
        .with_name(name)
        .with_data(data.clone())]);
    drizzle_exec!(stmt.execute());

    // Test inserting None values
    let stmt = db.insert(table).values([InsertPgArrayNullableTest::new()]);
    drizzle_exec!(stmt.execute());

    let stmt = db.select(()).from(table).order_by(table.id);
    let results: Vec<ArrayNullableResult> = drizzle_exec!(stmt.all());

    assert_eq!(results.len(), 2);

    // Check first row (Some values)
    assert_eq!(results[0].name.as_ref().unwrap().as_str(), "Some Name");
    assert_eq!(results[0].data.as_ref().unwrap().as_slice(), &[10, 20, 30]);

    // Check second row (None values)
    assert!(results[1].name.is_none());
    assert!(results[1].data.is_none());
});

postgres_test!(arraystring_unicode_boundary, PgArrayStringSchema, {
    let PgArrayStringSchema { table } = schema;

    // "こんにちは" is 15 bytes (3 bytes per char * 5 chars)
    // Capacity is 16, so this fits
    let name = ArrayString::<16>::from("こんにちは").unwrap();
    let stmt = db
        .insert(table)
        .values([InsertPgArrayStringTest::new(name, "unicode test")]);
    drizzle_exec!(stmt.execute());

    let stmt = db.select(()).from(table);
    let results: Vec<ArrayStringResult> = drizzle_exec!(stmt.all());

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].name.as_str(), "こんにちは");
});

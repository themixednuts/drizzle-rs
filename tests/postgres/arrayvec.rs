//! PostgreSQL ArrayVec tests - tests ArrayString and ArrayVec storage

#![cfg(all(
    feature = "arrayvec",
    any(feature = "postgres-sync", feature = "tokio-postgres")
))]

use arrayvec::{ArrayString, ArrayVec};
use drizzle::prelude::*;
use drizzle_macros::{PostgresFromRow, PostgresSchema, PostgresTable, postgres_test};

#[PostgresTable(name = "pg_arraystring_test")]
struct PgArrayStringTest {
    #[serial(primary)]
    id: i32,
    #[text]
    name: ArrayString<16>,
    #[text]
    description: String,
}

#[PostgresTable(name = "pg_arrayvec_blob_test")]
struct PgArrayVecBlobTest {
    #[serial(primary)]
    id: i32,
    #[bytea]
    data: ArrayVec<u8, 32>,
    #[text]
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
    name: String,
    description: String,
}

#[derive(Debug, PostgresFromRow)]
struct ArrayVecBlobResult {
    id: i32,
    data: Vec<u8>,
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
    assert_eq!(results[0].name, "Hello");
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
    assert_eq!(results[0].data, vec![1, 2, 3, 4, 5]);
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
    assert_eq!(results[0].name, "");
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
    assert_eq!(results[0].name, "1234567890123456");
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
        .set(UpdatePgArrayVecBlobTest::default().with_data(updated))
        .r#where(eq(table.label, "to update"));
    drizzle_exec!(stmt.execute());

    let stmt = db.select(()).from(table);
    let results: Vec<ArrayVecBlobResult> = drizzle_exec!(stmt.all());

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].data, vec![9, 8, 7, 6, 5]);
});

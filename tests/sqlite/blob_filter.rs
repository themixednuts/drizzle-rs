#![cfg(any(feature = "rusqlite", feature = "turso", feature = "libsql"))]

use drizzle::core::expr::*;
use drizzle::sqlite::prelude::*;

#[SQLiteTable(NAME = "blob_filter_test")]
struct BlobFilterTest {
    #[column(PRIMARY)]
    id: i32,
    hash: Vec<u8>,
}

#[derive(SQLiteSchema)]
struct BlobFilterSchema {
    blob_filter_test: BlobFilterTest,
}

#[drizzle::test]
fn eq_matches_blob_with_borrowed_byte_array(db: &mut TestDb<BlobFilterSchema>) {
    let table = schema.blob_filter_test;
    let first = [1_u8; 32];
    let second = [2_u8; 32];

    db.insert(table)
        .values([
            InsertBlobFilterTest::new(first.to_vec()).with_id(1),
            InsertBlobFilterTest::new(second.to_vec()).with_id(2),
        ])
        .execute();

    let sql = db
        .select(table.id)
        .from(table)
        .r#where(eq(table.hash, &second))
        .to_sql();
    let params: Vec<_> = sql.params().collect();

    assert_eq!(
        sql.sql(),
        r#"SELECT "blob_filter_test"."id" FROM "blob_filter_test" WHERE "blob_filter_test"."hash" = ?"#
    );
    assert_eq!(params.len(), 1);
    assert_eq!(params[0].as_bytes(), Some(second.as_slice()));

    #[derive(SQLiteFromRow, Debug)]
    struct IdRow {
        id: i32,
    }

    let rows: Vec<IdRow> = db
        .select(table.id)
        .from(table)
        .r#where(eq(table.hash, &second))
        .all();

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].id, 2);
}

#[drizzle::test]
fn eq_matches_blob_with_borrowed_byte_slice(db: &mut TestDb<BlobFilterSchema>) {
    let table = schema.blob_filter_test;
    let first = [3_u8; 32];
    let second = [4_u8; 32];

    db.insert(table)
        .values([
            InsertBlobFilterTest::new(first.to_vec()).with_id(3),
            InsertBlobFilterTest::new(second.to_vec()).with_id(4),
        ])
        .execute();

    #[derive(SQLiteFromRow, Debug)]
    struct IdRow {
        id: i32,
    }

    let rows: Vec<IdRow> = db
        .select(table.id)
        .from(table)
        .r#where(eq(table.hash, second.as_slice()))
        .all();

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].id, 4);
}

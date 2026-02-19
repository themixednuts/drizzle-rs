#![cfg(all(
    any(feature = "postgres-sync", feature = "tokio-postgres"),
    any(feature = "compact-str", feature = "bytes", feature = "smallvec-types")
))]

use drizzle::postgres::prelude::*;
use drizzle_macros::{PostgresFromRow, PostgresSchema, PostgresTable, postgres_test};

#[cfg(feature = "compact-str")]
use compact_str::CompactString;

#[cfg(feature = "bytes")]
use bytes::{Bytes, BytesMut};

#[cfg(feature = "smallvec-types")]
use smallvec::SmallVec;

#[cfg(feature = "compact-str")]
#[PostgresTable(name = "pg_compact_string_test")]
struct PgCompactStringTest {
    #[column(primary, serial)]
    id: i32,
    name: CompactString,
    note: String,
}

#[cfg(feature = "compact-str")]
#[derive(PostgresSchema)]
struct PgCompactStringSchema {
    table: PgCompactStringTest,
}

#[allow(dead_code)]
#[cfg(feature = "compact-str")]
#[derive(Debug, PostgresFromRow)]
struct CompactStringRow {
    id: i32,
    name: CompactString,
    note: String,
}

#[cfg(feature = "compact-str")]
postgres_test!(compact_string_roundtrip, PgCompactStringSchema, {
    let PgCompactStringSchema { table } = schema;

    let value = CompactString::new("pg compact");
    let stmt = db.insert(table).values([InsertPgCompactStringTest::new(
        value.clone(),
        "compact note",
    )]);
    drizzle_exec!(stmt => execute);

    let stmt = db.select(()).from(table);
    let rows: Vec<CompactStringRow> = drizzle_exec!(stmt => all);

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].name, value);
    assert_eq!(rows[0].note, "compact note");
});

#[cfg(feature = "bytes")]
#[PostgresTable(name = "pg_bytes_blob_test")]
struct PgBytesBlobTest {
    #[column(primary, serial)]
    id: i32,
    payload: Bytes,
    mutable_payload: BytesMut,
    note: String,
}

#[cfg(feature = "bytes")]
#[derive(PostgresSchema)]
struct PgBytesBlobSchema {
    table: PgBytesBlobTest,
}

#[allow(dead_code)]
#[cfg(feature = "bytes")]
#[derive(Debug, PostgresFromRow)]
struct BytesBlobRow {
    id: i32,
    payload: Bytes,
    mutable_payload: BytesMut,
    note: String,
}

#[cfg(feature = "bytes")]
postgres_test!(bytes_roundtrip, PgBytesBlobSchema, {
    let PgBytesBlobSchema { table } = schema;

    let payload = Bytes::from_static(b"pg-bytes");
    let mutable_payload = BytesMut::from(&b"pg-bytes-mut"[..]);

    let stmt = db.insert(table).values([InsertPgBytesBlobTest::new(
        payload.clone(),
        mutable_payload.clone(),
        "bytes note",
    )]);
    drizzle_exec!(stmt => execute);

    let stmt = db.select(()).from(table);
    let rows: Vec<BytesBlobRow> = drizzle_exec!(stmt => all);

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].payload.as_ref(), payload.as_ref());
    assert_eq!(rows[0].mutable_payload.as_ref(), mutable_payload.as_ref());
    assert_eq!(rows[0].note, "bytes note");
});

#[cfg(feature = "smallvec-types")]
#[PostgresTable(name = "pg_smallvec_blob_test")]
struct PgSmallVecBlobTest {
    #[column(primary, serial)]
    id: i32,
    payload: SmallVec<[u8; 16]>,
    note: String,
}

#[cfg(feature = "smallvec-types")]
#[derive(PostgresSchema)]
struct PgSmallVecBlobSchema {
    table: PgSmallVecBlobTest,
}

#[allow(dead_code)]
#[cfg(feature = "smallvec-types")]
#[derive(Debug, PostgresFromRow)]
struct SmallVecBlobRow {
    id: i32,
    payload: SmallVec<[u8; 16]>,
    note: String,
}

#[cfg(feature = "smallvec-types")]
postgres_test!(smallvec_roundtrip, PgSmallVecBlobSchema, {
    let PgSmallVecBlobSchema { table } = schema;

    let mut payload = SmallVec::<[u8; 16]>::new();
    payload.extend_from_slice(&[11, 22, 33, 44]);

    let stmt = db.insert(table).values([InsertPgSmallVecBlobTest::new(
        payload.clone(),
        "smallvec note",
    )]);
    drizzle_exec!(stmt => execute);

    let stmt = db.select(()).from(table);
    let rows: Vec<SmallVecBlobRow> = drizzle_exec!(stmt => all);

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].payload.as_slice(), payload.as_slice());
    assert_eq!(rows[0].note, "smallvec note");
});

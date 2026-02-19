#![cfg(all(
    any(feature = "rusqlite", feature = "turso", feature = "libsql"),
    any(feature = "compact-str", feature = "bytes", feature = "smallvec-types")
))]

use drizzle::core::expr::*;
use drizzle::sqlite::prelude::*;
use drizzle_macros::sqlite_test;

#[cfg(feature = "compact-str")]
use compact_str::CompactString;

#[cfg(feature = "bytes")]
use bytes::{Bytes, BytesMut};

#[cfg(feature = "smallvec-types")]
use smallvec::SmallVec;

#[cfg(feature = "compact-str")]
#[SQLiteTable(NAME = "compact_string_test")]
struct CompactStringTest {
    #[column(PRIMARY)]
    id: i32,
    name: CompactString,
    note: String,
}

#[cfg(feature = "compact-str")]
#[derive(SQLiteSchema)]
struct CompactStringSchema {
    compact_string_test: CompactStringTest,
}

#[cfg(feature = "compact-str")]
sqlite_test!(compact_string_roundtrip_and_storage, CompactStringSchema, {
    let table = schema.compact_string_test;

    let value = CompactString::new("compact hello");
    let row = InsertCompactStringTest::new(value.clone(), "compact note");
    drizzle_exec!(db.insert(table).values([row]) => execute);

    let out: Vec<SelectCompactStringTest> = drizzle_exec!(
        db.select((table.id, table.name, table.note))
            .from(table)
            .r#where(eq(table.id, 1))
            => all
    );

    assert_eq!(out.len(), 1);
    assert_eq!(out[0].name, value);
    assert_eq!(out[0].note, "compact note");

    #[derive(SQLiteFromRow, Debug)]
    struct Ty(String);
    let ty: Ty = drizzle_exec!(
        db.select(r#typeof(table.name).alias("name_type"))
            .from(table)
            .r#where(eq(table.id, 1))
            => get
    );

    assert_eq!(ty.0, "text");
});

#[cfg(feature = "bytes")]
#[SQLiteTable(NAME = "bytes_blob_test")]
struct BytesBlobTest {
    #[column(PRIMARY)]
    id: i32,
    payload: Bytes,
    mutable_payload: BytesMut,
    note: String,
}

#[cfg(feature = "bytes")]
#[derive(SQLiteSchema)]
struct BytesBlobSchema {
    bytes_blob_test: BytesBlobTest,
}

#[cfg(feature = "bytes")]
sqlite_test!(bytes_roundtrip_and_storage, BytesBlobSchema, {
    let table = schema.bytes_blob_test;

    let payload = Bytes::from_static(b"hello-bytes");
    let mutable_payload = BytesMut::from(&b"hello-bytes-mut"[..]);
    let row = InsertBytesBlobTest::new(payload.clone(), mutable_payload.clone(), "bytes note");
    drizzle_exec!(db.insert(table).values([row]) => execute);

    let out: Vec<SelectBytesBlobTest> = drizzle_exec!(
        db.select((table.id, table.payload, table.mutable_payload, table.note))
            .from(table)
            .r#where(eq(table.id, 1))
            => all
    );

    assert_eq!(out.len(), 1);
    assert_eq!(out[0].payload.as_ref(), payload.as_ref());
    assert_eq!(out[0].mutable_payload.as_ref(), mutable_payload.as_ref());
    assert_eq!(out[0].note, "bytes note");

    #[derive(SQLiteFromRow, Debug)]
    struct Ty(String, String);
    let ty: Ty = drizzle_exec!(
        db.select((
            r#typeof(table.payload).alias("payload_type"),
            r#typeof(table.mutable_payload).alias("mutable_payload_type")
        ))
        .from(table)
        .r#where(eq(table.id, 1))
        => get
    );

    assert_eq!(ty.0, "blob");
    assert_eq!(ty.1, "blob");
});

#[cfg(feature = "smallvec-types")]
#[SQLiteTable(NAME = "smallvec_blob_test")]
struct SmallVecBlobTest {
    #[column(PRIMARY)]
    id: i32,
    payload: SmallVec<[u8; 16]>,
    note: String,
}

#[cfg(feature = "smallvec-types")]
#[derive(SQLiteSchema)]
struct SmallVecBlobSchema {
    smallvec_blob_test: SmallVecBlobTest,
}

#[cfg(feature = "smallvec-types")]
sqlite_test!(smallvec_roundtrip_and_storage, SmallVecBlobSchema, {
    let table = schema.smallvec_blob_test;

    let mut payload = SmallVec::<[u8; 16]>::new();
    payload.extend_from_slice(&[1, 2, 3, 4, 5, 6]);
    let row = InsertSmallVecBlobTest::new(payload.clone(), "smallvec note");
    drizzle_exec!(db.insert(table).values([row]) => execute);

    let out: Vec<SelectSmallVecBlobTest> = drizzle_exec!(
        db.select((table.id, table.payload, table.note))
            .from(table)
            .r#where(eq(table.id, 1))
            => all
    );

    assert_eq!(out.len(), 1);
    assert_eq!(out[0].payload.as_slice(), payload.as_slice());
    assert_eq!(out[0].note, "smallvec note");

    #[derive(SQLiteFromRow, Debug)]
    struct Ty(String);
    let ty: Ty = drizzle_exec!(
        db.select(r#typeof(table.payload).alias("payload_type"))
            .from(table)
            .r#where(eq(table.id, 1))
            => get
    );

    assert_eq!(ty.0, "blob");
});

#![cfg(all(
    any(feature = "rusqlite", feature = "turso", feature = "libsql"),
    any(feature = "compact-str", feature = "bytes", feature = "smallvec-types")
))]

//! SQLite storage classes for the optional wrapper types.
//!
//! Round trips run in `crate::common::wrappers`; this file only checks the
//! storage class each wrapper lands in, which is a SQLite affinity question.

use drizzle::core::expr::{eq, typeof_};
use drizzle::sqlite::prelude::*;

#[cfg(feature = "compact-str")]
#[SQLiteTable(NAME = "compact_string_storage")]
struct CompactStringStorage {
    #[column(PRIMARY)]
    id: i32,
    name: compact_str::CompactString,
}

#[cfg(feature = "compact-str")]
#[derive(SQLiteSchema)]
struct CompactStringSchema {
    rows: CompactStringStorage,
}

#[cfg(feature = "compact-str")]
#[drizzle::test]
fn compact_strings_are_stored_as_text(db: &mut TestDb<CompactStringSchema>) {
    let CompactStringSchema { rows } = schema;
    db.insert(rows)
        .value(
            InsertCompactStringStorage::new(compact_str::CompactString::new("compact")).with_id(1),
        )
        .execute();

    let class: String = db
        .select(typeof_(rows.name))
        .from(rows)
        .r#where(eq(rows.id, 1))
        .get();
    assert_eq!(class, "text");
}

#[cfg(feature = "bytes")]
#[SQLiteTable(NAME = "bytes_storage")]
struct BytesStorage {
    #[column(PRIMARY)]
    id: i32,
    payload: bytes::Bytes,
    mutable_payload: bytes::BytesMut,
}

#[cfg(feature = "bytes")]
#[derive(SQLiteSchema)]
struct BytesSchema {
    rows: BytesStorage,
}

#[cfg(feature = "bytes")]
#[drizzle::test]
fn bytes_are_stored_as_blobs(db: &mut TestDb<BytesSchema>) {
    let BytesSchema { rows } = schema;
    db.insert(rows)
        .value(
            InsertBytesStorage::new(
                bytes::Bytes::from_static(b"hello"),
                bytes::BytesMut::from(&b"world"[..]),
            )
            .with_id(1),
        )
        .execute();

    let classes: (String, String) = db
        .select((typeof_(rows.payload), typeof_(rows.mutable_payload)))
        .from(rows)
        .r#where(eq(rows.id, 1))
        .get();
    assert_eq!(classes, ("blob".to_string(), "blob".to_string()));
}

#[cfg(feature = "smallvec-types")]
#[SQLiteTable(NAME = "smallvec_storage")]
struct SmallVecStorage {
    #[column(PRIMARY)]
    id: i32,
    payload: smallvec::SmallVec<[u8; 16]>,
}

#[cfg(feature = "smallvec-types")]
#[derive(SQLiteSchema)]
struct SmallVecSchema {
    rows: SmallVecStorage,
}

#[cfg(feature = "smallvec-types")]
#[drizzle::test]
fn small_vectors_are_stored_as_blobs(db: &mut TestDb<SmallVecSchema>) {
    let SmallVecSchema { rows } = schema;
    let mut payload = smallvec::SmallVec::<[u8; 16]>::new();
    payload.extend_from_slice(&[1, 2, 3]);
    db.insert(rows)
        .value(InsertSmallVecStorage::new(payload).with_id(1))
        .execute();

    let class: String = db
        .select(typeof_(rows.payload))
        .from(rows)
        .r#where(eq(rows.id, 1))
        .get();
    assert_eq!(class, "blob");
}

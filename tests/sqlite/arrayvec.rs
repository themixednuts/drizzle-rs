#![cfg(all(
    any(feature = "rusqlite", feature = "turso", feature = "libsql"),
    feature = "arrayvec"
))]

//! SQLite storage classes for bounded `arrayvec` columns.
//!
//! Round trips run in `crate::common::arrayvec`; this file only checks that
//! `ArrayString` lands in TEXT and `ArrayVec<u8, N>` in BLOB.

use arrayvec::{ArrayString, ArrayVec};
use drizzle::core::expr::{eq, typeof_};
use drizzle::sqlite::prelude::*;

#[SQLiteTable(NAME = "arrayvec_storage")]
struct ArrayVecStorage {
    #[column(PRIMARY)]
    id: i32,
    name: ArrayString<16>,
    data: ArrayVec<u8, 32>,
}

#[derive(SQLiteSchema)]
struct ArrayVecSchema {
    rows: ArrayVecStorage,
}

#[drizzle::test]
fn bounded_columns_use_text_and_blob_storage(db: &mut TestDb<ArrayVecSchema>) {
    let ArrayVecSchema { rows } = schema;
    let mut data = ArrayVec::<u8, 32>::new();
    data.extend([1, 2, 3]);
    db.insert(rows)
        .value(
            InsertArrayVecStorage::new(ArrayString::<16>::from("Hello").unwrap(), data).with_id(1),
        )
        .execute();

    let classes: (String, String) = db
        .select((typeof_(rows.name), typeof_(rows.data)))
        .from(rows)
        .r#where(eq(rows.id, 1))
        .get();
    assert_eq!(classes, ("text".to_string(), "blob".to_string()));
}

#![cfg(all(
    any(feature = "rusqlite", feature = "turso", feature = "libsql"),
    feature = "arrayvec"
))]

use arrayvec::{ArrayString, ArrayVec};
use drizzle::prelude::*;
use drizzle_macros::sqlite_test;

// Test table with ArrayString as TEXT column
#[SQLiteTable(name = "arraystring_test")]
struct ArrayStringTest {
    #[integer(primary)]
    id: i32,
    #[text] // ArrayString<16> stored as TEXT
    name: ArrayString<16>,
    #[text]
    description: String,
}

// Test table with ArrayVec<u8, N> as BLOB column
#[SQLiteTable(name = "arrayvec_blob_test")]
struct ArrayVecBlobTest {
    #[integer(primary)]
    id: i32,
    #[blob] // ArrayVec<u8, 32> stored as BLOB
    data: ArrayVec<u8, 32>,
    #[text]
    label: String,
}

// Test table with mixed arrayvec types
#[SQLiteTable(name = "mixed_arrayvec_test")]
struct MixedArrayVecTest {
    #[integer(primary)]
    id: i32,
    #[text]
    short_name: ArrayString<8>,
    #[text]
    long_name: ArrayString<64>,
    #[blob]
    small_data: ArrayVec<u8, 16>,
    #[blob]
    large_data: ArrayVec<u8, 128>,
}

#[derive(SQLiteSchema)]
struct ArrayStringSchema {
    arraystring_test: ArrayStringTest,
}

#[derive(SQLiteSchema)]
struct ArrayVecBlobSchema {
    arrayvec_blob_test: ArrayVecBlobTest,
}

#[derive(SQLiteSchema)]
struct MixedArrayVecSchema {
    mixed_arrayvec_test: MixedArrayVecTest,
}

sqlite_test!(test_arraystring_text_storage, ArrayStringSchema, {
    let table = schema.arraystring_test;

    // Create ArrayString (within capacity)
    let name = ArrayString::<16>::from("Hello").unwrap();
    let data = InsertArrayStringTest::new(name.clone(), "test description");

    // Insert data
    drizzle_exec!(db.insert(table).values([data]).execute());

    // Query back the data
    let results: Vec<SelectArrayStringTest> = drizzle_exec!(
        db.select((table.id, table.name, table.description))
            .from(table)
            .r#where(eq(table.description, "test description"))
            .all()
    );

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].name.as_str(), "Hello");
    assert_eq!(results[0].description, "test description");

    #[derive(SQLiteFromRow, Debug)]
    struct ReturnResult(String);
    // Verify it's stored as TEXT in the database
    let result: ReturnResult = drizzle_exec!(
        db.select(r#typeof(table.name).alias("name_type"))
            .from(table)
            .r#where(eq(table.id, 1))
            .get()
    );

    assert_eq!(result.0, "text");
});

sqlite_test!(test_arrayvec_blob_storage, ArrayVecBlobSchema, {
    let table = schema.arrayvec_blob_test;

    // Create ArrayVec with some bytes
    let mut data_vec = ArrayVec::<u8, 32>::new();
    data_vec.extend([1, 2, 3, 4, 5].iter().copied());

    let data = InsertArrayVecBlobTest::new(data_vec.clone(), "blob test");

    // Insert data
    drizzle_exec!(db.insert(table).values([data]).execute());

    // Query back the data
    let results: Vec<SelectArrayVecBlobTest> = drizzle_exec!(
        db.select((table.id, table.data, table.label))
            .from(table)
            .r#where(eq(table.label, "blob test"))
            .all()
    );

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].data.as_slice(), &[1, 2, 3, 4, 5]);
    assert_eq!(results[0].label, "blob test");

    #[derive(SQLiteFromRow, Debug)]
    struct ReturnResult(String);
    // Verify it's stored as BLOB in the database
    let result: ReturnResult = drizzle_exec!(
        db.select(r#typeof(table.data).alias("data_type"))
            .from(table)
            .r#where(eq(table.id, 1))
            .get()
    );

    assert_eq!(result.0, "blob");
});

sqlite_test!(test_arraystring_roundtrip, ArrayStringSchema, {
    let table = schema.arraystring_test;

    // Test with various string lengths (within capacity)
    let test_strings = vec!["A", "Hello", "Test", "1234567890123"];

    for (idx, test_str) in test_strings.iter().enumerate() {
        let name = ArrayString::<16>::from(test_str).unwrap();
        let desc = format!("test_{}", idx);
        let data = InsertArrayStringTest::new(name, &desc);

        drizzle_exec!(db.insert(table).values([data]).execute());
    }

    // Query all back
    let results: Vec<SelectArrayStringTest> = drizzle_exec!(
        db.select((table.id, table.name, table.description))
            .from(table)
            .all()
    );

    assert_eq!(results.len(), test_strings.len());
    for (idx, result) in results.iter().enumerate() {
        assert_eq!(result.name.as_str(), test_strings[idx]);
    }
});

sqlite_test!(test_arrayvec_roundtrip, ArrayVecBlobSchema, {
    let table = schema.arrayvec_blob_test;

    // Test with various byte arrays (within capacity)
    let test_data: Vec<Vec<u8>> = vec![
        vec![],            // Empty
        vec![0],           // Single byte
        vec![1, 2, 3],     // Few bytes
        vec![0; 20],       // 20 zeros
        (0..32).collect(), // Full capacity
    ];

    for (idx, test_bytes) in test_data.iter().enumerate() {
        let mut data_vec = ArrayVec::<u8, 32>::new();
        data_vec.extend(test_bytes.iter().copied());
        let label = format!("test_{}", idx);
        let data = InsertArrayVecBlobTest::new(data_vec, &label);

        drizzle_exec!(db.insert(table).values([data]).execute());
    }

    // Query all back
    let results: Vec<SelectArrayVecBlobTest> = drizzle_exec!(
        db.select((table.id, table.data, table.label))
            .from(table)
            .all()
    );

    assert_eq!(results.len(), test_data.len());
    for (idx, result) in results.iter().enumerate() {
        assert_eq!(result.data.as_slice(), test_data[idx].as_slice());
    }
});

sqlite_test!(test_mixed_arrayvec_types, MixedArrayVecSchema, {
    let table = schema.mixed_arrayvec_test;

    // Create data with different capacities
    let short_name = ArrayString::<8>::from("Short").unwrap();
    let long_name =
        ArrayString::<64>::from("This is a much longer name that fits in 64 chars").unwrap();

    let mut small_data = ArrayVec::<u8, 16>::new();
    small_data.extend([1, 2, 3, 4, 5].iter().copied());

    let mut large_data = ArrayVec::<u8, 128>::new();
    large_data.extend((0..100).map(|i| (i % 256) as u8));

    let data = InsertMixedArrayVecTest::new(
        short_name.clone(),
        long_name.clone(),
        small_data.clone(),
        large_data.clone(),
    );

    // Insert data
    drizzle_exec!(db.insert(table).values([data]).execute());

    // Query back the data
    let results: Vec<SelectMixedArrayVecTest> = drizzle_exec!(
        db.select((
            table.id,
            table.short_name,
            table.long_name,
            table.small_data,
            table.large_data
        ))
        .from(table)
        .all()
    );

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].short_name.as_str(), "Short");
    assert_eq!(
        results[0].long_name.as_str(),
        "This is a much longer name that fits in 64 chars"
    );
    assert_eq!(results[0].small_data.as_slice(), &[1, 2, 3, 4, 5]);
    assert_eq!(results[0].large_data.len(), 100);
    for i in 0..100 {
        assert_eq!(results[0].large_data[i], (i % 256) as u8);
    }
});

sqlite_test!(test_arraystring_empty, ArrayStringSchema, {
    let table = schema.arraystring_test;

    // Test with empty string
    let name = ArrayString::<16>::new();
    let data = InsertArrayStringTest::new(name, "empty test");

    drizzle_exec!(db.insert(table).values([data]).execute());

    let results: Vec<SelectArrayStringTest> = drizzle_exec!(
        db.select((table.id, table.name, table.description))
            .from(table)
            .all()
    );

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].name.as_str(), "");
});

sqlite_test!(test_arrayvec_empty, ArrayVecBlobSchema, {
    let table = schema.arrayvec_blob_test;

    // Test with empty ArrayVec
    let data_vec = ArrayVec::<u8, 32>::new();
    let data = InsertArrayVecBlobTest::new(data_vec, "empty blob");

    drizzle_exec!(db.insert(table).values([data]).execute());

    let results: Vec<SelectArrayVecBlobTest> = drizzle_exec!(
        db.select((table.id, table.data, table.label))
            .from(table)
            .all()
    );

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].data.len(), 0);
});

sqlite_test!(test_arraystring_max_capacity, ArrayStringSchema, {
    let table = schema.arraystring_test;

    // Test with string at maximum capacity (16 chars)
    let name = ArrayString::<16>::from("1234567890123456").unwrap();
    let data = InsertArrayStringTest::new(name, "max capacity");

    drizzle_exec!(db.insert(table).values([data]).execute());

    let results: Vec<SelectArrayStringTest> = drizzle_exec!(
        db.select((table.id, table.name, table.description))
            .from(table)
            .all()
    );

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].name.as_str(), "1234567890123456");
    assert_eq!(results[0].name.len(), 16);
});

sqlite_test!(test_arrayvec_max_capacity, ArrayVecBlobSchema, {
    let table = schema.arrayvec_blob_test;

    // Test with ArrayVec at maximum capacity (32 bytes)
    let mut data_vec = ArrayVec::<u8, 32>::new();
    for i in 0..32 {
        data_vec.push(i as u8);
    }
    let data = InsertArrayVecBlobTest::new(data_vec, "max capacity");

    drizzle_exec!(db.insert(table).values([data]).execute());

    let results: Vec<SelectArrayVecBlobTest> = drizzle_exec!(
        db.select((table.id, table.data, table.label))
            .from(table)
            .all()
    );

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].data.len(), 32);
    for i in 0..32 {
        assert_eq!(results[0].data[i], i as u8);
    }
});

sqlite_test!(test_arrayvec_update, ArrayVecBlobSchema, {
    let table = schema.arrayvec_blob_test;

    // Insert initial data
    let mut initial_data = ArrayVec::<u8, 32>::new();
    initial_data.extend([1, 2, 3].iter().copied());
    let data = InsertArrayVecBlobTest::new(initial_data, "update test");

    drizzle_exec!(db.insert(table).values([data]).execute());

    // Update with new data
    let mut updated_data = ArrayVec::<u8, 32>::new();
    updated_data.extend([10, 20, 30, 40].iter().copied());

    drizzle_exec!(
        db.update(table)
            .set(UpdateArrayVecBlobTest::default().with_data(updated_data.clone()))
            .r#where(eq(table.id, 1))
            .execute()
    );

    // Query back
    let results: Vec<SelectArrayVecBlobTest> = drizzle_exec!(
        db.select((table.id, table.data, table.label))
            .from(table)
            .all()
    );

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].data.as_slice(), &[10, 20, 30, 40]);
});

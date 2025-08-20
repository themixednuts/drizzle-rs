#![cfg(all(
    any(feature = "rusqlite", feature = "turso", feature = "libsql"),
    feature = "uuid"
))]

use drizzle_rs::prelude::*;
use uuid::Uuid;

mod common;

// Test table with UUID as TEXT column
#[SQLiteTable(name = "uuid_text_test")]
struct UuidTextTest {
    #[integer(primary)]
    id: i32,
    #[text] // UUID stored as TEXT
    uuid_field: Uuid,
    #[text]
    name: String,
}

// Test table with UUID as BLOB column
#[SQLiteTable(name = "uuid_blob_test")]
struct UuidBlobTest {
    #[integer(primary)]
    id: i32,
    #[blob] // UUID stored as BLOB
    uuid_field: Uuid,
    #[text]
    name: String,
}

// Test table with UUID TEXT column using default_fn
#[SQLiteTable(name = "uuid_text_default")]
struct UuidTextDefault {
    #[integer(primary)]
    id: i32,
    #[text(default_fn = Uuid::new_v4)] // UUID stored as TEXT with auto-generation
    uuid_field: Uuid,
    #[text]
    name: String,
}

// Test table with UUID BLOB column using default_fn
#[SQLiteTable(name = "uuid_blob_default")]
struct UuidBlobDefault {
    #[integer(primary)]
    id: i32,
    #[blob(default_fn = Uuid::new_v4)] // UUID stored as BLOB with auto-generation
    uuid_field: Uuid,
    #[text]
    name: String,
}

#[derive(SQLSchema)]
struct UuidTextSchema {
    uuid_text_test: UuidTextTest,
}

#[derive(SQLSchema)]
struct UuidBlobSchema {
    uuid_blob_test: UuidBlobTest,
}

#[derive(SQLSchema)]
struct UuidTextDefaultSchema {
    uuid_text_default: UuidTextDefault,
}

#[derive(SQLSchema)]
struct UuidBlobDefaultSchema {
    uuid_blob_default: UuidBlobDefault,
}

#[tokio::test]
async fn test_uuid_text_storage() {
    let conn = setup_test_db!();
    let (db, schema) = drizzle!(conn, UuidTextSchema);
    drizzle_exec!(db.create());
    let table = schema.uuid_text_test;

    // Generate test UUID
    let test_uuid = Uuid::new_v4();
    let data = InsertUuidTextTest::new(test_uuid, "text storage test");

    // Insert data
    drizzle_exec!(db.insert(table).values([data]).execute());

    // Query back the data using Drizzle
    let results: Vec<SelectUuidTextTest> = drizzle_exec!(
        db.select((table.id, table.uuid_field, table.name))
            .from(table)
            .r#where(eq(table.name, "text storage test"))
            .all()
    );

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].uuid_field, test_uuid);
    assert_eq!(results[0].name, "text storage test");

    // Verify it's stored as TEXT in the database
    let query = "SELECT typeof(uuid_field) as uuid_type FROM uuid_text_test WHERE id = 1";
    let mut stmt = prepare_stmt!(db.conn(), query);

    query_row!(stmt, db_params!(), |row| {
        let storage_type = row_get!(row, 0, String);
        assert_eq!(storage_type, "text");
    });
}

#[tokio::test]
async fn test_uuid_blob_storage() {
    let conn = setup_test_db!();
    let (db, schema) = drizzle!(conn, UuidBlobSchema);
    drizzle_exec!(db.create());
    let table = schema.uuid_blob_test;

    // Generate test UUID
    let test_uuid = Uuid::new_v4();
    let data = InsertUuidBlobTest::new(test_uuid, "blob storage test");

    // Insert data
    drizzle_exec!(db.insert(table).values([data]).execute());

    // Query back the data using Drizzle
    let results: Vec<SelectUuidBlobTest> = drizzle_exec!(
        db.select((table.id, table.uuid_field, table.name))
            .from(table)
            .r#where(eq(table.name, "blob storage test"))
            .all()
    );

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].uuid_field, test_uuid);
    assert_eq!(results[0].name, "blob storage test");

    // Verify it's stored as BLOB in the database
    let query = "SELECT typeof(uuid_field) as uuid_type FROM uuid_blob_test WHERE id = 1";
    let mut stmt = prepare_stmt!(db.conn(), query);

    query_row!(stmt, db_params!(), |row| {
        let storage_type = row_get!(row, 0, String);
        assert_eq!(storage_type, "blob");
    });
}

#[tokio::test]
async fn test_uuid_text_vs_blob_roundtrip() {
    // Test both tables with the same UUID
    let test_uuid = Uuid::new_v4();

    // Test TEXT storage
    {
        let conn = setup_test_db!();
        let (db, schema) = drizzle!(conn, UuidTextSchema);
        drizzle_exec!(db.create());
        let table = schema.uuid_text_test;

        let data = InsertUuidTextTest::new(test_uuid, "roundtrip test");
        drizzle_exec!(db.insert(table).values([data]).execute());

        let results: Vec<SelectUuidTextTest> = drizzle_exec!(
            db.select((table.id, table.uuid_field, table.name))
                .from(table)
                .all()
        );
        assert_eq!(results[0].uuid_field, test_uuid);
    }

    // Test BLOB storage
    {
        let conn = setup_test_db!();
        let (db, schema) = drizzle!(conn, UuidBlobSchema);
        drizzle_exec!(db.create());
        let table = schema.uuid_blob_test;

        let data = InsertUuidBlobTest::new(test_uuid, "roundtrip test");
        drizzle_exec!(db.insert(table).values([data]).execute());

        let results: Vec<SelectUuidBlobTest> = drizzle_exec!(
            db.select((table.id, table.uuid_field, table.name))
                .from(table)
                .all()
        );
        assert_eq!(results[0].uuid_field, test_uuid);
    }
}

#[tokio::test]
async fn test_uuid_text_default_fn() {
    let conn = setup_test_db!();
    let (db, schema) = drizzle!(conn, UuidTextDefaultSchema);
    drizzle_exec!(db.create());
    let table = schema.uuid_text_default;

    // Insert without specifying UUID - should use default_fn
    let data = InsertUuidTextDefault::new("auto-generated text uuid");
    drizzle_exec!(db.insert(table).values([data]).execute());

    // Query back the data
    let results: Vec<SelectUuidTextDefault> = drizzle_exec!(
        db.select((table.id, table.uuid_field, table.name))
            .from(table)
            .all()
    );

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].name, "auto-generated text uuid");

    // Verify UUID was generated (not nil)
    assert_ne!(results[0].uuid_field, Uuid::nil());

    // Verify it's stored as TEXT
    let query = "SELECT typeof(uuid_field) as uuid_type FROM uuid_text_default WHERE id = 1";
    let mut stmt = prepare_stmt!(db.conn(), query);

    query_row!(stmt, db_params!(), |row| {
        let storage_type = row_get!(row, 0, String);
        assert_eq!(storage_type, "text");
    });
}

#[tokio::test]
async fn test_uuid_blob_default_fn() {
    let conn = setup_test_db!();
    let (db, schema) = drizzle!(conn, UuidBlobDefaultSchema);
    drizzle_exec!(db.create());
    let table = schema.uuid_blob_default;

    // Insert without specifying UUID - should use default_fn
    let data = InsertUuidBlobDefault::new("auto-generated blob uuid");
    drizzle_exec!(db.insert(table).values([data]).execute());

    // Query back the data
    let results: Vec<SelectUuidBlobDefault> = drizzle_exec!(
        db.select((table.id, table.uuid_field, table.name))
            .from(table)
            .all()
    );

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].name, "auto-generated blob uuid");

    // Verify UUID was generated (not nil)
    assert_ne!(results[0].uuid_field, Uuid::nil());

    // Verify it's stored as BLOB
    let query = "SELECT typeof(uuid_field) as uuid_type FROM uuid_blob_default WHERE id = 1";
    let mut stmt = prepare_stmt!(db.conn(), query);

    query_row!(stmt, db_params!(), |row| {
        let storage_type = row_get!(row, 0, String);
        assert_eq!(storage_type, "blob");
    });
}

#[tokio::test]
async fn test_uuid_default_fn_uniqueness() {
    // Test TEXT default_fn generates unique UUIDs
    {
        let conn = setup_test_db!();
        let (db, schema) = drizzle!(conn, UuidTextDefaultSchema);
        drizzle_exec!(db.create());
        let table = schema.uuid_text_default;

        let data1 = InsertUuidTextDefault::new("first");
        let data2 = InsertUuidTextDefault::new("second");

        drizzle_exec!(db.insert(table).values([data1, data2]).execute());

        let results: Vec<SelectUuidTextDefault> = drizzle_exec!(
            db.select((table.id, table.uuid_field, table.name))
                .from(table)
                .all()
        );
        assert_eq!(results.len(), 2);

        // Verify UUIDs are different
        assert_ne!(results[0].uuid_field, results[1].uuid_field);
        assert_ne!(results[0].uuid_field, Uuid::nil());
        assert_ne!(results[1].uuid_field, Uuid::nil());
    }

    // Test BLOB default_fn generates unique UUIDs
    {
        let conn = setup_test_db!();
        let (db, schema) = drizzle!(conn, UuidBlobDefaultSchema);
        drizzle_exec!(db.create());
        let table = schema.uuid_blob_default;

        let data1 = InsertUuidBlobDefault::new("first");
        let data2 = InsertUuidBlobDefault::new("second");

        drizzle_exec!(db.insert(table).values([data1, data2]).execute());

        let results: Vec<SelectUuidBlobDefault> = drizzle_exec!(
            db.select((table.id, table.uuid_field, table.name))
                .from(table)
                .all()
        );
        assert_eq!(results.len(), 2);

        // Verify UUIDs are different
        assert_ne!(results[0].uuid_field, results[1].uuid_field);
        assert_ne!(results[0].uuid_field, Uuid::nil());
        assert_ne!(results[1].uuid_field, Uuid::nil());
    }
}

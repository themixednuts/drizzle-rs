use drizzle_rs::prelude::*;
#[cfg(feature = "libsql")]
use libsql::{Builder, Connection, Value};
#[cfg(feature = "rusqlite")]
use rusqlite::Connection;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
#[cfg(feature = "turso")]
use turso::{Builder, Connection, Value};

// Test all SQLite column types
#[SQLiteTable(name = "all_types")]
struct AllTypes {
    #[integer(primary)]
    id: i32,
    #[text]
    text_field: String,
    #[integer]
    int_field: i32,
    #[real]
    real_field: f64,
    #[blob]
    blob_field: Vec<u8>,
    #[integer]
    bool_field: bool,
}

// Test primary key variations
#[SQLiteTable(name = "pk_variations")]
struct PrimaryKeyVariations {
    #[integer(primary, autoincrement)]
    auto_id: i32,
    #[text]
    name: String,
}

#[SQLiteTable(name = "manual_pk")]
struct ManualPrimaryKey {
    #[text(primary)]
    manual_id: String,
    #[text]
    description: String,
}

// Test unique constraints
#[SQLiteTable(name = "unique_fields")]
struct UniqueFields {
    #[integer(primary, autoincrement)]
    id: i32,
    #[text(unique)]
    email: String,
    #[text(unique)]
    username: String,
    #[text]
    display_name: Option<String>,
}

// Test default values - compile time literals
#[SQLiteTable(name = "compile_defaults")]
struct CompileTimeDefaults {
    #[integer(primary, autoincrement)]
    id: i32,
    #[text(default = "default_name")]
    name: String,
    #[integer(default = 42)]
    answer: i32,
    #[real(default = 3.14)]
    pi: f64,
    #[boolean(default = true)]
    active: bool,
    #[text(default = "pending")]
    status: String,
}

// Test default values - runtime functions
#[SQLiteTable(name = "runtime_defaults")]
struct RuntimeDefaults {
    #[integer(primary, autoincrement)]
    id: i32,
    #[text(default_fn = String::new)]
    empty_text: String,
    #[integer(default_fn = || 100)]
    computed_int: i32,
    #[text]
    name: String,
}

// Test enums with different storage types
#[derive(SQLiteEnum, Default, Clone, PartialEq, Debug, Copy)]
enum Priority {
    Low = 1,
    #[default]
    Medium = 2,
    High = 3,
}

#[derive(SQLiteEnum, Default, Clone, PartialEq, Debug, Copy)]
enum TaskStatus {
    #[default]
    Todo,
    InProgress,
    Done,
}

#[SQLiteTable(name = "enum_fields")]
struct EnumFields {
    #[integer(primary, autoincrement)]
    id: i32,
    #[integer(enum)]
    priority: Priority,
    #[text(enum)]
    status: TaskStatus,
    #[text]
    description: String,
}

// Test table with tuple/struct enum fields
#[SQLiteTable(name = "complex_enum_fields")]
struct ComplexEnumFields {
    #[integer(primary, autoincrement)]
    id: i32,
    #[text]
    notes: String,
}

// Test JSON fields with serde feature
#[cfg(feature = "serde")]
#[derive(Deserialize, Serialize, Default, Debug, Clone, PartialEq)]
struct JsonData {
    value: i32,
    message: String,
}

#[cfg(feature = "serde")]
#[SQLiteTable(name = "json_fields")]
struct JsonFields {
    #[integer(primary, autoincrement)]
    id: i32,
    #[text(json)]
    text_json: Option<JsonData>,
    #[text]
    regular_text: String,
}

// Test UUID fields
#[cfg(feature = "uuid")]
#[SQLiteTable(name = "uuid_fields")]
struct UuidFields {
    #[blob(primary, default_fn = uuid::Uuid::new_v4)]
    id: uuid::Uuid,
    #[text]
    name: String,
    #[blob]
    other_uuid: Option<uuid::Uuid>,
}

// Test nullable vs non-nullable fields
#[SQLiteTable(name = "nullable_test")]
struct NullableTest {
    #[integer(primary, autoincrement)]
    id: i32,
    // Required fields (non-nullable)
    #[text]
    required_text: String,
    #[integer]
    required_int: i32,
    #[boolean]
    required_bool: bool,

    // Optional fields (nullable)
    #[text]
    optional_text: Option<String>,
    #[integer]
    optional_int: Option<i32>,
    #[real]
    optional_real: Option<f64>,
    #[blob]
    optional_blob: Option<Vec<u8>>,
    #[boolean]
    optional_bool: Option<bool>,
}

#[cfg(feature = "rusqlite")]
// Helper functions for setup
fn setup_test_db() -> Connection {
    let conn = Connection::open_in_memory().expect("Failed to create in-memory database");

    // Create all test tables
    conn.execute(AllTypes::SQL.sql().as_str(), []).unwrap();
    conn.execute(PrimaryKeyVariations::SQL.sql().as_str(), [])
        .unwrap();
    conn.execute(ManualPrimaryKey::SQL.sql().as_str(), [])
        .unwrap();
    conn.execute(UniqueFields::SQL.sql().as_str(), []).unwrap();
    conn.execute(CompileTimeDefaults::SQL.sql().as_str(), [])
        .unwrap();
    conn.execute(RuntimeDefaults::SQL.sql().as_str(), [])
        .unwrap();
    conn.execute(EnumFields::SQL.sql().as_str(), []).unwrap();
    conn.execute(ComplexEnumFields::SQL.sql().as_str(), [])
        .unwrap();

    #[cfg(feature = "serde")]
    conn.execute(JsonFields::SQL.sql().as_str(), []).unwrap();

    #[cfg(feature = "uuid")]
    conn.execute(UuidFields::SQL.sql().as_str(), []).unwrap();

    conn.execute(NullableTest::SQL.sql().as_str(), []).unwrap();

    conn
}

#[cfg(any(feature = "turso", feature = "libsql"))]
// Helper functions for setup
async fn setup_test_db() -> Connection {
    let db = Builder::new_local(":memory:")
        .build()
        .await
        .expect("build db");
    let conn = db.connect().expect("connect to db");

    // Create all test tables
    conn.execute(AllTypes::SQL.sql().as_str(), ())
        .await
        .unwrap();
    conn.execute(PrimaryKeyVariations::SQL.sql().as_str(), ())
        .await
        .unwrap();
    conn.execute(ManualPrimaryKey::SQL.sql().as_str(), ())
        .await
        .unwrap();
    conn.execute(UniqueFields::SQL.sql().as_str(), ())
        .await
        .unwrap();
    conn.execute(CompileTimeDefaults::SQL.sql().as_str(), ())
        .await
        .unwrap();
    conn.execute(RuntimeDefaults::SQL.sql().as_str(), ())
        .await
        .unwrap();
    conn.execute(EnumFields::SQL.sql().as_str(), ())
        .await
        .unwrap();
    conn.execute(ComplexEnumFields::SQL.sql().as_str(), ())
        .await
        .unwrap();

    #[cfg(feature = "serde")]
    conn.execute(JsonFields::SQL.sql().as_str(), ())
        .await
        .unwrap();

    #[cfg(feature = "uuid")]
    conn.execute(UuidFields::SQL.sql().as_str(), ())
        .await
        .unwrap();

    conn.execute(NullableTest::SQL.sql().as_str(), ())
        .await
        .unwrap();

    conn
}

#[tokio::test]
async fn test_all_column_types() {
    #[cfg(feature = "rusqlite")]
    let conn = setup_test_db();
    #[cfg(any(feature = "turso", feature = "libsql"))]
    let conn = setup_test_db().await;

    let (db, all_types) = drizzle!(conn, [AllTypes]);

    // Test insertion with all column types
    let test_data = InsertAllTypes::default()
        .with_text_field("test text".to_string())
        .with_int_field(123)
        .with_real_field(45.67)
        .with_blob_field(vec![1, 2, 3, 4, 5])
        .with_bool_field(true);

    #[cfg(feature = "rusqlite")]
    let result = db.insert(all_types).values([test_data]).execute().unwrap();
    #[cfg(any(feature = "turso", feature = "libsql"))]
    let result = db
        .insert(all_types)
        .values([test_data])
        .execute()
        .await
        .unwrap();

    assert_eq!(result, 1);

    // Verify the data was stored correctly
    let query = "SELECT * FROM all_types WHERE id = 1";
    #[cfg(feature = "rusqlite")]
    let mut stmt = db.conn().prepare(query).unwrap();
    #[cfg(any(feature = "turso", feature = "libsql"))]
    let mut stmt = db.conn().prepare(query).await.unwrap();

    #[cfg(feature = "rusqlite")]
    let row = stmt
        .query_row([], |row| {
            Ok((
                row.get::<_, String>("text_field")?,
                row.get::<_, i32>("int_field")?,
                row.get::<_, f64>("real_field")?,
                row.get::<_, Vec<u8>>("blob_field")?,
                row.get::<_, bool>("bool_field")?,
            ))
        })
        .unwrap();

    #[cfg(any(feature = "turso", feature = "libsql"))]
    let mut rows = stmt.query(()).await.unwrap();

    #[cfg(feature = "rusqlite")]
    {
        assert_eq!(row.0, "test text");
        assert_eq!(row.1, 123);
        assert_eq!(row.2, 45.67);
        assert_eq!(row.3, vec![1, 2, 3, 4, 5]);
        assert_eq!(row.4, true);
    }

    #[cfg(any(feature = "turso", feature = "libsql"))]
    {
        while let Some(row) = rows.next().await.unwrap() {
            assert_eq!(row.get_value(1).unwrap().as_text().unwrap(), "test text");
            assert_eq!(
                row.get_value(2).unwrap().as_integer().cloned().unwrap(),
                123
            );
            assert_eq!(row.get_value(3).unwrap().as_real().cloned().unwrap(), 45.67);
            assert_eq!(
                row.get_value(4).unwrap().as_blob().cloned().unwrap(),
                vec![1, 2, 3, 4, 5]
            );
            assert_eq!(
                row.get_value(5)
                    .unwrap()
                    .as_integer()
                    .map(|&v| v != 0)
                    .unwrap(),
                true
            );
        }
    }
}

#[tokio::test]
async fn test_primary_key_autoincrement() {
    #[cfg(feature = "rusqlite")]
    let conn = setup_test_db();
    #[cfg(any(feature = "turso", feature = "libsql"))]
    let conn = setup_test_db().await;

    let (db, pk_table) = drizzle!(conn, [PrimaryKeyVariations]);

    // Insert multiple records to test autoincrement
    let data1 = InsertPrimaryKeyVariations::default().with_name("first".to_string());
    let data2 = InsertPrimaryKeyVariations::default().with_name("second".to_string());

    #[cfg(feature = "rusqlite")]
    {
        db.insert(pk_table).values([data1]).execute().unwrap();
        db.insert(pk_table).values([data2]).execute().unwrap();
    }
    #[cfg(any(feature = "turso", feature = "libsql"))]
    {
        db.insert(pk_table).values([data1]).execute().await.unwrap();
        db.insert(pk_table).values([data2]).execute().await.unwrap();
    }

    // Verify autoincrement worked
    let query = "SELECT auto_id, name FROM pk_variations ORDER BY auto_id";
    #[cfg(feature = "rusqlite")]
    {
        let mut stmt = db.conn().prepare(query).unwrap();
        let rows: Result<Vec<_>, _> = stmt
            .query_map([], |row| {
                Ok((row.get::<_, i32>(0)?, row.get::<_, String>(1)?))
            })
            .unwrap()
            .collect();

        let rows = rows.unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0], (1, "first".to_string()));
        assert_eq!(rows[1], (2, "second".to_string()));
    }
    #[cfg(any(feature = "turso", feature = "libsql"))]
    {
        let mut stmt = db.conn().prepare(query).await.unwrap();
        let mut rows = stmt.query(()).await.unwrap();
        let mut results = Vec::new();

        while let Some(row) = rows.next().await.unwrap() {
            let auto_id = row.get_value(0).unwrap().as_integer().cloned().unwrap();
            let name = row.get_value(1).unwrap().as_text().unwrap().to_string();
            results.push((auto_id, name));
        }

        assert_eq!(results.len(), 2);
        assert_eq!(results[0], (1, "first".to_string()));
        assert_eq!(results[1], (2, "second".to_string()));
    }
}

#[tokio::test]
async fn test_manual_primary_key() {
    #[cfg(feature = "rusqlite")]
    let conn = setup_test_db();
    #[cfg(any(feature = "turso", feature = "libsql"))]
    let conn = setup_test_db().await;

    let (db, manual_pk) = drizzle!(conn, [ManualPrimaryKey]);

    let data = InsertManualPrimaryKey::default()
        .with_manual_id("custom_id_123".to_string())
        .with_description("Test description".to_string());

    #[cfg(feature = "rusqlite")]
    let result = db.insert(manual_pk).values([data]).execute().unwrap();
    #[cfg(any(feature = "turso", feature = "libsql"))]
    let result = db.insert(manual_pk).values([data]).execute().await.unwrap();

    assert_eq!(result, 1);

    // Verify the manual primary key
    let query = "SELECT manual_id, description FROM manual_pk WHERE manual_id = 'custom_id_123'";
    #[cfg(feature = "rusqlite")]
    {
        let mut stmt = db.conn().prepare(query).unwrap();
        let row = stmt
            .query_row([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })
            .unwrap();

        assert_eq!(row.0, "custom_id_123");
        assert_eq!(row.1, "Test description");
    }
    #[cfg(any(feature = "turso", feature = "libsql"))]
    {
        let mut stmt = db.conn().prepare(query).await.unwrap();
        let mut rows = stmt.query(()).await.unwrap();

        if let Some(row) = rows.next().await.unwrap() {
            let manual_id = row.get_value(0).unwrap().as_text().cloned().unwrap();
            let description = row.get_value(1).unwrap().as_text().cloned().unwrap();
            assert_eq!(manual_id, "custom_id_123");
            assert_eq!(description, "Test description");
        }
    }
}

#[tokio::test]
async fn test_unique_constraints() {
    #[cfg(feature = "rusqlite")]
    let conn = setup_test_db();
    #[cfg(any(feature = "turso", feature = "libsql"))]
    let conn = setup_test_db().await;

    let (db, unique_table) = drizzle!(conn, [UniqueFields]);

    // Insert first record
    let data1 = InsertUniqueFields::default()
        .with_email("test@example.com".to_string())
        .with_username("testuser".to_string())
        .with_display_name("Test User".to_string());

    #[cfg(feature = "rusqlite")]
    let result1 = db.insert(unique_table).values([data1]).execute().unwrap();
    #[cfg(any(feature = "turso", feature = "libsql"))]
    let result1 = db
        .insert(unique_table)
        .values([data1])
        .execute()
        .await
        .unwrap();

    assert_eq!(result1, 1);

    // Try to insert duplicate email - should fail
    let data2 = InsertUniqueFields::default()
        .with_email("test@example.com".to_string()) // Duplicate email
        .with_username("different_user".to_string());

    #[cfg(feature = "rusqlite")]
    let result2 = db.insert(unique_table).values([data2]).execute();
    #[cfg(any(feature = "turso", feature = "libsql"))]
    let result2 = db.insert(unique_table).values([data2]).execute().await;

    assert!(result2.is_err()); // Should fail due to unique constraint
}

#[tokio::test]
async fn test_compile_time_defaults() {
    #[cfg(feature = "rusqlite")]
    let conn = setup_test_db();
    #[cfg(any(feature = "turso", feature = "libsql"))]
    let conn = setup_test_db().await;

    let (db, defaults_table) = drizzle!(conn, [CompileTimeDefaults]);

    // Insert with minimal data - defaults should be used
    let data = InsertCompileTimeDefaults::default();

    #[cfg(feature = "rusqlite")]
    let result = db.insert(defaults_table).values([data]).execute().unwrap();
    #[cfg(any(feature = "turso", feature = "libsql"))]
    let result = db
        .insert(defaults_table)
        .values([data])
        .execute()
        .await
        .unwrap();

    assert_eq!(result, 1);

    // Verify defaults were applied
    let query = "SELECT name, answer, pi, active, status FROM compile_defaults WHERE id = 1";
    #[cfg(feature = "rusqlite")]
    {
        let mut stmt = db.conn().prepare(query).unwrap();
        let row = stmt
            .query_row([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, i32>(1)?,
                    row.get::<_, f64>(2)?,
                    row.get::<_, bool>(3)?,
                    row.get::<_, String>(4)?,
                ))
            })
            .unwrap();

        assert_eq!(row.0, "default_name");
        assert_eq!(row.1, 42);
        assert_eq!(row.2, 3.14);
        assert_eq!(row.3, true);
        assert_eq!(row.4, "pending");
    }
    #[cfg(any(feature = "turso", feature = "libsql"))]
    {
        let mut stmt = db.conn().prepare(query).await.unwrap();
        let mut rows = stmt.query(()).await.unwrap();

        if let Some(row) = rows.next().await.unwrap() {
            let name = row.get_value(0).unwrap().as_text().cloned().unwrap();
            let answer = row.get_value(1).unwrap().as_integer().cloned().unwrap();
            let pi = row.get_value(2).unwrap().as_real().cloned().unwrap();
            let active = row
                .get_value(3)
                .unwrap()
                .as_integer()
                .map(|&v| v != 0)
                .unwrap();
            let status = row.get_value(4).unwrap().as_text().cloned().unwrap();

            assert_eq!(name, "default_name");
            assert_eq!(answer, 42);
            assert_eq!(pi, 3.14);
            assert_eq!(active, true);
            assert_eq!(status, "pending");
        }
    }
}

#[tokio::test]
async fn test_runtime_defaults() {
    #[cfg(feature = "rusqlite")]
    let conn = setup_test_db();
    #[cfg(any(feature = "turso", feature = "libsql"))]
    let conn = setup_test_db().await;

    let (db, runtime_table) = drizzle!(conn, [RuntimeDefaults]);

    // Insert with minimal data - runtime defaults should be used
    let data = InsertRuntimeDefaults::default().with_name("test".to_string());

    #[cfg(feature = "rusqlite")]
    let result = db.insert(runtime_table).values([data]).execute().unwrap();
    #[cfg(any(feature = "turso", feature = "libsql"))]
    let result = db
        .insert(runtime_table)
        .values([data])
        .execute()
        .await
        .unwrap();

    assert_eq!(result, 1);

    // Verify runtime defaults were applied
    let query = "SELECT empty_text, computed_int, name FROM runtime_defaults WHERE id = 1";
    #[cfg(feature = "rusqlite")]
    {
        let mut stmt = db.conn().prepare(query).unwrap();
        let row = stmt
            .query_row([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, i32>(1)?,
                    row.get::<_, String>(2)?,
                ))
            })
            .unwrap();

        assert_eq!(row.0, ""); // String::new() returns empty string
        assert_eq!(row.1, 100); // Closure returns 100
        assert_eq!(row.2, "test");
    }
    #[cfg(any(feature = "turso", feature = "libsql"))]
    {
        let mut stmt = db.conn().prepare(query).await.unwrap();
        let mut rows = stmt.query(()).await.unwrap();

        if let Some(row) = rows.next().await.unwrap() {
            let empty_text = row.get_value(0).unwrap().as_text().cloned().unwrap();
            let computed_int = row.get_value(1).unwrap().as_integer().cloned().unwrap();
            let name = row.get_value(2).unwrap().as_text().cloned().unwrap();

            assert_eq!(empty_text, ""); // String::new() returns empty string
            assert_eq!(computed_int, 100); // Closure returns 100
            assert_eq!(name, "test");
        }
    }
}

#[tokio::test]
async fn test_enum_storage_types() {
    #[cfg(feature = "rusqlite")]
    let conn = setup_test_db();
    #[cfg(any(feature = "turso", feature = "libsql"))]
    let conn = setup_test_db().await;

    let (db, enum_table) = drizzle!(conn, [EnumFields]);

    // Test different enum storage types
    let data = InsertEnumFields::default()
        .with_priority(Priority::High)
        .with_status(TaskStatus::InProgress)
        .with_description("Test task".to_string());

    #[cfg(feature = "rusqlite")]
    let result = db.insert(enum_table).values([data]).execute().unwrap();
    #[cfg(any(feature = "turso", feature = "libsql"))]
    let result = db
        .insert(enum_table)
        .values([data])
        .execute()
        .await
        .unwrap();

    assert_eq!(result, 1);

    // Verify enum storage
    let query = "SELECT priority, status, typeof(priority) as priority_type, typeof(status) as status_type FROM enum_fields WHERE id = 1";
    #[cfg(feature = "rusqlite")]
    {
        let mut stmt = db.conn().prepare(query).unwrap();
        let row = stmt
            .query_row([], |row| {
                Ok((
                    row.get::<_, i32>(0)?,    // priority as integer
                    row.get::<_, String>(1)?, // status as text
                    row.get::<_, String>(2)?, // priority type
                    row.get::<_, String>(3)?, // status type
                ))
            })
            .unwrap();

        assert_eq!(row.0, 3); // Priority::High = 3
        assert_eq!(row.1, "InProgress"); // TaskStatus::InProgress as text
        assert_eq!(row.2, "integer"); // integer(enum) stores as INTEGER
        assert_eq!(row.3, "text"); // text(enum) stores as TEXT
    }
    #[cfg(any(feature = "turso", feature = "libsql"))]
    {
        let mut stmt = db.conn().prepare(query).await.unwrap();
        let mut rows = stmt.query(()).await.unwrap();

        if let Some(row) = rows.next().await.unwrap() {
            let priority = row.get_value(0).unwrap().as_integer().cloned().unwrap();
            let status = row.get_value(1).unwrap().as_text().cloned().unwrap();
            let priority_type = row.get_value(2).unwrap().as_text().cloned().unwrap();
            let status_type = row.get_value(3).unwrap().as_text().cloned().unwrap();

            assert_eq!(priority, 3); // Priority::High = 3
            assert_eq!(status, "InProgress"); // TaskStatus::InProgress as text
            assert_eq!(priority_type, "integer"); // integer(enum) stores as INTEGER
            assert_eq!(status_type, "text"); // text(enum) stores as TEXT
        }
    }
}

#[cfg(feature = "serde")]
#[tokio::test]
async fn test_json_storage_types() {
    #[cfg(feature = "rusqlite")]
    let conn = setup_test_db();
    #[cfg(any(feature = "turso", feature = "libsql"))]
    let conn = setup_test_db().await;

    let (db, json_table) = drizzle!(conn, [JsonFields]);

    let json_data = JsonData {
        value: 42,
        message: "Hello JSON".to_string(),
    };

    let data = InsertJsonFields::default()
        .with_text_json(json_data.clone())
        // .with_blob_json(json_data.clone())
        .with_regular_text("regular".to_string());

    #[cfg(feature = "rusqlite")]
    let result = db.insert(json_table).values([data]).execute().unwrap();
    #[cfg(any(feature = "turso", feature = "libsql"))]
    let result = db
        .insert(json_table)
        .values([data])
        .execute()
        .await
        .unwrap();

    assert_eq!(result, 1);

    // Verify JSON storage type
    let query = "SELECT typeof(text_json) as text_type FROM json_fields WHERE id = 1";
    #[cfg(feature = "rusqlite")]
    {
        let mut stmt = db.conn().prepare(query).unwrap();
        let row = stmt
            .query_row([], |row| Ok(row.get::<_, String>(0)?))
            .unwrap();

        assert_eq!(row, "text"); // text(json) stores as TEXT
    }
    #[cfg(any(feature = "turso", feature = "libsql"))]
    {
        let mut stmt = db.conn().prepare(query).await.unwrap();
        let mut rows = stmt.query(()).await.unwrap();

        if let Some(row) = rows.next().await.unwrap() {
            let text_type = row.get_value(0).unwrap().as_text().cloned().unwrap();
            assert_eq!(text_type, "text"); // text(json) stores as TEXT
        }
    }
}

#[cfg(feature = "uuid")]
#[tokio::test]
async fn test_uuid_primary_key_with_default_fn() {
    #[cfg(feature = "rusqlite")]
    let conn = setup_test_db();
    #[cfg(any(feature = "turso", feature = "libsql"))]
    let conn = setup_test_db().await;

    let (db, uuid_table) = drizzle!(conn, [UuidFields]);

    // Insert without specifying UUID - default_fn should generate one
    let data = InsertUuidFields::default().with_name("uuid test".to_string());

    #[cfg(feature = "rusqlite")]
    let result = db.insert(uuid_table).values([data]).execute().unwrap();
    #[cfg(any(feature = "turso", feature = "libsql"))]
    let result = db
        .insert(uuid_table)
        .values([data])
        .execute()
        .await
        .unwrap();

    assert_eq!(result, 1);

    // Verify UUID was generated and is valid
    let query = "SELECT id, name FROM uuid_fields WHERE name = 'uuid test'";
    #[cfg(feature = "rusqlite")]
    {
        let mut stmt = db.conn().prepare(query).unwrap();
        let row = stmt
            .query_row([], |row| {
                Ok((row.get::<_, uuid::Uuid>(0)?, row.get::<_, String>(1)?))
            })
            .unwrap();

        assert_eq!(row.1, "uuid test");
        // Verify it's a valid UUID (not nil)
        assert_ne!(row.0, uuid::Uuid::nil());
    }
    #[cfg(any(feature = "turso", feature = "libsql"))]
    {
        let mut stmt = db.conn().prepare(query).await.unwrap();
        let mut rows = stmt.query(()).await.unwrap();

        if let Some(row) = rows.next().await.unwrap() {
            let id_bytes = row.get_value(0).unwrap().as_blob().cloned().unwrap();
            let name = row.get_value(1).unwrap().as_text().cloned().unwrap();

            let id = uuid::Uuid::from_slice(&id_bytes).unwrap();

            assert_eq!(name, "uuid test");
            // Verify it's a valid UUID (not nil)
            assert_ne!(id, uuid::Uuid::nil());
        }
    }
}

#[tokio::test]
async fn test_nullable_vs_non_nullable() {
    #[cfg(feature = "rusqlite")]
    let conn = setup_test_db();
    #[cfg(any(feature = "turso", feature = "libsql"))]
    let conn = setup_test_db().await;

    let (db, nullable_table) = drizzle!(conn, [NullableTest]);

    // Test 1: Insert with all required fields, no optional fields
    let minimal_data = InsertNullableTest::default()
        .with_required_text("required".to_string())
        .with_required_int(123)
        .with_required_bool(true);

    #[cfg(feature = "rusqlite")]
    let result = db
        .insert(nullable_table)
        .values([minimal_data])
        .execute()
        .unwrap();
    #[cfg(any(feature = "turso", feature = "libsql"))]
    let result = db
        .insert(nullable_table)
        .values([minimal_data])
        .execute()
        .await
        .unwrap();

    assert_eq!(result, 1);

    // Test 2: Insert with all fields populated
    let full_data = InsertNullableTest::default()
        .with_required_text("full".to_string())
        .with_required_int(456)
        .with_required_bool(false)
        .with_optional_text("optional text".to_string())
        .with_optional_int(789)
        .with_optional_real(12.34)
        .with_optional_blob(vec![9, 8, 7])
        .with_optional_bool(true);

    #[cfg(feature = "rusqlite")]
    let result = db
        .insert(nullable_table)
        .values([full_data])
        .execute()
        .unwrap();
    #[cfg(any(feature = "turso", feature = "libsql"))]
    let result = db
        .insert(nullable_table)
        .values([full_data])
        .execute()
        .await
        .unwrap();

    assert_eq!(result, 1);

    // Verify both records
    let query = "SELECT required_text, optional_text, optional_int FROM nullable_test ORDER BY id";
    #[cfg(feature = "rusqlite")]
    {
        let mut stmt = db.conn().prepare(query).unwrap();
        let rows: Result<Vec<_>, _> = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, Option<String>>(1)?,
                    row.get::<_, Option<i32>>(2)?,
                ))
            })
            .unwrap()
            .collect();

        let rows = rows.unwrap();
        assert_eq!(rows.len(), 2);

        // First record: minimal data
        assert_eq!(rows[0].0, "required");
        assert_eq!(rows[0].1, None);
        assert_eq!(rows[0].2, None);

        // Second record: full data
        assert_eq!(rows[1].0, "full");
        assert_eq!(rows[1].1, Some("optional text".to_string()));
        assert_eq!(rows[1].2, Some(789));
    }
    #[cfg(any(feature = "turso", feature = "libsql"))]
    {
        let mut stmt = db.conn().prepare(query).await.unwrap();
        let mut rows = stmt.query(()).await.unwrap();
        let mut results = Vec::new();

        while let Some(row) = rows.next().await.unwrap() {
            let required_text = row.get_value(0).unwrap().as_text().cloned().unwrap();
            let optional_text = match row.get_value(1).unwrap() {
                Value::Null => None,
                val => Some(val.as_text().cloned().unwrap()),
            };
            let optional_int = match row.get_value(2).unwrap() {
                Value::Null => None,
                val => Some(val.as_integer().cloned().unwrap()),
            };
            results.push((required_text, optional_text, optional_int));
        }

        assert_eq!(results.len(), 2);

        // First record: minimal data
        assert_eq!(results[0].0, "required");
        assert_eq!(results[0].1, None);
        assert_eq!(results[0].2, None);

        // Second record: full data
        assert_eq!(results[1].0, "full");
        assert_eq!(results[1].1, Some("optional text".to_string()));
        assert_eq!(results[1].2, Some(789));
    }
}

#[test]
fn test_schema_generation() {
    // Test that all schema SQL generates without errors
    println!("AllTypes SQL: {}", AllTypes::SQL.to_sql().sql());
    println!(
        "PrimaryKeyVariations SQL: {}",
        PrimaryKeyVariations::SQL.to_sql().sql()
    );
    println!("UniqueFields SQL: {}", UniqueFields::SQL.to_sql().sql());
    println!(
        "CompileTimeDefaults SQL: {}",
        CompileTimeDefaults::SQL.to_sql().sql()
    );
    println!(
        "RuntimeDefaults SQL: {}",
        RuntimeDefaults::SQL.to_sql().sql()
    );
    println!("EnumFields SQL: {}", EnumFields::SQL.to_sql().sql());
    println!("NullableTest SQL: {}", NullableTest::SQL.to_sql().sql());

    #[cfg(feature = "serde")]
    println!("JsonFields SQL: {}", JsonFields::SQL.to_sql().sql());

    #[cfg(feature = "uuid")]
    println!("UuidFields SQL: {}", UuidFields::SQL.to_sql().sql());

    // Just verify they all compile and don't panic
    assert!(true);
}

#[test]
fn test_insert_update_models_generation() {
    // Verify that Insert and Update models are generated for all table types
    let _insert_all_types = InsertAllTypes::default();
    let _update_all_types = UpdateAllTypes::default();

    let _insert_pk = InsertPrimaryKeyVariations::default();
    let _update_pk = UpdatePrimaryKeyVariations::default();

    let _insert_unique = InsertUniqueFields::default();
    let _update_unique = UpdateUniqueFields::default();

    let _insert_defaults = InsertCompileTimeDefaults::default();
    let _update_defaults = UpdateCompileTimeDefaults::default();

    let _insert_runtime = InsertRuntimeDefaults::default();
    let _update_runtime = UpdateRuntimeDefaults::default();

    let _insert_enums = InsertEnumFields::default();
    let _update_enums = UpdateEnumFields::default();

    let _insert_nullable = InsertNullableTest::default();
    let _update_nullable = UpdateNullableTest::default();

    #[cfg(feature = "serde")]
    {
        let _insert_json = InsertJsonFields::default();
        let _update_json = UpdateJsonFields::default();
    }

    #[cfg(feature = "uuid")]
    {
        let _insert_uuid = InsertUuidFields::default();
        let _update_uuid = UpdateUuidFields::default();
    }

    // If this compiles, all model generation worked correctly
    assert!(true);
}

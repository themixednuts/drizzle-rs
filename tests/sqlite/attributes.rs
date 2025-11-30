#![cfg(any(feature = "rusqlite", feature = "turso", feature = "libsql"))]

use drizzle::prelude::*;
use drizzle_macros::sqlite_test;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

// Test all SQLite column types
#[SQLiteTable]
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
#[SQLiteTable]
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
#[SQLiteTable]
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

#[SQLiteTable]
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
#[SQLiteTable]
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
#[SQLiteTable]
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
#[SQLiteTable]
struct UuidFields {
    #[blob(primary, default_fn = uuid::Uuid::new_v4)]
    id: uuid::Uuid,
    #[text]
    name: String,
    #[blob]
    other_uuid: Option<uuid::Uuid>,
}

// Test nullable vs non-nullable fields
#[SQLiteTable]
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

// Schemas for individual table tests
#[derive(SQLiteSchema)]
struct AllTypesSchema {
    all_types: AllTypes,
}

#[derive(SQLiteSchema)]
struct PrimaryKeyVariationsSchema {
    pk_variations: PrimaryKeyVariations,
}

#[derive(SQLiteSchema)]
struct ManualPrimaryKeySchema {
    manual_pk: ManualPrimaryKey,
}

#[derive(SQLiteSchema)]
struct UniqueFieldsSchema {
    unique_fields: UniqueFields,
}

#[derive(SQLiteSchema)]
struct CompileTimeDefaultsSchema {
    compile_defaults: CompileTimeDefaults,
}

#[derive(SQLiteSchema)]
struct RuntimeDefaultsSchema {
    runtime_defaults: RuntimeDefaults,
}

#[derive(SQLiteSchema)]
struct EnumFieldsSchema {
    enum_fields: EnumFields,
}

#[derive(SQLiteSchema)]
struct ComplexEnumFieldsSchema {
    complex_enum_fields: ComplexEnumFields,
}

#[cfg(feature = "serde")]
#[derive(SQLiteSchema)]
struct JsonFieldsSchema {
    json_fields: JsonFields,
}

#[cfg(feature = "uuid")]
#[derive(SQLiteSchema)]
struct UuidFieldsSchema {
    uuid_fields: UuidFields,
}

#[derive(SQLiteSchema)]
struct NullableTestSchema {
    nullable_test: NullableTest,
}

sqlite_test!(test_all_column_types, AllTypesSchema, {
    let all_types = schema.all_types;

    // Test insertion with all column types
    let test_data = InsertAllTypes::new("test text", 123, 45.67, [1, 2, 3, 4, 5], true);

    let result = drizzle_exec!(db.insert(all_types).values([test_data]).execute());
    assert_eq!(result, 1);
});

sqlite_test!(
    test_primary_key_autoincrement,
    PrimaryKeyVariationsSchema,
    {
        let pk_table = schema.pk_variations;

        // Insert multiple records to test autoincrement
        let data1 = InsertPrimaryKeyVariations::new("first");
        let data2 = InsertPrimaryKeyVariations::new("second");

        drizzle_exec!(db.insert(pk_table).values([data1]).execute());
        drizzle_exec!(db.insert(pk_table).values([data2]).execute());

        // Verify autoincrement worked using unified approach
        let select_query = db
            .select((pk_table.auto_id, pk_table.name))
            .from(pk_table)
            .order_by(pk_table.auto_id);

        #[derive(SQLiteFromRow, Debug, PartialEq)]
        struct ReturnResult(i32, String);

        let results: Vec<ReturnResult> = drizzle_exec!(db.all(select_query));

        assert_eq!(results.len(), 2);
        assert_eq!(results[0], ReturnResult(1, "first".to_string()));
        assert_eq!(results[1], ReturnResult(2, "second".to_string()));
    }
);

sqlite_test!(test_manual_primary_key, ManualPrimaryKeySchema, {
    let manual_pk = schema.manual_pk;

    let data = InsertManualPrimaryKey::new("custom_id_123", "Test description");

    let result = drizzle_exec!(db.insert(manual_pk).values([data]).execute());
    assert_eq!(result, 1);

    // Verify the manual primary key using unified query approach
    let select_query = db
        .select(())
        .from(manual_pk)
        .r#where(eq(manual_pk.manual_id, "custom_id_123"));

    #[derive(SQLiteFromRow, Debug, PartialEq)]
    struct ReturnResult(String, String);

    let results: Vec<ReturnResult> = drizzle_exec!(db.all(select_query));

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].0, "custom_id_123");
    assert_eq!(results[0].1, "Test description");
});

sqlite_test!(test_unique_constraints, UniqueFieldsSchema, {
    let unique_table = schema.unique_fields;

    // Insert first record
    let data1 =
        InsertUniqueFields::new("test@example.com", "testuser").with_display_name("Test User");

    let result1 = drizzle_exec!(db.insert(unique_table).values([data1]).execute());
    assert_eq!(result1, 1);

    // Try to insert duplicate email - should fail
    let data2 = InsertUniqueFields::new("test@example.com", "anotheruser")
        .with_display_name("Another User");

    let result2 = drizzle_try!(db.insert(unique_table).values([data2]).execute());
    assert!(result2.is_err()); // Should fail due to unique constraint
});

sqlite_test!(test_compile_time_defaults, CompileTimeDefaultsSchema, {
    let defaults_table = schema.compile_defaults;

    // Insert with minimal data - defaults should be used
    let data = InsertCompileTimeDefaults::new();

    let result = drizzle_exec!(db.insert(defaults_table).values([data]).execute());
    assert_eq!(result, 1);

    // Verify compile-time defaults were applied
    let select_query = db
        .select((
            defaults_table.name,
            defaults_table.answer,
            defaults_table.pi,
            defaults_table.active,
            defaults_table.status,
        ))
        .from(defaults_table)
        .r#where(eq(defaults_table.id, 1));

    #[derive(SQLiteFromRow, Debug)]
    struct ReturnResult(String, i32, f64, bool, String);
    let results: Vec<ReturnResult> = drizzle_exec!(db.all(select_query));

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].0, "default_name");
    assert_eq!(results[0].1, 42);
    assert_eq!((results[0].2 - 3.14).abs() < f64::EPSILON, true); // Float comparison
    assert_eq!(results[0].3, true);
    assert_eq!(results[0].4, "pending");
});

sqlite_test!(test_runtime_defaults, RuntimeDefaultsSchema, {
    let RuntimeDefaultsSchema { runtime_defaults } = schema;

    // Insert with minimal data - runtime defaults should be used
    let data = InsertRuntimeDefaults::new("test");

    let result = drizzle_exec!(db.insert(runtime_defaults).values([data]).execute());
    assert_eq!(result, 1);

    // Verify runtime defaults were applied
    let select_query = db
        .select((
            runtime_defaults.empty_text,
            runtime_defaults.computed_int,
            runtime_defaults.name,
        ))
        .from(runtime_defaults)
        .r#where(eq(runtime_defaults.id, 1));

    #[derive(SQLiteFromRow, Debug)]
    struct ReturnResult(String, i32, String);
    let results: Vec<ReturnResult> = drizzle_exec!(db.all(select_query));

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].0, ""); // String::new() returns empty string
    assert_eq!(results[0].1, 100); // Closure returns 100
    assert_eq!(results[0].2, "test");
});

sqlite_test!(test_enum_storage_types, EnumFieldsSchema, {
    let enum_table = schema.enum_fields;

    // Test different enum storage types
    let data = InsertEnumFields::new(Priority::High, TaskStatus::InProgress, "Test task");

    let result = drizzle_exec!(db.insert(enum_table).values([data]).execute());
    assert_eq!(result, 1);

    // Verify enum storage using typeof helper
    let priority_col = enum_table.priority;
    let status_col = enum_table.status;
    let select_query = db
        .select((
            priority_col,
            status_col,
            alias(r#typeof(priority_col), "priority_type"),
            alias(r#typeof(status_col), "status_type"),
        ))
        .from(enum_table)
        .r#where(eq(enum_table.id, 1));

    #[derive(SQLiteFromRow, Debug)]
    struct ReturnResult(i32, String, String, String);
    let results: Vec<ReturnResult> = drizzle_exec!(db.all(select_query));

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].0, 3); // Priority::High = 3
    assert_eq!(results[0].1, "InProgress"); // TaskStatus::InProgress as text
    assert_eq!(results[0].2, "integer"); // integer(enum) stores as INTEGER
    assert_eq!(results[0].3, "text"); // text(enum) stores as TEXT
});

#[cfg(feature = "serde")]
sqlite_test!(test_json_storage_types, JsonFieldsSchema, {
    let json_table = schema.json_fields;

    let json_data = JsonData {
        value: 42,
        message: "Hello JSON".to_string(),
    };

    let data = InsertJsonFields::new("regular").with_text_json(json_data);

    let result = drizzle_exec!(db.insert(json_table).values([data]).execute());

    assert_eq!(result, 1);

    // Verify JSON storage type
    let text_json_col = json_table.text_json;
    let select_query = db
        .select(alias(r#typeof(text_json_col), "text_type"))
        .from(json_table)
        .r#where(eq(json_table.id, 1));

    #[derive(SQLiteFromRow, Debug)]
    struct ReturnResult(String);
    let results: Vec<ReturnResult> = drizzle_exec!(db.all(select_query));

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].0, "text"); // text(json) stores as TEXT
});

#[cfg(feature = "uuid")]
sqlite_test!(test_uuid_primary_key_with_default_fn, UuidFieldsSchema, {
    let uuid_table = schema.uuid_fields;

    // Insert without specifying UUID - default_fn should generate one
    let data = InsertUuidFields::new("uuid test");

    let result = drizzle_exec!(db.insert(uuid_table).values([data]).execute());

    assert_eq!(result, 1);

    // Verify UUID was generated and is valid
    let select_query = db
        .select((uuid_table.id, uuid_table.name))
        .from(uuid_table)
        .r#where(eq(uuid_table.name, "uuid test"));

    #[derive(SQLiteFromRow, Debug)]
    struct ReturnResult(uuid::Uuid, String);
    let results: Vec<ReturnResult> = drizzle_exec!(db.all(select_query));

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].1, "uuid test");

    // Validate UUID format and version
    let generated_uuid = results[0].0;
    assert_ne!(generated_uuid, uuid::Uuid::nil());
    assert_eq!(generated_uuid.get_version(), Some(uuid::Version::Random));

    // Verify UUID storage type using typeof
    let id_col = uuid_table.id;
    let type_query = db
        .select(alias(r#typeof(id_col), "id_type"))
        .from(uuid_table)
        .r#where(eq(uuid_table.name, "uuid test"));

    #[derive(SQLiteFromRow, Debug)]
    struct TypeResult(String);
    let type_results: Vec<TypeResult> = drizzle_exec!(db.all(type_query));

    assert_eq!(type_results.len(), 1);
    assert_eq!(type_results[0].0, "blob"); // blob(primary) stores UUIDs as BLOB
});

sqlite_test!(test_nullable_vs_non_nullable, NullableTestSchema, {
    let nullable_table = schema.nullable_test;

    // Test 1: Insert with all required fields, no optional fields
    let minimal_data = InsertNullableTest::new("required", 123, true);

    let result = drizzle_exec!(db.insert(nullable_table).values([minimal_data]).execute());

    assert_eq!(result, 1);

    // Test 2: Insert with all fields populated
    let full_data = InsertNullableTest::new("full", 456, false)
        .with_optional_text("optional text")
        .with_optional_int(789)
        .with_optional_real(12.34)
        .with_optional_blob([9, 8, 7])
        .with_optional_bool(true);

    let result = drizzle_exec!(db.insert(nullable_table).values([full_data]).execute());

    assert_eq!(result, 1);

    // Verify both records using unified query approach
    let select_query = db
        .select((
            nullable_table.required_text,
            nullable_table.optional_text,
            nullable_table.optional_int,
        ))
        .from(nullable_table)
        .order_by(nullable_table.id);

    #[derive(SQLiteFromRow, Debug)]
    struct ReturnResult(String, Option<String>, Option<i32>);
    let results: Vec<ReturnResult> = drizzle_exec!(db.all(select_query));

    assert_eq!(results.len(), 2);

    // First record: minimal data
    assert_eq!(results[0].0, "required");
    assert_eq!(results[0].1, None);
    assert_eq!(results[0].2, None);

    // Second record: full data
    assert_eq!(results[1].0, "full");
    assert_eq!(results[1].1, Some("optional text".to_string()));
    assert_eq!(results[1].2, Some(789));
});

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

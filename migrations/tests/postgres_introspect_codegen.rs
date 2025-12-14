//! Integration tests for PostgreSQL introspection and Rust schema code generation
//!
//! These tests verify that the PostgreSQL introspection and code generation
//! correctly handle various PostgreSQL-specific types and constraints.

use drizzle_migrations::{
    parser::{Dialect, SchemaParser},
    postgres::{
        PostgresDDL,
        codegen::{CodegenOptions, generate_rust_schema, sql_type_to_rust_type},
        ddl::{
            Column, Enum, ForeignKey, Identity, Index, IndexColumn, PrimaryKey, Table,
            UniqueConstraint,
        },
        introspect::{
            RawColumnInfo, RawForeignKeyInfo, RawIndexColumnInfo, RawIndexInfo, RawPrimaryKeyInfo,
            RawTableInfo, RawUniqueInfo, process_columns, process_foreign_keys, process_indexes,
            process_primary_keys, process_tables, process_unique_constraints,
        },
    },
};

// =============================================================================
// Helper Functions
// =============================================================================

fn identity_always() -> Identity {
    Identity {
        name: "test_seq".to_string(),
        schema: Some("public".to_string()),
        type_: "ALWAYS".to_string(),
        increment: None,
        min_value: None,
        max_value: None,
        start_with: None,
        cache: None,
        cycle: None,
    }
}

/// Create a PostgresDDL from introspection-like data
fn create_test_ddl() -> PostgresDDL {
    let mut ddl = PostgresDDL::new();

    // Add a users table
    ddl.tables.push(Table {
        schema: "public".to_string(),
        name: "users".to_string(),
        is_rls_enabled: Some(false),
    });

    // Add columns for users
    ddl.columns.push(Column {
        schema: "public".to_string(),
        table: "users".to_string(),
        name: "id".to_string(),
        sql_type: "int4".to_string(),
        type_schema: None,
        not_null: true,
        default: None,
        generated: None,
        identity: Some(identity_always()),
        dimensions: None,
    });

    ddl.columns.push(Column {
        schema: "public".to_string(),
        table: "users".to_string(),
        name: "email".to_string(),
        sql_type: "text".to_string(),
        type_schema: None,
        not_null: true,
        default: None,
        generated: None,
        identity: None,
        dimensions: None,
    });

    ddl.columns.push(Column {
        schema: "public".to_string(),
        table: "users".to_string(),
        name: "bio".to_string(),
        sql_type: "text".to_string(),
        type_schema: None,
        not_null: false,
        default: None,
        generated: None,
        identity: None,
        dimensions: None,
    });

    // Add primary key for users
    ddl.pks.push(PrimaryKey {
        schema: "public".to_string(),
        table: "users".to_string(),
        name: "users_pkey".to_string(),
        name_explicit: true,
        columns: vec!["id".to_string()],
    });

    // Add unique constraint for email
    ddl.uniques.push(UniqueConstraint {
        schema: "public".to_string(),
        table: "users".to_string(),
        name: "users_email_key".to_string(),
        name_explicit: true,
        columns: vec!["email".to_string()],
        nulls_not_distinct: false,
    });

    // Add a posts table
    ddl.tables.push(Table {
        schema: "public".to_string(),
        name: "posts".to_string(),
        is_rls_enabled: Some(false),
    });

    ddl.columns.push(Column {
        schema: "public".to_string(),
        table: "posts".to_string(),
        name: "id".to_string(),
        sql_type: "int4".to_string(),
        type_schema: None,
        not_null: true,
        default: None,
        generated: None,
        identity: Some(identity_always()),
        dimensions: None,
    });

    ddl.columns.push(Column {
        schema: "public".to_string(),
        table: "posts".to_string(),
        name: "title".to_string(),
        sql_type: "text".to_string(),
        type_schema: None,
        not_null: true,
        default: None,
        generated: None,
        identity: None,
        dimensions: None,
    });

    ddl.columns.push(Column {
        schema: "public".to_string(),
        table: "posts".to_string(),
        name: "author_id".to_string(),
        sql_type: "int4".to_string(),
        type_schema: None,
        not_null: true,
        default: None,
        generated: None,
        identity: None,
        dimensions: None,
    });

    // Add primary key for posts
    ddl.pks.push(PrimaryKey {
        schema: "public".to_string(),
        table: "posts".to_string(),
        name: "posts_pkey".to_string(),
        name_explicit: true,
        columns: vec!["id".to_string()],
    });

    // Add foreign key from posts to users
    ddl.fks.push(ForeignKey {
        schema: "public".to_string(),
        table: "posts".to_string(),
        name: "posts_author_id_fkey".to_string(),
        name_explicit: true,
        columns: vec!["author_id".to_string()],
        schema_to: "public".to_string(),
        table_to: "users".to_string(),
        columns_to: vec!["id".to_string()],
        on_update: Some("NO ACTION".to_string()),
        on_delete: Some("CASCADE".to_string()),
    });

    // Add an index
    ddl.indexes.push(Index {
        schema: "public".to_string(),
        table: "posts".to_string(),
        name: "idx_posts_title".to_string(),
        columns: vec![IndexColumn {
            value: "title".to_string(),
            is_expression: false,
            asc: true,
            nulls_first: false,
            opclass: None,
        }],
        is_unique: false,
        r#where: None,
        method: Some("btree".to_string()),
        concurrently: false,
        r#with: None,
    });

    ddl
}

// =============================================================================
// Code Generation Tests
// =============================================================================

#[test]
fn test_generate_postgres_schema() {
    let ddl = create_test_ddl();
    let options = CodegenOptions::default();
    let generated = generate_rust_schema(&ddl, &options);

    println!("Generated PostgreSQL schema:\n{}", generated.code);

    // Verify basic structure
    assert!(
        generated
            .code
            .contains("use drizzle::postgres::prelude::*;"),
        "Should have postgres import"
    );
    assert!(generated.tables.contains(&"users".to_string()));
    assert!(generated.tables.contains(&"posts".to_string()));
}

#[test]
fn test_parse_generated_postgres_code() {
    let ddl = create_test_ddl();
    let options = CodegenOptions::default();
    let generated = generate_rust_schema(&ddl, &options);

    println!("Generated code:\n{}", generated.code);

    // Parse with SchemaParser
    let parsed = SchemaParser::parse(&generated.code);

    // Verify dialect detection
    assert_eq!(
        parsed.dialect,
        Dialect::PostgreSQL,
        "Should detect Postgres dialect"
    );

    // Verify Users table
    let users = parsed.table("Users").expect("Should have Users struct");
    assert_eq!(users.attr, "#[PostgresTable]");

    let id = users.field("id").expect("Users should have id field");
    assert!(id.is_primary_key(), "id should be primary key");
    assert!(
        id.has_attr("identity(always)"),
        "id should have identity(always) attribute"
    );
    assert_eq!(id.ty, "i32", "id should be i32");

    let email = users.field("email").expect("Users should have email field");
    assert!(email.is_unique(), "email should be unique");
    assert!(!email.is_nullable(), "email should not be nullable");

    let bio = users.field("bio").expect("Users should have bio field");
    assert!(bio.is_nullable(), "bio should be nullable");
    assert_eq!(bio.ty, "Option<String>", "bio should be Option<String>");
}

#[test]
fn test_postgres_foreign_key_generation() {
    let ddl = create_test_ddl();
    let options = CodegenOptions::default();
    let generated = generate_rust_schema(&ddl, &options);

    let parsed = SchemaParser::parse(&generated.code);

    let posts = parsed.table("Posts").expect("Should have Posts struct");

    let author_id = posts
        .field("author_id")
        .expect("Posts should have author_id field");

    // Check FK reference
    assert_eq!(
        author_id.references(),
        Some("Users::id".to_string()),
        "author_id should reference Users::id"
    );

    // Check on_delete cascade (since it's not NO ACTION)
    assert_eq!(
        author_id.on_delete(),
        Some("cascade".to_string()),
        "author_id should have on_delete cascade"
    );
}

#[test]
fn test_postgres_index_generation() {
    let ddl = create_test_ddl();
    let options = CodegenOptions::default();
    let generated = generate_rust_schema(&ddl, &options);

    println!("Generated code:\n{}", generated.code);

    let parsed = SchemaParser::parse(&generated.code);

    // Debug: print all indexes found
    let index_names = parsed.index_names();
    println!("Found indexes: {:?}", index_names);

    // The index name uses to_pascal_case on "idx_posts_title" which produces "IdxPostsTitle"
    let idx = parsed
        .index("IdxPostsTitle")
        .expect("Should have IdxPostsTitle index");

    assert!(!idx.is_unique(), "Index should not be unique");
    assert_eq!(idx.columns.len(), 1);
    assert_eq!(idx.columns[0], "Posts::title");
}

#[test]
fn test_postgres_type_mapping() {
    // Integer types
    assert_eq!(sql_type_to_rust_type("int2", true), "i16");
    assert_eq!(sql_type_to_rust_type("int4", true), "i32");
    assert_eq!(sql_type_to_rust_type("int8", true), "i64");
    assert_eq!(sql_type_to_rust_type("serial", true), "i32");
    assert_eq!(sql_type_to_rust_type("bigserial", true), "i64");

    // Float types
    assert_eq!(sql_type_to_rust_type("float4", true), "f32");
    assert_eq!(sql_type_to_rust_type("float8", true), "f64");

    // Text types
    assert_eq!(sql_type_to_rust_type("text", true), "String");
    assert_eq!(sql_type_to_rust_type("varchar", true), "String");

    // Other types
    assert_eq!(sql_type_to_rust_type("bool", true), "bool");
    assert_eq!(sql_type_to_rust_type("bytea", true), "Vec<u8>");
    assert_eq!(sql_type_to_rust_type("uuid", true), "uuid::Uuid");
    assert_eq!(sql_type_to_rust_type("jsonb", true), "serde_json::Value");

    // Nullable
    assert_eq!(sql_type_to_rust_type("int4", false), "Option<i32>");
    assert_eq!(sql_type_to_rust_type("text", false), "Option<String>");
}

// =============================================================================
// Introspection Processing Tests
// =============================================================================

#[test]
fn test_process_tables() {
    let raw = vec![
        RawTableInfo {
            schema: "public".to_string(),
            name: "users".to_string(),
            is_rls_enabled: false,
        },
        RawTableInfo {
            schema: "public".to_string(),
            name: "posts".to_string(),
            is_rls_enabled: true,
        },
        // System table should be filtered
        RawTableInfo {
            schema: "pg_catalog".to_string(),
            name: "pg_class".to_string(),
            is_rls_enabled: false,
        },
    ];

    let tables = process_tables(&raw);
    assert_eq!(tables.len(), 2);
    assert!(tables.iter().any(|t| t.name == "users"));
    assert!(tables.iter().any(|t| t.name == "posts"));
    assert!(tables.iter().all(|t| t.schema != "pg_catalog"));
}

#[test]
fn test_process_columns() {
    let raw = vec![
        RawColumnInfo {
            schema: "public".to_string(),
            table: "users".to_string(),
            name: "id".to_string(),
            column_type: "int4".to_string(),
            type_schema: None,
            not_null: true,
            default_value: None,
            is_identity: true,
            identity_type: Some("ALWAYS".to_string()),
            is_generated: false,
            generated_expression: None,
            ordinal_position: 1,
        },
        RawColumnInfo {
            schema: "public".to_string(),
            table: "users".to_string(),
            name: "name".to_string(),
            column_type: "text".to_string(),
            type_schema: None,
            not_null: true,
            default_value: Some("'Anonymous'::text".to_string()),
            is_identity: false,
            identity_type: None,
            is_generated: false,
            generated_expression: None,
            ordinal_position: 2,
        },
    ];

    let columns = process_columns(&raw);
    assert_eq!(columns.len(), 2);

    let id_col = columns.iter().find(|c| c.name == "id").unwrap();
    assert!(id_col.identity.is_some());
    assert!(id_col.not_null);

    let name_col = columns.iter().find(|c| c.name == "name").unwrap();
    assert_eq!(name_col.default, Some("'Anonymous'::text".to_string()));
}

#[test]
fn test_process_indexes() {
    let raw = vec![
        RawIndexInfo {
            schema: "public".to_string(),
            table: "users".to_string(),
            name: "idx_users_email".to_string(),
            is_unique: true,
            is_primary: false,
            method: "btree".to_string(),
            columns: vec![RawIndexColumnInfo {
                name: "email".to_string(),
                is_expression: false,
                asc: true,
                nulls_first: false,
                opclass: None,
            }],
            where_clause: None,
            concurrent: false,
        },
        // Primary key index should be filtered out
        RawIndexInfo {
            schema: "public".to_string(),
            table: "users".to_string(),
            name: "users_pkey".to_string(),
            is_unique: true,
            is_primary: true,
            method: "btree".to_string(),
            columns: vec![RawIndexColumnInfo {
                name: "id".to_string(),
                is_expression: false,
                asc: true,
                nulls_first: false,
                opclass: None,
            }],
            where_clause: None,
            concurrent: false,
        },
    ];

    let indexes = process_indexes(&raw);
    assert_eq!(indexes.len(), 1);
    assert_eq!(indexes[0].name, "idx_users_email");
    assert!(indexes[0].is_unique);
}

#[test]
fn test_process_foreign_keys() {
    let raw = vec![RawForeignKeyInfo {
        schema: "public".to_string(),
        table: "posts".to_string(),
        name: "posts_author_id_fkey".to_string(),
        columns: vec!["author_id".to_string()],
        schema_to: "public".to_string(),
        table_to: "users".to_string(),
        columns_to: vec!["id".to_string()],
        on_update: "NO ACTION".to_string(),
        on_delete: "CASCADE".to_string(),
    }];

    let fks = process_foreign_keys(&raw);
    assert_eq!(fks.len(), 1);
    assert_eq!(fks[0].name, "posts_author_id_fkey");
    assert_eq!(fks[0].table_to, "users");
    assert_eq!(fks[0].on_delete, Some("CASCADE".to_string()));
}

#[test]
fn test_process_primary_keys() {
    let raw = vec![RawPrimaryKeyInfo {
        schema: "public".to_string(),
        table: "users".to_string(),
        name: "users_pkey".to_string(),
        columns: vec!["id".to_string()],
    }];

    let pks = process_primary_keys(&raw);
    assert_eq!(pks.len(), 1);
    assert_eq!(pks[0].name, "users_pkey");
    assert_eq!(pks[0].columns, vec!["id"]);
}

#[test]
fn test_process_unique_constraints() {
    let raw = vec![
        RawUniqueInfo {
            schema: "public".to_string(),
            table: "users".to_string(),
            name: "users_email_key".to_string(),
            columns: vec!["email".to_string()],
            nulls_not_distinct: false,
        },
        RawUniqueInfo {
            schema: "public".to_string(),
            table: "users".to_string(),
            name: "users_username_domain_key".to_string(),
            columns: vec!["username".to_string(), "domain".to_string()],
            nulls_not_distinct: true,
        },
    ];

    let uniques = process_unique_constraints(&raw);
    assert_eq!(uniques.len(), 2);

    let email_unique = uniques
        .iter()
        .find(|u| u.name == "users_email_key")
        .unwrap();
    assert_eq!(email_unique.columns.len(), 1);
    assert!(!email_unique.nulls_not_distinct);

    let composite = uniques
        .iter()
        .find(|u| u.name == "users_username_domain_key")
        .unwrap();
    assert_eq!(composite.columns.len(), 2);
    assert!(composite.nulls_not_distinct);
}

// =============================================================================
// Enum Tests
// =============================================================================

#[test]
fn test_process_enums() {
    use drizzle_migrations::postgres::introspect::{RawEnumInfo, process_enums};

    let raw = vec![
        RawEnumInfo {
            schema: "public".to_string(),
            name: "status".to_string(),
            values: vec![
                "pending".to_string(),
                "active".to_string(),
                "completed".to_string(),
            ],
        },
        RawEnumInfo {
            schema: "public".to_string(),
            name: "priority".to_string(),
            values: vec!["low".to_string(), "medium".to_string(), "high".to_string()],
        },
        // System schema should be filtered
        RawEnumInfo {
            schema: "pg_catalog".to_string(),
            name: "anyenum".to_string(),
            values: vec![],
        },
    ];

    let enums = process_enums(&raw);
    assert_eq!(enums.len(), 2);

    let status = enums.iter().find(|e| e.name == "status").unwrap();
    assert_eq!(status.schema, "public");
    assert_eq!(status.values.len(), 3);
    assert_eq!(status.values[0], "pending");

    let priority = enums.iter().find(|e| e.name == "priority").unwrap();
    assert_eq!(priority.values.len(), 3);
}

// =============================================================================
// Complex Type Tests
// =============================================================================

#[test]
fn test_postgres_array_type_mapping() {
    // TODO: Once array type support is added to codegen
    // For now, arrays default to String
    assert_eq!(sql_type_to_rust_type("_int4", true), "String"); // PostgreSQL array types use _ prefix
    assert_eq!(sql_type_to_rust_type("_text", true), "String");
}

#[test]
fn test_postgres_date_time_types() {
    assert_eq!(sql_type_to_rust_type("date", true), "chrono::NaiveDate");
    assert_eq!(sql_type_to_rust_type("time", true), "chrono::NaiveTime");
    assert_eq!(
        sql_type_to_rust_type("timestamp", true),
        "chrono::NaiveDateTime"
    );
    assert_eq!(
        sql_type_to_rust_type("timestamptz", true),
        "chrono::DateTime<chrono::Utc>"
    );
}

#[test]
fn test_postgres_json_types() {
    assert_eq!(sql_type_to_rust_type("json", true), "serde_json::Value");
    assert_eq!(sql_type_to_rust_type("jsonb", true), "serde_json::Value");
}

#[test]
fn test_postgres_numeric_types() {
    // Small integers
    assert_eq!(sql_type_to_rust_type("int2", true), "i16");
    assert_eq!(sql_type_to_rust_type("smallint", true), "i16");
    assert_eq!(sql_type_to_rust_type("smallserial", true), "i16");

    // Regular integers
    assert_eq!(sql_type_to_rust_type("int4", true), "i32");
    assert_eq!(sql_type_to_rust_type("integer", true), "i32");
    assert_eq!(sql_type_to_rust_type("serial", true), "i32");

    // Big integers
    assert_eq!(sql_type_to_rust_type("int8", true), "i64");
    assert_eq!(sql_type_to_rust_type("bigint", true), "i64");
    assert_eq!(sql_type_to_rust_type("bigserial", true), "i64");

    // Floating point
    assert_eq!(sql_type_to_rust_type("float4", true), "f32");
    assert_eq!(sql_type_to_rust_type("real", true), "f32");
    assert_eq!(sql_type_to_rust_type("float8", true), "f64");

    // Numeric/decimal stored as String for precision
    assert_eq!(sql_type_to_rust_type("numeric", true), "String");
    assert_eq!(sql_type_to_rust_type("decimal", true), "String");
}

#[test]
fn test_postgres_text_types() {
    assert_eq!(sql_type_to_rust_type("text", true), "String");
    assert_eq!(sql_type_to_rust_type("varchar", true), "String");
    assert_eq!(sql_type_to_rust_type("char", true), "String");
    assert_eq!(sql_type_to_rust_type("bpchar", true), "String");
    assert_eq!(sql_type_to_rust_type("name", true), "String");
}

#[test]
fn test_postgres_binary_types() {
    assert_eq!(sql_type_to_rust_type("bytea", true), "Vec<u8>");
}

#[test]
fn test_postgres_uuid_type() {
    assert_eq!(sql_type_to_rust_type("uuid", true), "uuid::Uuid");
    assert_eq!(sql_type_to_rust_type("uuid", false), "Option<uuid::Uuid>");
}

// =============================================================================
// Check Constraint Tests
// =============================================================================

#[test]
fn test_process_check_constraints() {
    use drizzle_migrations::postgres::introspect::{RawCheckInfo, process_check_constraints};

    let raw = vec![
        RawCheckInfo {
            schema: "public".to_string(),
            table: "products".to_string(),
            name: "products_price_check".to_string(),
            expression: "price > 0".to_string(),
        },
        RawCheckInfo {
            schema: "public".to_string(),
            table: "products".to_string(),
            name: "products_quantity_check".to_string(),
            expression: "quantity >= 0".to_string(),
        },
    ];

    let checks = process_check_constraints(&raw);
    assert_eq!(checks.len(), 2);

    let price_check = checks
        .iter()
        .find(|c| c.name == "products_price_check")
        .unwrap();
    assert_eq!(price_check.value, "price > 0");
    assert_eq!(price_check.table, "products");
}

// =============================================================================
// Generated Column Tests
// =============================================================================

#[test]
fn test_generated_column_codegen() {
    use drizzle_migrations::postgres::ddl::Generated;

    let mut ddl = PostgresDDL::new();

    ddl.tables.push(Table {
        schema: "public".to_string(),
        name: "products".to_string(),
        is_rls_enabled: Some(false),
    });

    ddl.columns.push(Column {
        schema: "public".to_string(),
        table: "products".to_string(),
        name: "price".to_string(),
        sql_type: "numeric".to_string(),
        type_schema: None,
        not_null: true,
        default: None,
        generated: None,
        identity: None,
        dimensions: None,
    });

    ddl.columns.push(Column {
        schema: "public".to_string(),
        table: "products".to_string(),
        name: "quantity".to_string(),
        sql_type: "int4".to_string(),
        type_schema: None,
        not_null: true,
        default: None,
        generated: None,
        identity: None,
        dimensions: None,
    });

    ddl.columns.push(Column {
        schema: "public".to_string(),
        table: "products".to_string(),
        name: "total".to_string(),
        sql_type: "numeric".to_string(),
        type_schema: None,
        not_null: true,
        default: None,
        generated: Some(Generated {
            expression: "price * quantity".to_string(),
            type_: "stored".to_string(),
        }),
        identity: None,
        dimensions: None,
    });

    let options = CodegenOptions::default();
    let generated = generate_rust_schema(&ddl, &options);
    let parsed = SchemaParser::parse(&generated.code);

    // Verify table exists
    let table = parsed
        .table("Products")
        .expect("Should have Products table");

    // Regular columns should be present without generated attribute
    let price = table.field("price").expect("Should have price field");
    assert!(
        !price.has_attr("generated"),
        "price should NOT have generated attribute"
    );

    let quantity = table.field("quantity").expect("Should have quantity field");
    assert!(
        !quantity.has_attr("generated"),
        "quantity should NOT have generated attribute"
    );

    // Generated column should have the generated(stored, "...") attribute
    let total = table.field("total").expect("Should have total field");
    assert!(
        total.attrs.iter().any(|a| a.contains("generated(stored")),
        "total should have generated(stored, ...) attribute, got: {:?}",
        total.attrs
    );
}

// =============================================================================
// Default Value Tests
// =============================================================================

#[test]
fn test_default_value_codegen() {
    let mut ddl = PostgresDDL::new();

    ddl.tables.push(Table {
        schema: "public".to_string(),
        name: "settings".to_string(),
        is_rls_enabled: Some(false),
    });

    ddl.columns.push(Column {
        schema: "public".to_string(),
        table: "settings".to_string(),
        name: "enabled".to_string(),
        sql_type: "bool".to_string(),
        type_schema: None,
        not_null: true,
        default: Some("true".to_string()),
        generated: None,
        identity: None,
        dimensions: None,
    });

    ddl.columns.push(Column {
        schema: "public".to_string(),
        table: "settings".to_string(),
        name: "retries".to_string(),
        sql_type: "int4".to_string(),
        type_schema: None,
        not_null: true,
        default: Some("3".to_string()),
        generated: None,
        identity: None,
        dimensions: None,
    });

    ddl.columns.push(Column {
        schema: "public".to_string(),
        table: "settings".to_string(),
        name: "name".to_string(),
        sql_type: "text".to_string(),
        type_schema: None,
        not_null: true,
        default: Some("'default'::text".to_string()),
        generated: None,
        identity: None,
        dimensions: None,
    });

    let options = CodegenOptions::default();
    let generated = generate_rust_schema(&ddl, &options);
    let parsed = SchemaParser::parse(&generated.code);

    let settings = parsed.table("Settings").expect("Should have Settings");

    // Check boolean default
    let enabled = settings.field("enabled").unwrap();
    assert_eq!(enabled.default_value(), Some("true".to_string()));

    // Check numeric default
    let retries = settings.field("retries").unwrap();
    assert_eq!(retries.default_value(), Some("3".to_string()));

    // Check string default (should be quoted)
    let name = settings.field("name").unwrap();
    assert_eq!(name.default_value(), Some("\"default\"".to_string()));
}

// =============================================================================
// Identity Column Tests
// =============================================================================

#[test]
fn test_identity_column_types() {
    let mut ddl = PostgresDDL::new();

    ddl.tables.push(Table {
        schema: "public".to_string(),
        name: "test_identity".to_string(),
        is_rls_enabled: Some(false),
    });

    // Identity ALWAYS
    ddl.columns.push(Column {
        schema: "public".to_string(),
        table: "test_identity".to_string(),
        name: "id_always".to_string(),
        sql_type: "int4".to_string(),
        type_schema: None,
        not_null: true,
        default: None,
        generated: None,
        identity: Some(Identity {
            name: "id_always_seq".to_string(),
            schema: Some("public".to_string()),
            type_: "ALWAYS".to_string(),
            increment: None,
            min_value: None,
            max_value: None,
            start_with: None,
            cache: None,
            cycle: None,
        }),
        dimensions: None,
    });

    // Identity BY DEFAULT
    ddl.columns.push(Column {
        schema: "public".to_string(),
        table: "test_identity".to_string(),
        name: "id_by_default".to_string(),
        sql_type: "int4".to_string(),
        type_schema: None,
        not_null: true,
        default: None,
        generated: None,
        identity: Some(Identity {
            name: "id_by_default_seq".to_string(),
            schema: Some("public".to_string()),
            type_: "BY DEFAULT".to_string(),
            increment: None,
            min_value: None,
            max_value: None,
            start_with: None,
            cache: None,
            cycle: None,
        }),
        dimensions: None,
    });

    ddl.pks.push(PrimaryKey {
        schema: "public".to_string(),
        table: "test_identity".to_string(),
        name: "test_identity_pkey".to_string(),
        name_explicit: true,
        columns: vec!["id_always".to_string()],
    });

    let options = CodegenOptions::default();
    let generated = generate_rust_schema(&ddl, &options);
    let parsed = SchemaParser::parse(&generated.code);

    let table = parsed
        .table("TestIdentity")
        .expect("Should have TestIdentity");

    // ALWAYS should generate identity(always)
    let id_always = table.field("id_always").unwrap();
    assert!(
        id_always.has_attr("identity(always)"),
        "Should have identity(always) attribute"
    );

    // BY DEFAULT should generate just identity
    let id_by_default = table.field("id_by_default").unwrap();
    assert!(
        id_by_default.has_attr("identity"),
        "Should have identity attribute"
    );
    assert!(
        !id_by_default.has_attr("identity(always)"),
        "Should NOT have identity(always)"
    );
}

// =============================================================================
// Unique Index Tests
// =============================================================================

#[test]
fn test_unique_index_generation() {
    let mut ddl = PostgresDDL::new();

    ddl.tables.push(Table {
        schema: "public".to_string(),
        name: "items".to_string(),
        is_rls_enabled: Some(false),
    });

    ddl.columns.push(Column {
        schema: "public".to_string(),
        table: "items".to_string(),
        name: "code".to_string(),
        sql_type: "text".to_string(),
        type_schema: None,
        not_null: true,
        default: None,
        generated: None,
        identity: None,
        dimensions: None,
    });

    // Unique index
    ddl.indexes.push(Index {
        schema: "public".to_string(),
        table: "items".to_string(),
        name: "idx_items_code_unique".to_string(),
        columns: vec![IndexColumn {
            value: "code".to_string(),
            is_expression: false,
            asc: true,
            nulls_first: false,
            opclass: None,
        }],
        is_unique: true,
        r#where: None,
        method: Some("btree".to_string()),
        concurrently: false,
        r#with: None,
    });

    let options = CodegenOptions::default();
    let generated = generate_rust_schema(&ddl, &options);
    let parsed = SchemaParser::parse(&generated.code);

    let idx = parsed
        .index("IdxItemsCodeUnique")
        .expect("Should have unique index");
    assert!(idx.is_unique(), "Index should be unique");
    assert!(
        idx.attr.contains("unique"),
        "Index attr should contain unique"
    );
}

// =============================================================================
// Sequence Tests
// =============================================================================

#[test]
fn test_process_sequences() {
    use drizzle_migrations::postgres::introspect::{RawSequenceInfo, process_sequences};

    let raw = vec![
        RawSequenceInfo {
            schema: "public".to_string(),
            name: "users_id_seq".to_string(),
            data_type: "bigint".to_string(),
            start_value: "1".to_string(),
            min_value: "1".to_string(),
            max_value: "9223372036854775807".to_string(),
            increment: "1".to_string(),
            cycle: false,
            cache_value: "1".to_string(),
        },
        RawSequenceInfo {
            schema: "public".to_string(),
            name: "order_num_seq".to_string(),
            data_type: "integer".to_string(),
            start_value: "1000".to_string(),
            min_value: "1000".to_string(),
            max_value: "2147483647".to_string(),
            increment: "1".to_string(),
            cycle: true,
            cache_value: "10".to_string(),
        },
    ];

    let sequences = process_sequences(&raw);
    assert_eq!(sequences.len(), 2);

    let users_seq = sequences.iter().find(|s| s.name == "users_id_seq").unwrap();
    assert_eq!(users_seq.start_with, Some("1".to_string()));
    assert_eq!(users_seq.cycle, Some(false));

    let order_seq = sequences
        .iter()
        .find(|s| s.name == "order_num_seq")
        .unwrap();
    assert_eq!(order_seq.start_with, Some("1000".to_string()));
    assert_eq!(order_seq.cycle, Some(true));
    assert_eq!(order_seq.cache, Some("10".to_string()));
}

// =============================================================================
// Schema Generation Tests
// =============================================================================

#[test]
fn test_schema_struct_generation() {
    let ddl = create_test_ddl();
    let options = CodegenOptions {
        include_schema: true,
        schema_name: "AppSchema".to_string(),
        use_pub: true,
        ..Default::default()
    };

    let generated = generate_rust_schema(&ddl, &options);
    let parsed = SchemaParser::parse(&generated.code);

    let schema = parsed.schema.expect("Should have schema");
    assert_eq!(schema.name, "AppSchema");
    assert_eq!(schema.dialect, Dialect::PostgreSQL);

    // Should have members for tables
    assert!(schema.members.contains_key("users"));
    assert!(schema.members.contains_key("posts"));
}

// =============================================================================
// Role Tests
// =============================================================================

#[test]
fn test_process_roles() {
    use drizzle_migrations::postgres::introspect::{RawRoleInfo, process_roles};

    let raw = vec![
        RawRoleInfo {
            name: "app_user".to_string(),
            create_db: false,
            create_role: false,
            inherit: true,
        },
        RawRoleInfo {
            name: "admin".to_string(),
            create_db: true,
            create_role: true,
            inherit: true,
        },
        // System roles should be filtered
        RawRoleInfo {
            name: "postgres".to_string(),
            create_db: true,
            create_role: true,
            inherit: true,
        },
    ];

    let roles = process_roles(&raw);
    assert_eq!(roles.len(), 2);

    let app_user = roles.iter().find(|r| r.name == "app_user").unwrap();
    assert_eq!(app_user.create_db, Some(false));
    assert_eq!(app_user.inherit, Some(true));

    let admin = roles.iter().find(|r| r.name == "admin").unwrap();
    assert_eq!(admin.create_role, Some(true));
}

// =============================================================================
// Enum Codegen Tests
// =============================================================================

#[test]
fn test_enum_codegen() {
    let mut ddl = PostgresDDL::new();

    // Add an enum type
    ddl.enums.push(Enum {
        schema: "public".to_string(),
        name: "order_status".to_string(),
        values: vec![
            "pending".to_string(),
            "processing".to_string(),
            "completed".to_string(),
            "cancelled".to_string(),
        ],
    });

    // Add a table that uses the enum
    ddl.tables.push(Table {
        schema: "public".to_string(),
        name: "orders".to_string(),
        is_rls_enabled: Some(false),
    });

    ddl.columns.push(Column {
        schema: "public".to_string(),
        table: "orders".to_string(),
        name: "id".to_string(),
        sql_type: "int4".to_string(),
        type_schema: None,
        not_null: true,
        default: None,
        generated: None,
        identity: Some(identity_always()),
        dimensions: None,
    });

    ddl.columns.push(Column {
        schema: "public".to_string(),
        table: "orders".to_string(),
        name: "status".to_string(),
        sql_type: "order_status".to_string(), // References the enum
        type_schema: Some("public".to_string()),
        not_null: true,
        default: None,
        generated: None,
        identity: None,
        dimensions: None,
    });

    ddl.columns.push(Column {
        schema: "public".to_string(),
        table: "orders".to_string(),
        name: "previous_status".to_string(),
        sql_type: "order_status".to_string(), // References the enum, nullable
        type_schema: Some("public".to_string()),
        not_null: false,
        default: None,
        generated: None,
        identity: None,
        dimensions: None,
    });

    ddl.pks.push(PrimaryKey {
        schema: "public".to_string(),
        table: "orders".to_string(),
        name: "orders_pkey".to_string(),
        name_explicit: true,
        columns: vec!["id".to_string()],
    });

    let options = CodegenOptions::default();
    let generated = generate_rust_schema(&ddl, &options);

    println!("Generated code with enum:\n{}", generated.code);

    // Verify enum was generated
    assert!(generated.enums.contains(&"order_status".to_string()));

    // Verify enum definition in generated code
    assert!(
        generated
            .code
            .contains("#[derive(PostgresEnum, Default, Clone, PartialEq, Debug)]")
    );
    assert!(generated.code.contains("enum OrderStatus {"));
    assert!(generated.code.contains("#[default]"));
    assert!(generated.code.contains("Pending,"));
    assert!(generated.code.contains("Completed,"));

    // Verify table uses the enum type
    assert!(generated.code.contains("status: OrderStatus,"));
    assert!(
        generated
            .code
            .contains("previous_status: Option<OrderStatus>,")
    );

    // Verify enum attribute is added for enum columns
    assert!(generated.code.contains("#[column(enum)]"));
}

#[test]
fn test_multiple_enums_codegen() {
    let mut ddl = PostgresDDL::new();

    // Add multiple enum types
    ddl.enums.push(Enum {
        schema: "public".to_string(),
        name: "priority".to_string(),
        values: vec!["low".to_string(), "medium".to_string(), "high".to_string()],
    });

    ddl.enums.push(Enum {
        schema: "public".to_string(),
        name: "task_type".to_string(),
        values: vec![
            "bug".to_string(),
            "feature".to_string(),
            "chore".to_string(),
        ],
    });

    // Add a table that uses both enums
    ddl.tables.push(Table {
        schema: "public".to_string(),
        name: "tasks".to_string(),
        is_rls_enabled: Some(false),
    });

    ddl.columns.push(Column {
        schema: "public".to_string(),
        table: "tasks".to_string(),
        name: "id".to_string(),
        sql_type: "int4".to_string(),
        type_schema: None,
        not_null: true,
        default: None,
        generated: None,
        identity: Some(identity_always()),
        dimensions: None,
    });

    ddl.columns.push(Column {
        schema: "public".to_string(),
        table: "tasks".to_string(),
        name: "priority".to_string(),
        sql_type: "priority".to_string(),
        type_schema: Some("public".to_string()),
        not_null: true,
        default: None,
        generated: None,
        identity: None,
        dimensions: None,
    });

    ddl.columns.push(Column {
        schema: "public".to_string(),
        table: "tasks".to_string(),
        name: "task_type".to_string(),
        sql_type: "task_type".to_string(),
        type_schema: Some("public".to_string()),
        not_null: true,
        default: None,
        generated: None,
        identity: None,
        dimensions: None,
    });

    ddl.pks.push(PrimaryKey {
        schema: "public".to_string(),
        table: "tasks".to_string(),
        name: "tasks_pkey".to_string(),
        name_explicit: true,
        columns: vec!["id".to_string()],
    });

    let options = CodegenOptions::default();
    let generated = generate_rust_schema(&ddl, &options);

    println!("Generated code with multiple enums:\n{}", generated.code);

    // Both enums should be generated
    assert_eq!(generated.enums.len(), 2);
    assert!(generated.enums.contains(&"priority".to_string()));
    assert!(generated.enums.contains(&"task_type".to_string()));

    // Verify both enum definitions
    assert!(generated.code.contains("enum Priority {"));
    assert!(generated.code.contains("enum TaskType {"));

    // Verify table uses both enum types
    assert!(generated.code.contains("priority: Priority,"));
    assert!(generated.code.contains("task_type: TaskType,"));
}

#[test]
fn test_enum_with_special_values() {
    let mut ddl = PostgresDDL::new();

    // Add an enum with values that need pascal case conversion
    ddl.enums.push(Enum {
        schema: "public".to_string(),
        name: "http_method".to_string(),
        values: vec![
            "GET".to_string(),
            "POST".to_string(),
            "PUT".to_string(),
            "DELETE".to_string(),
            "PATCH".to_string(),
        ],
    });

    let options = CodegenOptions::default();
    let generated = generate_rust_schema(&ddl, &options);

    println!("Generated enum with special values:\n{}", generated.code);

    // Verify enum is properly named
    assert!(generated.code.contains("enum HttpMethod {"));

    // Verify derives are correct
    assert!(
        generated
            .code
            .contains("#[derive(PostgresEnum, Default, Clone, PartialEq, Debug)]")
    );

    // Verify first variant has #[default]
    assert!(generated.code.contains("#[default]"));

    // Verify variant names are converted to PascalCase
    assert!(generated.code.contains("Get,"));
    assert!(generated.code.contains("Post,"));
    assert!(generated.code.contains("Delete,"));
}

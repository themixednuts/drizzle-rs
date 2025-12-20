//! Integration tests for PostgreSQL introspection and Rust schema code generation
//!
//! These tests verify that the PostgreSQL introspection and code generation
//! correctly handle various PostgreSQL-specific types and constraints.

use drizzle_migrations::{
    parser::SchemaParser,
    postgres::{
        PostgresDDL,
        codegen::{CodegenOptions, generate_rust_schema, sql_type_to_rust_type},
        ddl::{
            Column, Enum, ForeignKey, GeneratedType, Identity, IdentityType, Index, IndexColumn,
            PrimaryKey, Table, UniqueConstraint,
        },
        introspect::{
            RawColumnInfo, RawForeignKeyInfo, RawIndexColumnInfo, RawIndexInfo, RawPrimaryKeyInfo,
            RawTableInfo, RawUniqueInfo, process_columns, process_foreign_keys, process_indexes,
            process_primary_keys, process_tables, process_unique_constraints,
        },
    },
};
use drizzle_types::Dialect;
use std::{borrow::Cow, sync::OnceLock};

// =============================================================================
// Helper Functions
// =============================================================================

fn identity_always() -> Identity {
    Identity {
        name: Cow::Borrowed("test_seq"),
        schema: Some(Cow::Borrowed("public")),
        type_: IdentityType::Always,
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
        schema: Cow::Borrowed("public"),
        name: Cow::Borrowed("users"),
        is_rls_enabled: Some(false),
    });

    // Add columns for users
    ddl.columns.push(Column {
        schema: Cow::Borrowed("public"),
        table: Cow::Borrowed("users"),
        name: Cow::Borrowed("id"),
        sql_type: Cow::Borrowed("int4"),
        type_schema: None,
        not_null: true,
        default: None,
        generated: None,
        identity: Some(identity_always()),
        dimensions: None,
    });

    ddl.columns.push(Column {
        schema: Cow::Borrowed("public"),
        table: Cow::Borrowed("users"),
        name: Cow::Borrowed("email"),
        sql_type: Cow::Borrowed("text"),
        type_schema: None,
        not_null: true,
        default: None,
        generated: None,
        identity: None,
        dimensions: None,
    });

    ddl.columns.push(Column {
        schema: Cow::Borrowed("public"),
        table: Cow::Borrowed("users"),
        name: Cow::Borrowed("bio"),
        sql_type: Cow::Borrowed("text"),
        type_schema: None,
        not_null: false,
        default: None,
        generated: None,
        identity: None,
        dimensions: None,
    });

    // Add primary key for users
    ddl.pks.push(
        PrimaryKey::from_strings(
            "public".to_string(),
            "users".to_string(),
            "users_pkey".to_string(),
            vec!["id".to_string()],
        )
        .explicit_name(),
    );

    // Add unique constraint for email
    ddl.uniques.push(
        UniqueConstraint::from_strings(
            "public".to_string(),
            "users".to_string(),
            "users_email_key".to_string(),
            vec!["email".to_string()],
        )
        .explicit_name(),
    );

    // Add a posts table
    ddl.tables.push(Table {
        schema: Cow::Borrowed("public"),
        name: Cow::Borrowed("posts"),
        is_rls_enabled: Some(false),
    });

    ddl.columns.push(Column {
        schema: Cow::Borrowed("public"),
        table: Cow::Borrowed("posts"),
        name: Cow::Borrowed("id"),
        sql_type: Cow::Borrowed("int4"),
        type_schema: None,
        not_null: true,
        default: None,
        generated: None,
        identity: Some(identity_always()),
        dimensions: None,
    });

    ddl.columns.push(Column {
        schema: Cow::Borrowed("public"),
        table: Cow::Borrowed("posts"),
        name: Cow::Borrowed("title"),
        sql_type: Cow::Borrowed("text"),
        type_schema: None,
        not_null: true,
        default: None,
        generated: None,
        identity: None,
        dimensions: None,
    });

    ddl.columns.push(Column {
        schema: Cow::Borrowed("public"),
        table: Cow::Borrowed("posts"),
        name: Cow::Borrowed("author_id"),
        sql_type: Cow::Borrowed("int4"),
        type_schema: None,
        not_null: true,
        default: None,
        generated: None,
        identity: None,
        dimensions: None,
    });

    // Add primary key for posts
    ddl.pks.push(
        PrimaryKey::from_strings(
            "public".to_string(),
            "posts".to_string(),
            "posts_pkey".to_string(),
            vec!["id".to_string()],
        )
        .explicit_name(),
    );

    // Add foreign key from posts to users
    ddl.fks.push(ForeignKey {
        schema: Cow::Borrowed("public"),
        table: Cow::Borrowed("posts"),
        name: Cow::Borrowed("posts_author_id_fkey"),
        name_explicit: true,
        columns: Cow::Owned(vec![Cow::Borrowed("author_id")]),
        schema_to: Cow::Borrowed("public"),
        table_to: Cow::Borrowed("users"),
        columns_to: Cow::Owned(vec![Cow::Borrowed("id")]),
        on_update: Some(Cow::Borrowed("NO ACTION")),
        on_delete: Some(Cow::Borrowed("CASCADE")),
    });

    // Add an index
    ddl.indexes.push(Index {
        schema: Cow::Borrowed("public"),
        table: Cow::Borrowed("posts"),
        name: Cow::Borrowed("idx_posts_title"),
        name_explicit: false,
        columns: vec![IndexColumn::new("title")],
        is_unique: false,
        where_clause: None,
        method: Some(Cow::Borrowed("btree")),
        with: None,
        concurrently: false,
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
    assert!(generated.tables.contains(&"users".into()));
    assert!(generated.tables.contains(&"posts".into()));
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
    let users = parsed
        .table("Users", Dialect::PostgreSQL)
        .expect("Should have Users struct");
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

    let posts = parsed
        .table("Posts", Dialect::PostgreSQL)
        .expect("Should have Posts struct");

    let author_id = posts
        .field("author_id")
        .expect("Posts should have author_id field");

    // Check FK reference
    assert_eq!(
        author_id.references(),
        Some("Users::id".into()),
        "author_id should reference Users::id"
    );

    // Check on_delete cascade (since it's not NO ACTION)
    assert_eq!(
        author_id.on_delete(),
        Some("cascade".into()),
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
        .index("IdxPostsTitle", Dialect::PostgreSQL)
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
            schema: "public".into(),
            name: "users".into(),
            is_rls_enabled: false,
        },
        RawTableInfo {
            schema: "public".into(),
            name: "posts".into(),
            is_rls_enabled: true,
        },
        // System table should be filtered
        RawTableInfo {
            schema: "pg_catalog".into(),
            name: "pg_class".into(),
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
            schema: "public".into(),
            table: "users".into(),
            name: "id".into(),
            column_type: "int4".into(),
            type_schema: None,
            not_null: true,
            default_value: None,
            is_identity: true,
            identity_type: Some("ALWAYS".into()),
            is_generated: false,
            generated_expression: None,
            ordinal_position: 1,
        },
        RawColumnInfo {
            schema: "public".into(),
            table: "users".into(),
            name: "name".into(),
            column_type: "text".into(),
            type_schema: None,
            not_null: true,
            default_value: Some("'Anonymous'::text".into()),
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
    assert_eq!(name_col.default, Some("'Anonymous'::text".into()));
}

#[test]
fn test_process_indexes() {
    let raw = vec![
        RawIndexInfo {
            schema: "public".into(),
            table: "users".into(),
            name: "idx_users_email".into(),
            is_unique: true,
            is_primary: false,
            method: "btree".into(),
            columns: vec![RawIndexColumnInfo {
                name: "email".into(),
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
            schema: "public".into(),
            table: "users".into(),
            name: "users_pkey".into(),
            is_unique: true,
            is_primary: true,
            method: "btree".into(),
            columns: vec![RawIndexColumnInfo {
                name: "id".into(),
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
        schema: "public".into(),
        table: "posts".into(),
        name: "posts_author_id_fkey".into(),
        columns: vec!["author_id".into()],
        schema_to: "public".into(),
        table_to: "users".into(),
        columns_to: vec!["id".into()],
        on_update: "NO ACTION".into(),
        on_delete: "CASCADE".into(),
    }];

    static RAW_FKS: OnceLock<Vec<RawForeignKeyInfo>> = OnceLock::new();

    let fks = process_foreign_keys(RAW_FKS.get_or_init(|| raw));
    assert_eq!(fks.len(), 1);
    assert_eq!(fks[0].name, "posts_author_id_fkey");
    assert_eq!(fks[0].table_to, "users");
    assert_eq!(fks[0].on_delete, Some("CASCADE".into()));
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
    assert_eq!(pks[0].columns.len(), 1);
}

#[test]
fn test_process_unique_constraints() {
    let raw = vec![
        RawUniqueInfo {
            schema: "public".into(),
            table: "users".into(),
            name: "users_email_key".into(),
            columns: vec!["email".into()],
            nulls_not_distinct: false,
        },
        RawUniqueInfo {
            schema: "public".into(),
            table: "users".into(),
            name: "users_username_domain_key".into(),
            columns: vec!["username".into(), "domain".into()],
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
            schema: "public".into(),
            name: "status".into(),
            values: vec!["pending".into(), "active".into(), "completed".into()],
        },
        RawEnumInfo {
            schema: "public".into(),
            name: "priority".into(),
            values: vec!["low".into(), "medium".into(), "high".into()],
        },
        // System schema should be filtered
        RawEnumInfo {
            schema: "pg_catalog".into(),
            name: "anyenum".into(),
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
            schema: "public".into(),
            table: "products".into(),
            name: "products_price_check".into(),
            expression: "price > 0".into(),
        },
        RawCheckInfo {
            schema: "public".into(),
            table: "products".into(),
            name: "products_quantity_check".into(),
            expression: "quantity >= 0".into(),
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
        schema: "public".into(),
        name: "products".into(),
        is_rls_enabled: Some(false),
    });

    ddl.columns.push(Column {
        schema: "public".into(),
        table: "products".into(),
        name: "price".into(),
        sql_type: "numeric".into(),
        type_schema: None,
        not_null: true,
        default: None,
        generated: None,
        identity: None,
        dimensions: None,
    });

    ddl.columns.push(Column {
        schema: "public".into(),
        table: "products".into(),
        name: "quantity".into(),
        sql_type: "int4".into(),
        type_schema: None,
        not_null: true,
        default: None,
        generated: None,
        identity: None,
        dimensions: None,
    });

    ddl.columns.push(Column {
        schema: "public".into(),
        table: "products".into(),
        name: "total".into(),
        sql_type: "numeric".into(),
        type_schema: None,
        not_null: true,
        default: None,
        generated: Some(Generated {
            expression: "price * quantity".into(),
            gen_type: GeneratedType::Stored,
        }),
        identity: None,
        dimensions: None,
    });

    let options = CodegenOptions::default();
    let generated = generate_rust_schema(&ddl, &options);
    let parsed = SchemaParser::parse(&generated.code);

    // Verify table exists
    let table = parsed
        .table("Products", Dialect::PostgreSQL)
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
        schema: "public".into(),
        name: "settings".into(),
        is_rls_enabled: Some(false),
    });

    ddl.columns.push(Column {
        schema: "public".into(),
        table: "settings".into(),
        name: "enabled".into(),
        sql_type: "bool".into(),
        type_schema: None,
        not_null: true,
        default: Some("true".into()),
        generated: None,
        identity: None,
        dimensions: None,
    });

    ddl.columns.push(Column {
        schema: "public".into(),
        table: "settings".into(),
        name: "retries".into(),
        sql_type: "int4".into(),
        type_schema: None,
        not_null: true,
        default: Some("3".into()),
        generated: None,
        identity: None,
        dimensions: None,
    });

    ddl.columns.push(Column {
        schema: "public".into(),
        table: "settings".into(),
        name: "name".into(),
        sql_type: "text".into(),
        type_schema: None,
        not_null: true,
        default: Some("'default'::text".into()),
        generated: None,
        identity: None,
        dimensions: None,
    });

    let options = CodegenOptions::default();
    let generated = generate_rust_schema(&ddl, &options);
    let parsed = SchemaParser::parse(&generated.code);

    let settings = parsed
        .table("Settings", Dialect::PostgreSQL)
        .expect("Should have Settings");

    // Check boolean default
    let enabled = settings.field("enabled").unwrap();
    assert_eq!(enabled.default_value(), Some("true".into()));

    // Check numeric default
    let retries = settings.field("retries").unwrap();
    assert_eq!(retries.default_value(), Some("3".into()));

    // Check string default (should be quoted)
    let name = settings.field("name").unwrap();
    assert_eq!(name.default_value(), Some("\"default\"".into()));
}

// =============================================================================
// Identity Column Tests
// =============================================================================

#[test]
fn test_identity_column_types() {
    let mut ddl = PostgresDDL::new();

    ddl.tables.push(Table {
        schema: "public".into(),
        name: "test_identity".into(),
        is_rls_enabled: Some(false),
    });

    // Identity ALWAYS
    ddl.columns.push(Column {
        schema: "public".into(),
        table: "test_identity".into(),
        name: "id_always".into(),
        sql_type: "int4".into(),
        type_schema: None,
        not_null: true,
        default: None,
        generated: None,
        identity: Some(Identity {
            name: "id_always_seq".into(),
            schema: Some("public".into()),
            type_: IdentityType::Always,
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
        schema: "public".into(),
        table: "test_identity".into(),
        name: "id_by_default".into(),
        sql_type: "int4".into(),
        type_schema: None,
        not_null: true,
        default: None,
        generated: None,
        identity: Some(Identity {
            name: "id_by_default_seq".into(),
            schema: Some("public".into()),
            type_: IdentityType::ByDefault,
            increment: None,
            min_value: None,
            max_value: None,
            start_with: None,
            cache: None,
            cycle: None,
        }),
        dimensions: None,
    });

    ddl.pks.push(
        PrimaryKey::from_strings(
            "public".to_string(),
            "test_identity".to_string(),
            "test_identity_pkey".to_string(),
            vec!["id_always".to_string()],
        )
        .explicit_name(),
    );

    let options = CodegenOptions::default();
    let generated = generate_rust_schema(&ddl, &options);
    let parsed = SchemaParser::parse(&generated.code);

    let table = parsed
        .table("TestIdentity", Dialect::PostgreSQL)
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
        schema: "public".into(),
        name: "items".into(),
        is_rls_enabled: Some(false),
    });

    ddl.columns.push(Column {
        schema: "public".into(),
        table: "items".into(),
        name: "code".into(),
        sql_type: "text".into(),
        type_schema: None,
        not_null: true,
        default: None,
        generated: None,
        identity: None,
        dimensions: None,
    });

    // Unique index
    ddl.indexes.push(Index {
        schema: "public".into(),
        table: "items".into(),
        name: "idx_items_code_unique".into(),
        columns: vec![IndexColumn::new("code")],
        is_unique: true,
        name_explicit: true,
        where_clause: None,
        with: None,
        method: Some("btree".into()),
        concurrently: false,
    });

    let options = CodegenOptions::default();
    let generated = generate_rust_schema(&ddl, &options);
    let parsed = SchemaParser::parse(&generated.code);

    let idx = parsed
        .index("IdxItemsCodeUnique", Dialect::PostgreSQL)
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
            schema: "public".into(),
            name: "users_id_seq".into(),
            data_type: "bigint".into(),
            start_value: "1".into(),
            min_value: "1".into(),
            max_value: "9223372036854775807".into(),
            increment: "1".into(),
            cycle: false,
            cache_value: "1".into(),
        },
        RawSequenceInfo {
            schema: "public".into(),
            name: "order_num_seq".into(),
            data_type: "integer".into(),
            start_value: "1000".into(),
            min_value: "1000".into(),
            max_value: "2147483647".into(),
            increment: "1".into(),
            cycle: true,
            cache_value: "10".into(),
        },
    ];

    let sequences = process_sequences(&raw);
    assert_eq!(sequences.len(), 2);

    let users_seq = sequences.iter().find(|s| s.name == "users_id_seq").unwrap();
    assert_eq!(users_seq.start_with, Some("1".into()));
    assert_eq!(users_seq.cycle, Some(false));

    let order_seq = sequences
        .iter()
        .find(|s| s.name == "order_num_seq")
        .unwrap();
    assert_eq!(order_seq.start_with, Some("1000".into()));
    assert_eq!(order_seq.cycle, Some(true));
    assert_eq!(order_seq.cache_size, Some(10));
}

// =============================================================================
// Schema Generation Tests
// =============================================================================

#[test]
fn test_schema_struct_generation() {
    let ddl = create_test_ddl();
    let options = CodegenOptions {
        include_schema: true,
        schema_name: "AppSchema".into(),
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
            name: "app_user".into(),
            create_db: false,
            create_role: false,
            inherit: true,
        },
        RawRoleInfo {
            name: "admin".into(),
            create_db: true,
            create_role: true,
            inherit: true,
        },
        // System roles should be filtered
        RawRoleInfo {
            name: "postgres".into(),
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
    ddl.enums.push(Enum::from_strings(
        "public".to_string(),
        "order_status".to_string(),
        vec![
            "pending".to_string(),
            "processing".to_string(),
            "completed".to_string(),
            "cancelled".to_string(),
        ],
    ));

    // Add a table that uses the enum
    ddl.tables.push(Table {
        schema: "public".into(),
        name: "orders".into(),
        is_rls_enabled: Some(false),
    });

    ddl.columns.push(Column {
        schema: "public".into(),
        table: "orders".into(),
        name: "id".into(),
        sql_type: "int4".into(),
        type_schema: None,
        not_null: true,
        default: None,
        generated: None,
        identity: Some(identity_always()),
        dimensions: None,
    });

    ddl.columns.push(Column {
        schema: "public".into(),
        table: "orders".into(),
        name: "status".into(),
        sql_type: "order_status".into(), // References the enum
        type_schema: Some("public".into()),
        not_null: true,
        default: None,
        generated: None,
        identity: None,
        dimensions: None,
    });

    ddl.columns.push(Column {
        schema: "public".into(),
        table: "orders".into(),
        name: "previous_status".into(),
        sql_type: "order_status".into(), // References the enum, nullable
        type_schema: Some("public".into()),
        not_null: false,
        default: None,
        generated: None,
        identity: None,
        dimensions: None,
    });

    ddl.pks.push(
        PrimaryKey::from_strings(
            "public".to_string(),
            "orders".to_string(),
            "orders_pkey".to_string(),
            vec!["id".to_string()],
        )
        .explicit_name(),
    );

    let options = CodegenOptions::default();
    let generated = generate_rust_schema(&ddl, &options);

    println!("Generated code with enum:\n{}", generated.code);

    // Verify enum was generated
    assert!(generated.enums.contains(&"order_status".into()));

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
    ddl.enums.push(Enum::from_strings(
        "public".to_string(),
        "priority".to_string(),
        vec!["low".to_string(), "medium".to_string(), "high".to_string()],
    ));

    ddl.enums.push(Enum::from_strings(
        "public".to_string(),
        "task_type".to_string(),
        vec![
            "bug".to_string(),
            "feature".to_string(),
            "chore".to_string(),
        ],
    ));

    // Add a table that uses both enums
    ddl.tables.push(Table {
        schema: "public".into(),
        name: "tasks".into(),
        is_rls_enabled: Some(false),
    });

    ddl.columns.push(Column {
        schema: "public".into(),
        table: "tasks".into(),
        name: "id".into(),
        sql_type: "int4".into(),
        type_schema: None,
        not_null: true,
        default: None,
        generated: None,
        identity: Some(identity_always()),
        dimensions: None,
    });

    ddl.columns.push(Column {
        schema: "public".into(),
        table: "tasks".into(),
        name: "priority".into(),
        sql_type: "priority".into(),
        type_schema: Some("public".into()),
        not_null: true,
        default: None,
        generated: None,
        identity: None,
        dimensions: None,
    });

    ddl.columns.push(Column {
        schema: "public".into(),
        table: "tasks".into(),
        name: "task_type".into(),
        sql_type: "task_type".into(),
        type_schema: Some("public".into()),
        not_null: true,
        default: None,
        generated: None,
        identity: None,
        dimensions: None,
    });

    ddl.pks.push(
        PrimaryKey::from_strings(
            "public".to_string(),
            "tasks".to_string(),
            "tasks_pkey".to_string(),
            vec!["id".to_string()],
        )
        .explicit_name(),
    );

    let options = CodegenOptions::default();
    let generated = generate_rust_schema(&ddl, &options);

    println!("Generated code with multiple enums:\n{}", generated.code);

    // Both enums should be generated
    assert_eq!(generated.enums.len(), 2);
    assert!(generated.enums.contains(&"priority".into()));
    assert!(generated.enums.contains(&"task_type".into()));

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
    ddl.enums.push(Enum::from_strings(
        "public".to_string(),
        "http_method".to_string(),
        vec![
            "GET".to_string(),
            "POST".to_string(),
            "PUT".to_string(),
            "DELETE".to_string(),
            "PATCH".to_string(),
        ],
    ));

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

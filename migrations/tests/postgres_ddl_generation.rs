//! PostgreSQL DDL generation tests
//!
//! These tests verify that the PostgreSQL migration generator correctly generates
//! SQL statements for CREATE TABLE, DROP TABLE, indexes, constraints, etc.
//! They mirror the test patterns from drizzle-orm's drizzle-kit tests.

use drizzle_migrations::postgres::{
    PostgresDDL,
    collection::diff_ddl,
    ddl::{
        Column, Enum, ForeignKey, Generated, GeneratedType, Index, IndexColumn, PrimaryKey, Table,
        UniqueConstraint,
    },
    statements::PostgresGenerator,
};
use std::borrow::Cow;

// =============================================================================
// Helper Functions
// =============================================================================

/// Generate SQL statements from the diff between two DDL states
fn diff_to_sql(from: &PostgresDDL, to: &PostgresDDL) -> Vec<String> {
    let diffs = diff_ddl(from, to);
    let generator = PostgresGenerator::new().with_breakpoints(false);
    generator.generate(&diffs)
}

/// Helper to create a basic column
fn column(table: &str, name: &str, sql_type: &str) -> Column {
    Column {
        schema: Cow::Borrowed("public"),
        table: Cow::Owned(table.to_string()),
        name: Cow::Owned(name.to_string()),
        sql_type: Cow::Owned(sql_type.to_string()),
        type_schema: None,
        not_null: false,
        default: None,
        generated: None,
        identity: None,
        dimensions: None,
    }
}

/// Helper to create a NOT NULL column
fn column_not_null(table: &str, name: &str, sql_type: &str) -> Column {
    Column {
        not_null: true,
        ..column(table, name, sql_type)
    }
}

/// Helper to create a table
fn table(name: &str) -> Table {
    Table {
        schema: Cow::Borrowed("public"),
        name: Cow::Owned(name.to_string()),
        is_rls_enabled: None,
    }
}

/// Helper to create a primary key
fn primary_key(table_name: &str, columns: Vec<&str>) -> PrimaryKey {
    let pk_name = format!("{}_pkey", table_name);
    PrimaryKey::from_strings(
        "public".to_string(),
        table_name.to_string(),
        pk_name,
        columns.into_iter().map(|s| s.to_string()).collect(),
    )
}

/// Helper to create a foreign key
fn foreign_key(
    table_name: &str,
    name: &str,
    columns: Vec<&str>,
    ref_table: &str,
    ref_columns: Vec<&str>,
) -> ForeignKey {
    ForeignKey::from_strings(
        "public".to_string(),
        table_name.to_string(),
        name.to_string(),
        columns.into_iter().map(|s| s.to_string()).collect(),
        "public".to_string(),
        ref_table.to_string(),
        ref_columns.into_iter().map(|s| s.to_string()).collect(),
    )
}

/// Helper to create an index
fn index(table_name: &str, name: &str, columns: Vec<&str>) -> Index {
    Index {
        schema: Cow::Borrowed("public"),
        table: Cow::Owned(table_name.to_string()),
        name: Cow::Owned(name.to_string()),
        name_explicit: false,
        columns: columns
            .into_iter()
            .map(|c| IndexColumn {
                value: Cow::Owned(c.to_string()),
                is_expression: false,
                asc: true,
                nulls_first: false,
                opclass: None,
            })
            .collect(),
        method: None,
        is_unique: false,
        concurrently: false,
        where_clause: None,
        with: None,
    }
}

/// Helper to create a unique index
fn unique_index(table_name: &str, name: &str, columns: Vec<&str>) -> Index {
    Index {
        is_unique: true,
        ..index(table_name, name, columns)
    }
}

/// Helper to create a unique constraint
fn unique_constraint(table_name: &str, name: &str, columns: Vec<&str>) -> UniqueConstraint {
    UniqueConstraint::from_strings(
        "public".to_string(),
        table_name.to_string(),
        name.to_string(),
        columns.into_iter().map(|s| s.to_string()).collect(),
    )
}

// =============================================================================
// CREATE TABLE Tests
// =============================================================================

#[test]
fn test_create_table_basic() {
    let from = PostgresDDL::new();
    let mut to = PostgresDDL::new();

    to.tables.push(table("users"));
    to.columns.push(column_not_null("users", "id", "integer"));
    to.columns.push(column_not_null("users", "name", "text"));

    let sql = diff_to_sql(&from, &to);

    assert_eq!(sql.len(), 1);
    assert!(sql[0].contains("CREATE TABLE \"users\""));
    assert!(sql[0].contains("\"id\" integer NOT NULL"));
    assert!(sql[0].contains("\"name\" text NOT NULL"));
}

#[test]
fn test_create_table_with_primary_key() {
    let from = PostgresDDL::new();
    let mut to = PostgresDDL::new();

    to.tables.push(table("users"));
    to.columns.push(column_not_null("users", "id", "integer"));
    to.columns.push(column_not_null("users", "name", "text"));
    to.pks.push(primary_key("users", vec!["id"]));

    let sql = diff_to_sql(&from, &to);

    assert_eq!(sql.len(), 1);
    assert!(sql[0].contains("CREATE TABLE \"users\""));
    assert!(sql[0].contains("PRIMARY KEY(\"id\")"));
}

#[test]
fn test_create_table_composite_pk() {
    let from = PostgresDDL::new();
    let mut to = PostgresDDL::new();

    to.tables.push(table("order_items"));
    to.columns
        .push(column_not_null("order_items", "order_id", "integer"));
    to.columns
        .push(column_not_null("order_items", "product_id", "integer"));
    to.columns
        .push(column_not_null("order_items", "quantity", "integer"));

    let pk = PrimaryKey::from_strings(
        "public".to_string(),
        "order_items".to_string(),
        "order_items_pkey".to_string(),
        vec!["order_id".to_string(), "product_id".to_string()],
    );
    to.pks.push(pk);

    let sql = diff_to_sql(&from, &to);

    assert_eq!(sql.len(), 1);
    assert!(sql[0].contains("CREATE TABLE \"order_items\""));
    assert!(sql[0].contains("PRIMARY KEY(\"order_id\", \"product_id\")"));
}

#[test]
fn test_create_table_with_foreign_key() {
    let from = PostgresDDL::new();
    let mut to = PostgresDDL::new();

    // Create users table
    to.tables.push(table("users"));
    to.columns.push(column_not_null("users", "id", "integer"));
    to.pks.push(primary_key("users", vec!["id"]));

    // Create posts table with foreign key
    to.tables.push(table("posts"));
    to.columns.push(column_not_null("posts", "id", "integer"));
    to.columns
        .push(column_not_null("posts", "author_id", "integer"));
    to.pks.push(primary_key("posts", vec!["id"]));
    to.fks.push(foreign_key(
        "posts",
        "posts_author_fk",
        vec!["author_id"],
        "users",
        vec!["id"],
    ));

    let sql = diff_to_sql(&from, &to);

    assert_eq!(sql.len(), 2);
    let posts_sql = sql
        .iter()
        .find(|s| s.contains("CREATE TABLE \"posts\""))
        .unwrap();
    assert!(posts_sql.contains("FOREIGN KEY (\"author_id\") REFERENCES \"users\"(\"id\")"));
}

#[test]
fn test_foreign_key_on_delete_cascade() {
    let from = PostgresDDL::new();
    let mut to = PostgresDDL::new();

    to.tables.push(table("users"));
    to.columns.push(column_not_null("users", "id", "integer"));
    to.pks.push(primary_key("users", vec!["id"]));

    to.tables.push(table("posts"));
    to.columns.push(column_not_null("posts", "id", "integer"));
    to.columns
        .push(column_not_null("posts", "author_id", "integer"));
    to.pks.push(primary_key("posts", vec!["id"]));

    let mut fk = foreign_key(
        "posts",
        "posts_author_fk",
        vec!["author_id"],
        "users",
        vec!["id"],
    );
    fk.on_delete = Some(Cow::Borrowed("CASCADE"));
    to.fks.push(fk);

    let sql = diff_to_sql(&from, &to);

    let posts_sql = sql
        .iter()
        .find(|s| s.contains("CREATE TABLE \"posts\""))
        .unwrap();
    assert!(posts_sql.contains("ON DELETE CASCADE"));
}

#[test]
fn test_create_table_with_unique_constraint() {
    let from = PostgresDDL::new();
    let mut to = PostgresDDL::new();

    to.tables.push(table("users"));
    to.columns.push(column_not_null("users", "id", "integer"));
    to.columns.push(column_not_null("users", "email", "text"));
    to.pks.push(primary_key("users", vec!["id"]));
    to.uniques.push(unique_constraint(
        "users",
        "users_email_unique",
        vec!["email"],
    ));

    let sql = diff_to_sql(&from, &to);

    assert_eq!(sql.len(), 1);
    assert!(sql[0].contains("CONSTRAINT \"users_email_unique\" UNIQUE(\"email\")"));
}

#[test]
fn test_create_table_with_default() {
    let from = PostgresDDL::new();
    let mut to = PostgresDDL::new();

    to.tables.push(table("users"));
    to.columns.push(column_not_null("users", "id", "integer"));
    let mut col = column("users", "status", "text");
    col.default = Some(Cow::Borrowed("'active'"));
    to.columns.push(col);

    let sql = diff_to_sql(&from, &to);

    assert_eq!(sql.len(), 1);
    assert!(sql[0].contains("\"status\" text DEFAULT 'active'"));
}

// =============================================================================
// DROP TABLE Tests
// =============================================================================

#[test]
fn test_drop_table() {
    let mut from = PostgresDDL::new();
    let to = PostgresDDL::new();

    from.tables.push(table("users"));
    from.columns.push(column_not_null("users", "id", "integer"));

    let sql = diff_to_sql(&from, &to);

    assert_eq!(sql.len(), 1);
    assert!(sql[0].contains("DROP TABLE \"users\""));
}

#[test]
fn test_drop_and_create_table() {
    let mut from = PostgresDDL::new();
    let mut to = PostgresDDL::new();

    // Drop old_table
    from.tables.push(table("old_table"));
    from.columns
        .push(column_not_null("old_table", "id", "integer"));

    // Create new_table
    to.tables.push(table("new_table"));
    to.columns
        .push(column_not_null("new_table", "id", "integer"));

    let sql = diff_to_sql(&from, &to);

    assert_eq!(sql.len(), 2);
    assert!(sql.iter().any(|s| s.contains("DROP TABLE \"old_table\"")));
    assert!(sql.iter().any(|s| s.contains("CREATE TABLE \"new_table\"")));
}

// =============================================================================
// INDEX Tests
// =============================================================================

#[test]
fn test_create_index() {
    let mut from = PostgresDDL::new();
    let mut to = PostgresDDL::new();

    // Table exists in both
    from.tables.push(table("users"));
    from.columns.push(column_not_null("users", "id", "integer"));
    from.columns.push(column_not_null("users", "email", "text"));

    to.tables.push(table("users"));
    to.columns.push(column_not_null("users", "id", "integer"));
    to.columns.push(column_not_null("users", "email", "text"));

    // New index in 'to'
    to.indexes
        .push(index("users", "users_email_idx", vec!["email"]));

    let sql = diff_to_sql(&from, &to);

    assert_eq!(sql.len(), 1);
    assert!(sql[0].contains("CREATE INDEX \"users_email_idx\" ON \"users\""));
}

#[test]
fn test_create_unique_index() {
    let mut from = PostgresDDL::new();
    let mut to = PostgresDDL::new();

    from.tables.push(table("users"));
    from.columns.push(column_not_null("users", "id", "integer"));
    from.columns.push(column_not_null("users", "email", "text"));

    to.tables.push(table("users"));
    to.columns.push(column_not_null("users", "id", "integer"));
    to.columns.push(column_not_null("users", "email", "text"));
    to.indexes.push(unique_index(
        "users",
        "users_email_unique_idx",
        vec!["email"],
    ));

    let sql = diff_to_sql(&from, &to);

    assert_eq!(sql.len(), 1);
    assert!(sql[0].contains("CREATE UNIQUE INDEX \"users_email_unique_idx\""));
}

#[test]
fn test_drop_index() {
    let mut from = PostgresDDL::new();
    let mut to = PostgresDDL::new();

    from.tables.push(table("users"));
    from.columns.push(column_not_null("users", "id", "integer"));
    from.columns.push(column_not_null("users", "email", "text"));
    from.indexes
        .push(index("users", "users_email_idx", vec!["email"]));

    to.tables.push(table("users"));
    to.columns.push(column_not_null("users", "id", "integer"));
    to.columns.push(column_not_null("users", "email", "text"));

    let sql = diff_to_sql(&from, &to);

    assert_eq!(sql.len(), 1);
    assert!(sql[0].contains("DROP INDEX \"users_email_idx\""));
}

// =============================================================================
// COLUMN Tests
// =============================================================================

#[test]
fn test_add_column() {
    let mut from = PostgresDDL::new();
    let mut to = PostgresDDL::new();

    from.tables.push(table("users"));
    from.columns.push(column_not_null("users", "id", "integer"));

    to.tables.push(table("users"));
    to.columns.push(column_not_null("users", "id", "integer"));
    to.columns.push(column_not_null("users", "email", "text"));

    let sql = diff_to_sql(&from, &to);

    assert_eq!(sql.len(), 1);
    assert!(sql[0].contains("ALTER TABLE \"users\" ADD COLUMN \"email\" text NOT NULL"));
}

#[test]
fn test_drop_column() {
    let mut from = PostgresDDL::new();
    let mut to = PostgresDDL::new();

    from.tables.push(table("users"));
    from.columns.push(column_not_null("users", "id", "integer"));
    from.columns.push(column_not_null("users", "email", "text"));

    to.tables.push(table("users"));
    to.columns.push(column_not_null("users", "id", "integer"));

    let sql = diff_to_sql(&from, &to);

    assert_eq!(sql.len(), 1);
    assert!(sql[0].contains("ALTER TABLE \"users\" DROP COLUMN \"email\""));
}

// =============================================================================
// ENUM Tests
// =============================================================================

#[test]
fn test_create_enum() {
    let from = PostgresDDL::new();
    let mut to = PostgresDDL::new();

    to.enums.push(Enum {
        schema: Cow::Borrowed("public"),
        name: Cow::Borrowed("status"),
        values: Cow::Borrowed(&[
            Cow::Borrowed("active"),
            Cow::Borrowed("inactive"),
            Cow::Borrowed("pending"),
        ]),
    });

    let sql = diff_to_sql(&from, &to);

    assert_eq!(sql.len(), 1);
    assert!(sql[0].contains("CREATE TYPE \"status\" AS ENUM"));
    assert!(sql[0].contains("'active'"));
    assert!(sql[0].contains("'inactive'"));
    assert!(sql[0].contains("'pending'"));
}

#[test]
fn test_drop_enum() {
    let mut from = PostgresDDL::new();
    let to = PostgresDDL::new();

    from.enums.push(Enum {
        schema: Cow::Borrowed("public"),
        name: Cow::Borrowed("status"),
        values: Cow::Borrowed(&[Cow::Borrowed("active"), Cow::Borrowed("inactive")]),
    });

    let sql = diff_to_sql(&from, &to);

    assert_eq!(sql.len(), 1);
    assert!(sql[0].contains("DROP TYPE \"status\""));
}

// =============================================================================
// Column Types Tests
// =============================================================================

#[test]
fn test_column_types() {
    let from = PostgresDDL::new();
    let mut to = PostgresDDL::new();

    to.tables.push(table("all_types"));
    to.columns
        .push(column_not_null("all_types", "id", "serial"));
    to.columns
        .push(column_not_null("all_types", "small", "smallint"));
    to.columns
        .push(column_not_null("all_types", "big", "bigint"));
    to.columns.push(column("all_types", "real_val", "real"));
    to.columns
        .push(column("all_types", "double_val", "double precision"));
    to.columns.push(column("all_types", "text_val", "text"));
    to.columns
        .push(column("all_types", "varchar_val", "varchar(255)"));
    to.columns.push(column("all_types", "char_val", "char(10)"));
    to.columns.push(column("all_types", "bool_val", "boolean"));
    to.columns
        .push(column("all_types", "timestamp_val", "timestamp"));
    to.columns
        .push(column("all_types", "timestamptz_val", "timestamptz"));
    to.columns.push(column("all_types", "date_val", "date"));
    to.columns.push(column("all_types", "time_val", "time"));
    to.columns.push(column("all_types", "json_val", "json"));
    to.columns.push(column("all_types", "jsonb_val", "jsonb"));
    to.columns.push(column("all_types", "uuid_val", "uuid"));

    let sql = diff_to_sql(&from, &to);

    assert_eq!(sql.len(), 1);
    assert!(sql[0].contains("\"id\" serial NOT NULL"));
    assert!(sql[0].contains("\"small\" smallint NOT NULL"));
    assert!(sql[0].contains("\"big\" bigint NOT NULL"));
    assert!(sql[0].contains("\"real_val\" real"));
    assert!(sql[0].contains("\"double_val\" double precision"));
    assert!(sql[0].contains("\"text_val\" text"));
    assert!(sql[0].contains("\"varchar_val\" varchar(255)"));
    assert!(sql[0].contains("\"char_val\" char(10)"));
    assert!(sql[0].contains("\"bool_val\" boolean"));
    assert!(sql[0].contains("\"timestamp_val\" timestamp"));
    assert!(sql[0].contains("\"timestamptz_val\" timestamptz"));
    assert!(sql[0].contains("\"date_val\" date"));
    assert!(sql[0].contains("\"time_val\" time"));
    assert!(sql[0].contains("\"json_val\" json"));
    assert!(sql[0].contains("\"jsonb_val\" jsonb"));
    assert!(sql[0].contains("\"uuid_val\" uuid"));
}

// =============================================================================
// No-Diff Tests
// =============================================================================

#[test]
fn test_no_diff_for_identical_schemas() {
    let mut from = PostgresDDL::new();
    let mut to = PostgresDDL::new();

    // Same schema in both
    from.tables.push(table("users"));
    from.columns.push(column_not_null("users", "id", "integer"));
    from.columns.push(column_not_null("users", "name", "text"));
    from.pks.push(primary_key("users", vec!["id"]));

    to.tables.push(table("users"));
    to.columns.push(column_not_null("users", "id", "integer"));
    to.columns.push(column_not_null("users", "name", "text"));
    to.pks.push(primary_key("users", vec!["id"]));

    let sql = diff_to_sql(&from, &to);

    assert!(sql.is_empty(), "Expected no diff for identical schemas");
}

// =============================================================================
// Schema Tests
// =============================================================================

#[test]
fn test_create_table_in_custom_schema() {
    let from = PostgresDDL::new();
    let mut to = PostgresDDL::new();

    // Create table in custom schema
    to.tables.push(Table {
        schema: Cow::Borrowed("myschema"),
        name: Cow::Borrowed("users"),
        is_rls_enabled: None,
    });
    to.columns.push(Column {
        schema: Cow::Borrowed("myschema"),
        table: Cow::Borrowed("users"),
        name: Cow::Borrowed("id"),
        sql_type: Cow::Borrowed("integer"),
        type_schema: None,
        not_null: true,
        default: None,
        generated: None,
        identity: None,
        dimensions: None,
    });

    let sql = diff_to_sql(&from, &to);

    assert_eq!(sql.len(), 1);
    assert!(sql[0].contains("CREATE TABLE \"myschema\".\"users\""));
}

#[test]
fn test_create_multiple_tables() {
    let from = PostgresDDL::new();
    let mut to = PostgresDDL::new();

    to.tables.push(table("users"));
    to.columns.push(column_not_null("users", "id", "integer"));
    to.pks.push(primary_key("users", vec!["id"]));

    to.tables.push(table("posts"));
    to.columns.push(column_not_null("posts", "id", "integer"));
    to.pks.push(primary_key("posts", vec!["id"]));

    to.tables.push(table("comments"));
    to.columns
        .push(column_not_null("comments", "id", "integer"));
    to.pks.push(primary_key("comments", vec!["id"]));

    let sql = diff_to_sql(&from, &to);

    assert_eq!(sql.len(), 3);
    assert!(sql.iter().any(|s| s.contains("CREATE TABLE \"users\"")));
    assert!(sql.iter().any(|s| s.contains("CREATE TABLE \"posts\"")));
    assert!(sql.iter().any(|s| s.contains("CREATE TABLE \"comments\"")));
}

// =============================================================================
// Self-Referencing Foreign Key Tests
// =============================================================================

#[test]
fn test_self_referencing_fk() {
    let from = PostgresDDL::new();
    let mut to = PostgresDDL::new();

    to.tables.push(table("categories"));
    to.columns
        .push(column_not_null("categories", "id", "integer"));
    to.columns
        .push(column("categories", "parent_id", "integer"));
    to.pks.push(primary_key("categories", vec!["id"]));
    to.fks.push(foreign_key(
        "categories",
        "categories_parent_fk",
        vec!["parent_id"],
        "categories",
        vec!["id"],
    ));

    let sql = diff_to_sql(&from, &to);

    assert_eq!(sql.len(), 1);
    assert!(sql[0].contains("REFERENCES \"categories\"(\"id\")"));
}

// =============================================================================
// Generated Column Tests
// =============================================================================

/// Test adding a generated expression to an existing column uses RecreateColumn
#[test]
fn test_add_generated_column_expression() {
    let mut from = PostgresDDL::new();
    let mut to = PostgresDDL::new();

    // From: table with a regular column
    from.tables.push(table("users"));
    from.columns.push(column_not_null("users", "id", "integer"));
    from.columns.push(column("users", "first_name", "text"));
    from.columns.push(column("users", "last_name", "text"));
    from.columns.push(column("users", "full_name", "text")); // Regular column
    from.pks.push(primary_key("users", vec!["id"]));

    // To: same table but full_name is now a generated column
    to.tables.push(table("users"));
    to.columns.push(column_not_null("users", "id", "integer"));
    to.columns.push(column("users", "first_name", "text"));
    to.columns.push(column("users", "last_name", "text"));

    // full_name as generated column
    let mut full_name_col = column("users", "full_name", "text");
    full_name_col.generated = Some(Generated {
        expression: Cow::Borrowed("first_name || ' ' || last_name"),
        gen_type: GeneratedType::Stored,
    });
    to.columns.push(full_name_col);
    to.pks.push(primary_key("users", vec!["id"]));

    let sql = diff_to_sql(&from, &to);
    let all_sql = sql.join("\n");

    // Should have DROP COLUMN and ADD COLUMN for the recreate
    assert!(
        all_sql.contains("DROP COLUMN \"full_name\""),
        "Expected DROP COLUMN for recreating generated column, got:\n{}",
        all_sql
    );
    assert!(
        all_sql.contains("ADD COLUMN \"full_name\""),
        "Expected ADD COLUMN for recreating generated column, got:\n{}",
        all_sql
    );
    assert!(
        all_sql.contains("GENERATED ALWAYS AS"),
        "Expected GENERATED ALWAYS AS in the new column definition, got:\n{}",
        all_sql
    );
}

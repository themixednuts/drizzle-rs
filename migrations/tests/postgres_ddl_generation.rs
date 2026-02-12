//! PostgreSQL DDL generation tests
//!
//! These tests verify that the PostgreSQL migration generator correctly generates
//! SQL statements for CREATE TABLE, DROP TABLE, indexes, constraints, etc.
//! They mirror the test patterns from drizzle-orm's drizzle-kit tests.

use drizzle_migrations::postgres::{
    PostgresDDL,
    collection::diff_ddl,
    ddl::{
        Column, Enum, ForeignKey, Generated, GeneratedType, Index, IndexColumn, Policy, PrimaryKey,
        Table, UniqueConstraint,
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
        ordinal_position: None,
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
    // Column order may vary
    let sql_str = &sql[0];
    assert!(sql_str.starts_with("CREATE TABLE \"users\" ("));
    assert!(sql_str.contains("\"id\" integer NOT NULL"));
    assert!(sql_str.contains("\"name\" text NOT NULL"));
    assert!(sql_str.ends_with(");"));
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
    // Column order may vary, PK rendered without CONSTRAINT prefix
    let sql_str = &sql[0];
    assert!(sql_str.starts_with("CREATE TABLE \"users\" ("));
    assert!(sql_str.contains("\"id\" integer NOT NULL"));
    assert!(sql_str.contains("\"name\" text NOT NULL"));
    assert!(sql_str.contains("PRIMARY KEY(\"id\")"));
    assert!(sql_str.ends_with(");"));
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
    // Column order may vary, but PK column order should be preserved
    let sql_str = &sql[0];
    assert!(sql_str.starts_with("CREATE TABLE \"order_items\" ("));
    assert!(sql_str.contains("\"order_id\" integer NOT NULL"));
    assert!(sql_str.contains("\"product_id\" integer NOT NULL"));
    assert!(sql_str.contains("\"quantity\" integer NOT NULL"));
    assert!(sql_str.contains("PRIMARY KEY(\"order_id\", \"product_id\")"));
    assert!(sql_str.ends_with(");"));
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

    // Find the posts SQL (which has FK)
    let posts_sql = sql
        .iter()
        .find(|s| s.contains("CREATE TABLE \"posts\""))
        .unwrap();
    let users_sql = sql
        .iter()
        .find(|s| s.contains("CREATE TABLE \"users\""))
        .unwrap();

    assert_eq!(
        *users_sql,
        "CREATE TABLE \"users\" (\n\t\"id\" integer NOT NULL,\n\tPRIMARY KEY(\"id\")\n);",
        "Unexpected users table SQL"
    );
    // Column order may vary
    let expected_v1 = "CREATE TABLE \"posts\" (\n\t\"id\" integer NOT NULL,\n\t\"author_id\" integer NOT NULL,\n\tPRIMARY KEY(\"id\"),\n\tCONSTRAINT \"posts_author_fk\" FOREIGN KEY (\"author_id\") REFERENCES \"users\"(\"id\")\n);";
    let expected_v2 = "CREATE TABLE \"posts\" (\n\t\"author_id\" integer NOT NULL,\n\t\"id\" integer NOT NULL,\n\tPRIMARY KEY(\"id\"),\n\tCONSTRAINT \"posts_author_fk\" FOREIGN KEY (\"author_id\") REFERENCES \"users\"(\"id\")\n);";
    assert!(
        *posts_sql == expected_v1 || *posts_sql == expected_v2,
        "Unexpected posts table SQL with FK: {}",
        posts_sql
    );
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

    // Column order may vary
    let expected_v1 = "CREATE TABLE \"posts\" (\n\t\"id\" integer NOT NULL,\n\t\"author_id\" integer NOT NULL,\n\tPRIMARY KEY(\"id\"),\n\tCONSTRAINT \"posts_author_fk\" FOREIGN KEY (\"author_id\") REFERENCES \"users\"(\"id\") ON DELETE CASCADE\n);";
    let expected_v2 = "CREATE TABLE \"posts\" (\n\t\"author_id\" integer NOT NULL,\n\t\"id\" integer NOT NULL,\n\tPRIMARY KEY(\"id\"),\n\tCONSTRAINT \"posts_author_fk\" FOREIGN KEY (\"author_id\") REFERENCES \"users\"(\"id\") ON DELETE CASCADE\n);";
    assert!(
        *posts_sql == expected_v1 || *posts_sql == expected_v2,
        "Unexpected FK with CASCADE SQL: {}",
        posts_sql
    );
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
    // Column order may vary
    let sql_str = &sql[0];
    assert!(sql_str.starts_with("CREATE TABLE \"users\" ("));
    assert!(sql_str.contains("\"id\" integer NOT NULL"));
    assert!(sql_str.contains("\"email\" text NOT NULL"));
    assert!(sql_str.contains("PRIMARY KEY(\"id\")"));
    assert!(sql_str.contains("CONSTRAINT \"users_email_unique\" UNIQUE(\"email\")"));
    assert!(sql_str.ends_with(");"));
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
    // Column order may vary
    let sql_str = &sql[0];
    assert!(sql_str.starts_with("CREATE TABLE \"users\" ("));
    assert!(sql_str.contains("\"id\" integer NOT NULL"));
    assert!(sql_str.contains("\"status\" text DEFAULT 'active'"));
    assert!(sql_str.ends_with(");"));
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
    assert_eq!(sql[0], "DROP TABLE \"users\";", "Unexpected DROP TABLE SQL");
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

    let drop_sql = sql.iter().find(|s| s.contains("DROP TABLE")).unwrap();
    let create_sql = sql.iter().find(|s| s.contains("CREATE TABLE")).unwrap();

    assert_eq!(
        *drop_sql, "DROP TABLE \"old_table\";",
        "Unexpected DROP TABLE SQL"
    );
    assert_eq!(
        *create_sql, "CREATE TABLE \"new_table\" (\n\t\"id\" integer NOT NULL\n);",
        "Unexpected CREATE TABLE SQL"
    );
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
    assert_eq!(
        sql[0], "CREATE INDEX \"users_email_idx\" ON \"users\" USING btree (\"email\" NULLS LAST);",
        "Unexpected CREATE INDEX SQL"
    );
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
    assert_eq!(
        sql[0],
        "CREATE UNIQUE INDEX \"users_email_unique_idx\" ON \"users\" USING btree (\"email\" NULLS LAST);",
        "Unexpected CREATE UNIQUE INDEX SQL"
    );
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
    assert_eq!(
        sql[0], "DROP INDEX \"users_email_idx\";",
        "Unexpected DROP INDEX SQL"
    );
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
    assert_eq!(
        sql[0], "ALTER TABLE \"users\" ADD COLUMN \"email\" text NOT NULL;",
        "Unexpected ADD COLUMN SQL"
    );
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
    assert_eq!(
        sql[0], "ALTER TABLE \"users\" DROP COLUMN \"email\";",
        "Unexpected DROP COLUMN SQL"
    );
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
    assert_eq!(
        sql[0], "CREATE TYPE \"status\" AS ENUM ('active', 'inactive', 'pending');",
        "Unexpected CREATE TYPE SQL"
    );
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
    assert_eq!(sql[0], "DROP TYPE \"status\";", "Unexpected DROP TYPE SQL");
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
    // Verify all column types are present in the output
    let sql_str = &sql[0];
    assert!(
        sql_str.starts_with("CREATE TABLE \"all_types\" ("),
        "Should start with CREATE TABLE"
    );
    assert!(
        sql_str.contains("\"id\" serial NOT NULL"),
        "Should have serial column"
    );
    assert!(
        sql_str.contains("\"small\" smallint NOT NULL"),
        "Should have smallint column"
    );
    assert!(
        sql_str.contains("\"big\" bigint NOT NULL"),
        "Should have bigint column"
    );
    assert!(
        sql_str.contains("\"real_val\" real"),
        "Should have real column"
    );
    assert!(
        sql_str.contains("\"double_val\" double precision"),
        "Should have double precision column"
    );
    assert!(
        sql_str.contains("\"text_val\" text"),
        "Should have text column"
    );
    assert!(
        sql_str.contains("\"varchar_val\" varchar(255)"),
        "Should have varchar column"
    );
    assert!(
        sql_str.contains("\"char_val\" char(10)"),
        "Should have char column"
    );
    assert!(
        sql_str.contains("\"bool_val\" boolean"),
        "Should have boolean column"
    );
    assert!(
        sql_str.contains("\"timestamp_val\" timestamp"),
        "Should have timestamp column"
    );
    assert!(
        sql_str.contains("\"timestamptz_val\" timestamptz"),
        "Should have timestamptz column"
    );
    assert!(
        sql_str.contains("\"date_val\" date"),
        "Should have date column"
    );
    assert!(
        sql_str.contains("\"time_val\" time"),
        "Should have time column"
    );
    assert!(
        sql_str.contains("\"json_val\" json"),
        "Should have json column"
    );
    assert!(
        sql_str.contains("\"jsonb_val\" jsonb"),
        "Should have jsonb column"
    );
    assert!(
        sql_str.contains("\"uuid_val\" uuid"),
        "Should have uuid column"
    );
    assert!(sql_str.ends_with(");"), "Should end with );");
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

    assert!(
        sql.is_empty(),
        "Expected no diff for identical schemas, got: {:?}",
        sql
    );
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
        ordinal_position: None,
    });

    let sql = diff_to_sql(&from, &to);

    assert_eq!(sql.len(), 1);
    assert_eq!(
        sql[0], "CREATE TABLE \"myschema\".\"users\" (\n\t\"id\" integer NOT NULL\n);",
        "Unexpected custom schema SQL"
    );
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

    let users_sql = sql
        .iter()
        .find(|s| s.contains("CREATE TABLE \"users\""))
        .unwrap();
    let posts_sql = sql
        .iter()
        .find(|s| s.contains("CREATE TABLE \"posts\""))
        .unwrap();
    let comments_sql = sql
        .iter()
        .find(|s| s.contains("CREATE TABLE \"comments\""))
        .unwrap();

    assert_eq!(
        *users_sql,
        "CREATE TABLE \"users\" (\n\t\"id\" integer NOT NULL,\n\tPRIMARY KEY(\"id\")\n);",
        "Unexpected users SQL"
    );
    assert_eq!(
        *posts_sql,
        "CREATE TABLE \"posts\" (\n\t\"id\" integer NOT NULL,\n\tPRIMARY KEY(\"id\")\n);",
        "Unexpected posts SQL"
    );
    assert_eq!(
        *comments_sql,
        "CREATE TABLE \"comments\" (\n\t\"id\" integer NOT NULL,\n\tPRIMARY KEY(\"id\")\n);",
        "Unexpected comments SQL"
    );
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
    // Column order may vary
    let sql_str = &sql[0];
    assert!(sql_str.starts_with("CREATE TABLE \"categories\" ("));
    assert!(sql_str.contains("\"id\" integer NOT NULL"));
    assert!(sql_str.contains("\"parent_id\" integer"));
    assert!(sql_str.contains("PRIMARY KEY(\"id\")"));
    assert!(sql_str.contains("CONSTRAINT \"categories_parent_fk\" FOREIGN KEY (\"parent_id\") REFERENCES \"categories\"(\"id\")"));
    assert!(sql_str.ends_with(");"));
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

#[test]
fn test_drop_policy_sql_is_well_formed() {
    let mut from = PostgresDDL::new();
    let mut to = PostgresDDL::new();

    // Keep table present in both schemas; only policy is removed.
    from.tables.push(Table::new("auth", "users"));
    from.columns
        .push(Column::new("auth", "users", "id", "integer").not_null());
    from.pks.push(PrimaryKey::from_strings(
        "auth".to_string(),
        "users".to_string(),
        "users_pkey".to_string(),
        vec!["id".to_string()],
    ));
    from.policies
        .push(Policy::new("auth", "users", "users_rls_policy"));

    to.tables.push(Table::new("auth", "users"));
    to.columns
        .push(Column::new("auth", "users", "id", "integer").not_null());
    to.pks.push(PrimaryKey::from_strings(
        "auth".to_string(),
        "users".to_string(),
        "users_pkey".to_string(),
        vec!["id".to_string()],
    ));

    let sql = diff_to_sql(&from, &to);
    assert_eq!(sql.len(), 1, "Expected one DROP POLICY statement: {sql:?}");
    assert_eq!(
        sql[0],
        "DROP POLICY \"users_rls_policy\" ON \"auth\".\"users\";"
    );
}

#[test]
fn test_drop_policy_sql_public_schema_no_prefix() {
    let mut from = PostgresDDL::new();
    let mut to = PostgresDDL::new();

    from.tables.push(table("users"));
    from.columns.push(column_not_null("users", "id", "integer"));
    from.pks.push(primary_key("users", vec!["id"]));
    from.policies
        .push(Policy::new("public", "users", "users_public_policy"));

    to.tables.push(table("users"));
    to.columns.push(column_not_null("users", "id", "integer"));
    to.pks.push(primary_key("users", vec!["id"]));

    let sql = diff_to_sql(&from, &to);
    assert_eq!(sql.len(), 1, "Expected one DROP POLICY statement: {sql:?}");
    assert_eq!(sql[0], "DROP POLICY \"users_public_policy\" ON \"users\";");
}

#[test]
fn test_create_index_concurrently_sql() {
    let mut from = PostgresDDL::new();
    from.tables.push(table("users"));
    from.columns.push(column_not_null("users", "id", "integer"));
    from.columns.push(column("users", "email", "text"));
    from.pks.push(primary_key("users", vec!["id"]));

    let mut to = from.clone();
    let mut idx = index("users", "users_email_concurrent_idx", vec!["email"]);
    idx.concurrently = true;
    to.indexes.push(idx);

    let sql = diff_to_sql(&from, &to);
    assert_eq!(sql.len(), 1, "Expected one CREATE INDEX statement: {sql:?}");
    assert_eq!(
        sql[0],
        "CREATE INDEX CONCURRENTLY \"users_email_concurrent_idx\" ON \"users\" USING btree (\"email\" NULLS LAST);"
    );
}

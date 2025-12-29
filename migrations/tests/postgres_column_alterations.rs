//! PostgreSQL Column Alteration Tests
//!
//! These tests verify that column alterations generate proper SQL statements.
//! Unlike SQLite, PostgreSQL supports ALTER COLUMN for most changes:
//! - SET NOT NULL / DROP NOT NULL
//! - SET DATA TYPE
//! - SET DEFAULT / DROP DEFAULT
//! - DROP EXPRESSION
//!
//! Only adding generated expressions requires column recreation (drop + add).

use drizzle_migrations::postgres::{
    PostgresDDL,
    collection::diff_ddl,
    ddl::{
        Column, ForeignKey, Generated, GeneratedType, Identity, Index, IndexColumn, PrimaryKey,
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
    }
}

/// Helper to create a NOT NULL column
fn column_not_null(table: &str, name: &str, sql_type: &str) -> Column {
    Column {
        not_null: true,
        ..column(table, name, sql_type)
    }
}

/// Helper to create a column with default
fn column_default(table: &str, name: &str, sql_type: &str, default: &str) -> Column {
    Column {
        default: Some(Cow::Owned(default.to_string())),
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
// ADD COLUMN Tests
// =============================================================================

/// Test: Add single column NOT NULL (uses ALTER TABLE ADD COLUMN)
#[test]
fn test_add_column_not_null() {
    let mut from = PostgresDDL::new();
    from.tables.push(table("users"));
    from.columns.push(column_not_null("users", "id", "integer"));
    from.pks.push(primary_key("users", vec!["id"]));

    let mut to = PostgresDDL::new();
    to.tables.push(table("users"));
    to.columns.push(column_not_null("users", "id", "integer"));
    to.columns.push(column_not_null("users", "name", "text"));
    to.pks.push(primary_key("users", vec!["id"]));

    let sql = diff_to_sql(&from, &to);

    assert_eq!(sql.len(), 1, "Expected 1 SQL statement, got: {:?}", sql);
    assert!(
        sql[0].contains("ALTER TABLE") && sql[0].contains("ADD COLUMN"),
        "Expected ALTER TABLE ADD COLUMN, got: {}",
        sql[0]
    );
    assert!(
        sql[0].contains("\"name\"") && sql[0].contains("NOT NULL"),
        "Expected \"name\" NOT NULL column, got: {}",
        sql[0]
    );
}

/// Test: Add multiple columns
#[test]
fn test_add_multiple_columns() {
    let mut from = PostgresDDL::new();
    from.tables.push(table("users"));
    from.columns.push(column_not_null("users", "id", "integer"));
    from.pks.push(primary_key("users", vec!["id"]));

    let mut to = PostgresDDL::new();
    to.tables.push(table("users"));
    to.columns.push(column_not_null("users", "id", "integer"));
    to.columns.push(column("users", "name", "text"));
    to.columns.push(column("users", "email", "text"));
    to.pks.push(primary_key("users", vec!["id"]));

    let sql = diff_to_sql(&from, &to);

    assert_eq!(sql.len(), 2, "Expected 2 SQL statements, got: {:?}", sql);
    let combined = sql.join("\n");
    assert!(
        combined.contains("\"name\"") && combined.contains("\"email\""),
        "Expected both columns, got: {}",
        combined
    );
}

/// Test: Add column with default
#[test]
fn test_add_column_with_default() {
    let mut from = PostgresDDL::new();
    from.tables.push(table("users"));
    from.columns.push(column_not_null("users", "id", "integer"));
    from.pks.push(primary_key("users", vec!["id"]));

    let mut to = PostgresDDL::new();
    to.tables.push(table("users"));
    to.columns.push(column_not_null("users", "id", "integer"));
    to.columns
        .push(column_default("users", "status", "text", "'active'"));
    to.pks.push(primary_key("users", vec!["id"]));

    let sql = diff_to_sql(&from, &to);

    assert_eq!(sql.len(), 1, "Expected 1 SQL statement, got: {:?}", sql);
    assert!(
        sql[0].contains("DEFAULT 'active'"),
        "Expected DEFAULT 'active', got: {}",
        sql[0]
    );
}

// =============================================================================
// DROP COLUMN Tests
// =============================================================================

/// Test: Drop column (uses ALTER TABLE DROP COLUMN)
#[test]
fn test_drop_column() {
    let mut from = PostgresDDL::new();
    from.tables.push(table("users"));
    from.columns.push(column_not_null("users", "id", "integer"));
    from.columns.push(column("users", "name", "text"));
    from.pks.push(primary_key("users", vec!["id"]));

    let mut to = PostgresDDL::new();
    to.tables.push(table("users"));
    to.columns.push(column_not_null("users", "id", "integer"));
    to.pks.push(primary_key("users", vec!["id"]));

    let sql = diff_to_sql(&from, &to);

    assert_eq!(sql.len(), 1, "Expected 1 SQL statement, got: {:?}", sql);
    assert!(
        sql[0].contains("DROP COLUMN") && sql[0].contains("\"name\""),
        "Expected DROP COLUMN \"name\", got: {}",
        sql[0]
    );
}

// =============================================================================
// ALTER COLUMN Tests - PostgreSQL uses ALTER TABLE ALTER COLUMN
// =============================================================================

/// Test: Alter column add NOT NULL (uses SET NOT NULL)
#[test]
fn test_alter_column_add_not_null() {
    let mut from = PostgresDDL::new();
    from.tables.push(table("users"));
    from.columns.push(column_not_null("users", "id", "integer"));
    from.columns.push(column("users", "email", "text")); // nullable
    from.pks.push(primary_key("users", vec!["id"]));

    let mut to = PostgresDDL::new();
    to.tables.push(table("users"));
    to.columns.push(column_not_null("users", "id", "integer"));
    to.columns.push(column_not_null("users", "email", "text")); // NOT NULL
    to.pks.push(primary_key("users", vec!["id"]));

    let sql = diff_to_sql(&from, &to);

    let combined = sql.join("\n");
    assert!(
        combined.contains("SET NOT NULL"),
        "Expected SET NOT NULL, got: {}",
        combined
    );
    assert!(
        combined.contains("ALTER TABLE") && combined.contains("ALTER COLUMN"),
        "Expected ALTER TABLE ALTER COLUMN, got: {}",
        combined
    );
}

/// Test: Alter column drop NOT NULL (uses DROP NOT NULL)
#[test]
fn test_alter_column_drop_not_null() {
    let mut from = PostgresDDL::new();
    from.tables.push(table("users"));
    from.columns.push(column_not_null("users", "id", "integer"));
    from.columns.push(column_not_null("users", "email", "text")); // NOT NULL
    from.pks.push(primary_key("users", vec!["id"]));

    let mut to = PostgresDDL::new();
    to.tables.push(table("users"));
    to.columns.push(column_not_null("users", "id", "integer"));
    to.columns.push(column("users", "email", "text")); // nullable
    to.pks.push(primary_key("users", vec!["id"]));

    let sql = diff_to_sql(&from, &to);

    let combined = sql.join("\n");
    assert!(
        combined.contains("DROP NOT NULL"),
        "Expected DROP NOT NULL, got: {}",
        combined
    );
}

/// Test: Alter column add default (uses SET DEFAULT)
#[test]
fn test_alter_column_add_default() {
    let mut from = PostgresDDL::new();
    from.tables.push(table("users"));
    from.columns.push(column_not_null("users", "id", "integer"));
    from.columns.push(column("users", "status", "text")); // no default
    from.pks.push(primary_key("users", vec!["id"]));

    let mut to = PostgresDDL::new();
    to.tables.push(table("users"));
    to.columns.push(column_not_null("users", "id", "integer"));
    to.columns
        .push(column_default("users", "status", "text", "'active'")); // with default
    to.pks.push(primary_key("users", vec!["id"]));

    let sql = diff_to_sql(&from, &to);

    let combined = sql.join("\n");
    assert!(
        combined.contains("SET DEFAULT"),
        "Expected SET DEFAULT, got: {}",
        combined
    );
    assert!(
        combined.contains("'active'"),
        "Expected 'active' default value, got: {}",
        combined
    );
}

/// Test: Alter column drop default (uses DROP DEFAULT)
#[test]
fn test_alter_column_drop_default() {
    let mut from = PostgresDDL::new();
    from.tables.push(table("users"));
    from.columns.push(column_not_null("users", "id", "integer"));
    from.columns
        .push(column_default("users", "status", "text", "'active'")); // with default
    from.pks.push(primary_key("users", vec!["id"]));

    let mut to = PostgresDDL::new();
    to.tables.push(table("users"));
    to.columns.push(column_not_null("users", "id", "integer"));
    to.columns.push(column("users", "status", "text")); // no default
    to.pks.push(primary_key("users", vec!["id"]));

    let sql = diff_to_sql(&from, &to);

    let combined = sql.join("\n");
    assert!(
        combined.contains("DROP DEFAULT"),
        "Expected DROP DEFAULT, got: {}",
        combined
    );
}

/// Test: Alter column type change (uses SET DATA TYPE)
#[test]
fn test_alter_column_type_change() {
    let mut from = PostgresDDL::new();
    from.tables.push(table("users"));
    from.columns.push(column_not_null("users", "id", "integer"));
    from.columns.push(column("users", "age", "text")); // text
    from.pks.push(primary_key("users", vec!["id"]));

    let mut to = PostgresDDL::new();
    to.tables.push(table("users"));
    to.columns.push(column_not_null("users", "id", "integer"));
    to.columns.push(column("users", "age", "integer")); // integer
    to.pks.push(primary_key("users", vec!["id"]));

    let sql = diff_to_sql(&from, &to);

    let combined = sql.join("\n");
    assert!(
        combined.contains("SET DATA TYPE"),
        "Expected SET DATA TYPE, got: {}",
        combined
    );
    assert!(
        combined.contains("integer"),
        "Expected integer type, got: {}",
        combined
    );
}

/// Test: Alter column multiple properties at once
#[test]
fn test_alter_column_multiple_changes() {
    let mut from = PostgresDDL::new();
    from.tables.push(table("users"));
    from.columns.push(column_not_null("users", "id", "integer"));
    from.columns.push(column("users", "status", "text")); // nullable, no default
    from.pks.push(primary_key("users", vec!["id"]));

    let mut to = PostgresDDL::new();
    to.tables.push(table("users"));
    to.columns.push(column_not_null("users", "id", "integer"));
    // NOT NULL + default
    let mut status_col = column_default("users", "status", "text", "'pending'");
    status_col.not_null = true;
    to.columns.push(status_col);
    to.pks.push(primary_key("users", vec!["id"]));

    let sql = diff_to_sql(&from, &to);

    let combined = sql.join("\n");
    assert!(
        combined.contains("SET NOT NULL"),
        "Expected SET NOT NULL, got: {}",
        combined
    );
    assert!(
        combined.contains("SET DEFAULT"),
        "Expected SET DEFAULT, got: {}",
        combined
    );
}

// =============================================================================
// Generated Column Tests - Adding generated expressions requires column recreation
// =============================================================================

/// Test: Add generated stored column to new table (inline in CREATE TABLE)
#[test]
fn test_create_table_with_generated_column() {
    let from = PostgresDDL::new();

    let mut to = PostgresDDL::new();
    to.tables.push(table("users"));
    to.columns.push(column_not_null("users", "id", "integer"));
    to.columns.push(column("users", "first_name", "text"));
    to.columns.push(column("users", "last_name", "text"));

    // Generated column
    let mut full_name = column("users", "full_name", "text");
    full_name.generated = Some(Generated {
        expression: Cow::Borrowed("first_name || ' ' || last_name"),
        gen_type: GeneratedType::Stored,
    });
    to.columns.push(full_name);
    to.pks.push(primary_key("users", vec!["id"]));

    let sql = diff_to_sql(&from, &to);

    // Should be CREATE TABLE with GENERATED ALWAYS AS
    let combined = sql.join("\n");
    assert!(
        combined.contains("CREATE TABLE"),
        "Expected CREATE TABLE, got: {}",
        combined
    );
    assert!(
        combined.contains("GENERATED ALWAYS AS"),
        "Expected GENERATED ALWAYS AS, got: {}",
        combined
    );
}

/// Test: Adding generated expression to existing column requires DROP+ADD
#[test]
fn test_alter_column_add_generated_expression() {
    let mut from = PostgresDDL::new();
    from.tables.push(table("users"));
    from.columns.push(column_not_null("users", "id", "integer"));
    from.columns.push(column("users", "first_name", "text"));
    from.columns.push(column("users", "last_name", "text"));
    from.columns.push(column("users", "full_name", "text")); // Regular column
    from.pks.push(primary_key("users", vec!["id"]));

    let mut to = PostgresDDL::new();
    to.tables.push(table("users"));
    to.columns.push(column_not_null("users", "id", "integer"));
    to.columns.push(column("users", "first_name", "text"));
    to.columns.push(column("users", "last_name", "text"));

    // full_name is now a generated column
    let mut full_name = column("users", "full_name", "text");
    full_name.generated = Some(Generated {
        expression: Cow::Borrowed("first_name || ' ' || last_name"),
        gen_type: GeneratedType::Stored,
    });
    to.columns.push(full_name);
    to.pks.push(primary_key("users", vec!["id"]));

    let sql = diff_to_sql(&from, &to);
    let combined = sql.join("\n");

    // Should have DROP COLUMN and ADD COLUMN for the recreate
    assert!(
        combined.contains("DROP COLUMN \"full_name\""),
        "Expected DROP COLUMN for recreating generated column, got:\n{}",
        combined
    );
    assert!(
        combined.contains("ADD COLUMN \"full_name\""),
        "Expected ADD COLUMN for recreating generated column, got:\n{}",
        combined
    );
    assert!(
        combined.contains("GENERATED ALWAYS AS"),
        "Expected GENERATED ALWAYS AS in the new column definition, got:\n{}",
        combined
    );
}

/// Test: Drop generated expression (uses DROP EXPRESSION)
#[test]
fn test_alter_column_drop_generated_expression() {
    let mut from = PostgresDDL::new();
    from.tables.push(table("users"));
    from.columns.push(column_not_null("users", "id", "integer"));
    from.columns.push(column("users", "first_name", "text"));
    from.columns.push(column("users", "last_name", "text"));

    // Generated column
    let mut full_name = column("users", "full_name", "text");
    full_name.generated = Some(Generated {
        expression: Cow::Borrowed("first_name || ' ' || last_name"),
        gen_type: GeneratedType::Stored,
    });
    from.columns.push(full_name);
    from.pks.push(primary_key("users", vec!["id"]));

    let mut to = PostgresDDL::new();
    to.tables.push(table("users"));
    to.columns.push(column_not_null("users", "id", "integer"));
    to.columns.push(column("users", "first_name", "text"));
    to.columns.push(column("users", "last_name", "text"));
    to.columns.push(column("users", "full_name", "text")); // Now regular column
    to.pks.push(primary_key("users", vec!["id"]));

    let sql = diff_to_sql(&from, &to);
    let combined = sql.join("\n");

    // Should use DROP EXPRESSION (not recreation)
    assert!(
        combined.contains("DROP EXPRESSION"),
        "Expected DROP EXPRESSION for removing generated, got:\n{}",
        combined
    );
}

// =============================================================================
// Identity Column Tests
// =============================================================================

/// Test: Add identity to column
#[test]
fn test_alter_column_add_identity() {
    let mut from = PostgresDDL::new();
    from.tables.push(table("users"));
    from.columns.push(column_not_null("users", "id", "integer")); // no identity
    from.pks.push(primary_key("users", vec!["id"]));

    let mut to = PostgresDDL::new();
    to.tables.push(table("users"));

    let mut id_col = column_not_null("users", "id", "integer");
    id_col.identity = Some(Identity::always("users_id_seq").schema("public"));
    to.columns.push(id_col);
    to.pks.push(primary_key("users", vec!["id"]));

    let sql = diff_to_sql(&from, &to);
    let combined = sql.join("\n");

    assert!(
        combined.contains("ADD GENERATED ALWAYS AS IDENTITY"),
        "Expected ADD GENERATED ALWAYS AS IDENTITY, got:\n{}",
        combined
    );
}

/// Test: Drop identity from column
#[test]
fn test_alter_column_drop_identity() {
    let mut from = PostgresDDL::new();
    from.tables.push(table("users"));

    let mut id_col = column_not_null("users", "id", "integer");
    id_col.identity = Some(Identity::always("users_id_seq").schema("public"));
    from.columns.push(id_col);
    from.pks.push(primary_key("users", vec!["id"]));

    let mut to = PostgresDDL::new();
    to.tables.push(table("users"));
    to.columns.push(column_not_null("users", "id", "integer")); // no identity
    to.pks.push(primary_key("users", vec!["id"]));

    let sql = diff_to_sql(&from, &to);
    let combined = sql.join("\n");

    assert!(
        combined.contains("DROP IDENTITY"),
        "Expected DROP IDENTITY, got:\n{}",
        combined
    );
}

// =============================================================================
// Constraint Tests - PostgreSQL uses ALTER TABLE ADD/DROP CONSTRAINT
// =============================================================================

/// Test: Add foreign key to existing table
#[test]
fn test_add_foreign_key() {
    let mut from = PostgresDDL::new();
    from.tables.push(table("users"));
    from.columns.push(column_not_null("users", "id", "integer"));
    from.pks.push(primary_key("users", vec!["id"]));

    from.tables.push(table("posts"));
    from.columns.push(column_not_null("posts", "id", "integer"));
    from.columns.push(column("posts", "author_id", "integer"));
    from.pks.push(primary_key("posts", vec!["id"]));

    let mut to = from.clone();
    to.fks.push(foreign_key(
        "posts",
        "posts_author_fk",
        vec!["author_id"],
        "users",
        vec!["id"],
    ));

    let sql = diff_to_sql(&from, &to);
    let combined = sql.join("\n");

    assert!(
        combined.contains("ADD CONSTRAINT") || combined.contains("FOREIGN KEY"),
        "Expected ADD CONSTRAINT FOREIGN KEY, got:\n{}",
        combined
    );
}

/// Test: Drop foreign key
#[test]
fn test_drop_foreign_key() {
    let mut from = PostgresDDL::new();
    from.tables.push(table("users"));
    from.columns.push(column_not_null("users", "id", "integer"));
    from.pks.push(primary_key("users", vec!["id"]));

    from.tables.push(table("posts"));
    from.columns.push(column_not_null("posts", "id", "integer"));
    from.columns.push(column("posts", "author_id", "integer"));
    from.pks.push(primary_key("posts", vec!["id"]));
    from.fks.push(foreign_key(
        "posts",
        "posts_author_fk",
        vec!["author_id"],
        "users",
        vec!["id"],
    ));

    let mut to = PostgresDDL::new();
    to.tables.push(table("users"));
    to.columns.push(column_not_null("users", "id", "integer"));
    to.pks.push(primary_key("users", vec!["id"]));

    to.tables.push(table("posts"));
    to.columns.push(column_not_null("posts", "id", "integer"));
    to.columns.push(column("posts", "author_id", "integer"));
    to.pks.push(primary_key("posts", vec!["id"]));
    // No FK in "to"

    let sql = diff_to_sql(&from, &to);
    let combined = sql.join("\n");

    assert!(
        combined.contains("DROP CONSTRAINT"),
        "Expected DROP CONSTRAINT, got:\n{}",
        combined
    );
}

/// Test: Add primary key to existing table
#[test]
fn test_add_primary_key() {
    let mut from = PostgresDDL::new();
    from.tables.push(table("users"));
    from.columns.push(column_not_null("users", "id", "integer"));
    from.columns.push(column("users", "name", "text"));
    // No PK

    let mut to = PostgresDDL::new();
    to.tables.push(table("users"));
    to.columns.push(column_not_null("users", "id", "integer"));
    to.columns.push(column("users", "name", "text"));
    to.pks.push(primary_key("users", vec!["id"])); // Add PK

    let sql = diff_to_sql(&from, &to);
    let combined = sql.join("\n");

    assert!(
        combined.contains("ADD CONSTRAINT") && combined.contains("PRIMARY KEY"),
        "Expected ADD CONSTRAINT PRIMARY KEY, got:\n{}",
        combined
    );
}

/// Test: Drop primary key
#[test]
fn test_drop_primary_key() {
    let mut from = PostgresDDL::new();
    from.tables.push(table("users"));
    from.columns.push(column_not_null("users", "id", "integer"));
    from.columns.push(column("users", "name", "text"));
    from.pks.push(primary_key("users", vec!["id"]));

    let mut to = PostgresDDL::new();
    to.tables.push(table("users"));
    to.columns.push(column_not_null("users", "id", "integer"));
    to.columns.push(column("users", "name", "text"));
    // No PK

    let sql = diff_to_sql(&from, &to);
    let combined = sql.join("\n");

    assert!(
        combined.contains("DROP CONSTRAINT"),
        "Expected DROP CONSTRAINT, got:\n{}",
        combined
    );
}

/// Test: Add unique constraint
#[test]
fn test_add_unique_constraint() {
    let mut from = PostgresDDL::new();
    from.tables.push(table("users"));
    from.columns.push(column_not_null("users", "id", "integer"));
    from.columns.push(column("users", "email", "text"));
    from.pks.push(primary_key("users", vec!["id"]));

    let mut to = from.clone();
    to.uniques.push(unique_constraint(
        "users",
        "users_email_unique",
        vec!["email"],
    ));

    let sql = diff_to_sql(&from, &to);
    let combined = sql.join("\n");

    assert!(
        combined.contains("ADD CONSTRAINT") && combined.contains("UNIQUE"),
        "Expected ADD CONSTRAINT UNIQUE, got:\n{}",
        combined
    );
}

/// Test: Drop unique constraint
#[test]
fn test_drop_unique_constraint() {
    let mut from = PostgresDDL::new();
    from.tables.push(table("users"));
    from.columns.push(column_not_null("users", "id", "integer"));
    from.columns.push(column("users", "email", "text"));
    from.pks.push(primary_key("users", vec!["id"]));
    from.uniques.push(unique_constraint(
        "users",
        "users_email_unique",
        vec!["email"],
    ));

    let mut to = PostgresDDL::new();
    to.tables.push(table("users"));
    to.columns.push(column_not_null("users", "id", "integer"));
    to.columns.push(column("users", "email", "text"));
    to.pks.push(primary_key("users", vec!["id"]));
    // No unique constraint

    let sql = diff_to_sql(&from, &to);
    let combined = sql.join("\n");

    assert!(
        combined.contains("DROP CONSTRAINT"),
        "Expected DROP CONSTRAINT, got:\n{}",
        combined
    );
}

// =============================================================================
// Index Tests
// =============================================================================

/// Test: Add index to existing table
#[test]
fn test_add_index() {
    let mut from = PostgresDDL::new();
    from.tables.push(table("users"));
    from.columns.push(column_not_null("users", "id", "integer"));
    from.columns.push(column("users", "email", "text"));
    from.pks.push(primary_key("users", vec!["id"]));

    let mut to = from.clone();
    to.indexes
        .push(index("users", "users_email_idx", vec!["email"]));

    let sql = diff_to_sql(&from, &to);
    let combined = sql.join("\n");

    assert!(
        combined.contains("CREATE INDEX"),
        "Expected CREATE INDEX, got:\n{}",
        combined
    );
}

/// Test: Drop index
#[test]
fn test_drop_index() {
    let mut from = PostgresDDL::new();
    from.tables.push(table("users"));
    from.columns.push(column_not_null("users", "id", "integer"));
    from.columns.push(column("users", "email", "text"));
    from.pks.push(primary_key("users", vec!["id"]));
    from.indexes
        .push(index("users", "users_email_idx", vec!["email"]));

    let mut to = PostgresDDL::new();
    to.tables.push(table("users"));
    to.columns.push(column_not_null("users", "id", "integer"));
    to.columns.push(column("users", "email", "text"));
    to.pks.push(primary_key("users", vec!["id"]));
    // No index

    let sql = diff_to_sql(&from, &to);
    let combined = sql.join("\n");

    assert!(
        combined.contains("DROP INDEX"),
        "Expected DROP INDEX, got:\n{}",
        combined
    );
}

// =============================================================================
// Edge Cases and No-Op Tests
// =============================================================================

/// Test: No changes produces no SQL
#[test]
fn test_no_changes_no_sql() {
    let mut schema = PostgresDDL::new();
    schema.tables.push(table("users"));
    schema
        .columns
        .push(column_not_null("users", "id", "integer"));
    schema
        .columns
        .push(column_not_null("users", "name", "text"));
    schema.pks.push(primary_key("users", vec!["id"]));

    let sql = diff_to_sql(&schema, &schema.clone());

    assert!(
        sql.is_empty(),
        "Expected no SQL for identical schemas, got: {:?}",
        sql
    );
}

/// Test: Multiple tables with different changes
#[test]
fn test_multiple_tables_different_changes() {
    let mut from = PostgresDDL::new();
    // users table
    from.tables.push(table("users"));
    from.columns.push(column_not_null("users", "id", "integer"));
    from.columns.push(column("users", "email", "text")); // nullable
    from.pks.push(primary_key("users", vec!["id"]));
    // posts table
    from.tables.push(table("posts"));
    from.columns.push(column_not_null("posts", "id", "integer"));
    from.columns.push(column_not_null("posts", "title", "text"));
    from.pks.push(primary_key("posts", vec!["id"]));

    let mut to = PostgresDDL::new();
    // users: make email NOT NULL
    to.tables.push(table("users"));
    to.columns.push(column_not_null("users", "id", "integer"));
    to.columns.push(column_not_null("users", "email", "text")); // NOT NULL now
    to.pks.push(primary_key("users", vec!["id"]));
    // posts: add a column
    to.tables.push(table("posts"));
    to.columns.push(column_not_null("posts", "id", "integer"));
    to.columns.push(column_not_null("posts", "title", "text"));
    to.columns.push(column("posts", "content", "text")); // new column
    to.pks.push(primary_key("posts", vec!["id"]));

    let sql = diff_to_sql(&from, &to);
    let combined = sql.join("\n");

    // users: SET NOT NULL
    assert!(
        combined.contains("SET NOT NULL"),
        "Expected SET NOT NULL for users.email, got:\n{}",
        combined
    );
    // posts: ADD COLUMN
    assert!(
        combined.contains("ADD COLUMN") && combined.contains("\"content\""),
        "Expected ADD COLUMN for posts.content, got:\n{}",
        combined
    );
}

/// Test: Custom schema table alterations
#[test]
fn test_custom_schema_alterations() {
    let mut from = PostgresDDL::new();
    from.tables.push(Table {
        schema: Cow::Borrowed("myschema"),
        name: Cow::Borrowed("users"),
        is_rls_enabled: None,
    });
    from.columns.push(Column {
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

    let mut to = from.clone();
    to.columns.push(Column {
        schema: Cow::Borrowed("myschema"),
        table: Cow::Borrowed("users"),
        name: Cow::Borrowed("name"),
        sql_type: Cow::Borrowed("text"),
        type_schema: None,
        not_null: false,
        default: None,
        generated: None,
        identity: None,
        dimensions: None,
    });

    let sql = diff_to_sql(&from, &to);
    let combined = sql.join("\n");

    assert!(
        combined.contains("\"myschema\".\"users\""),
        "Expected schema-qualified table name, got:\n{}",
        combined
    );
    assert!(
        combined.contains("ADD COLUMN"),
        "Expected ADD COLUMN, got:\n{}",
        combined
    );
}

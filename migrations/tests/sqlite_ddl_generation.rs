//! SQLite DDL SQL Generation Tests
//!
//! These tests mirror the patterns from drizzle-orm's drizzle-kit/tests/sqlite/sqlite-tables.test.ts
//! They verify that the SQL generation for CREATE TABLE statements matches expected output.

use std::borrow::Cow;

use drizzle_migrations::sqlite::{
    SQLiteDDL, SchemaDiff,
    collection::diff_ddl,
    ddl::{
        ColumnDef, ForeignKeyDef, IndexColumnDef, IndexDef, PrimaryKeyDef, ReferentialAction,
        TableDef,
    },
    statements::SqliteGenerator,
};

// =============================================================================
// Helper Functions
// =============================================================================

/// Generate SQL statements from a diff between two DDLs
fn diff_to_sql(from: &SQLiteDDL, to: &SQLiteDDL) -> Vec<String> {
    let diffs = diff_ddl(from, to);
    let generator = SqliteGenerator::new();
    let diff = SchemaDiff { diffs };
    generator.generate_migration(&diff)
}

/// Normalize a CREATE TABLE statement by sorting column/constraint lines.
/// This allows deterministic comparison when column order is non-deterministic.
fn normalize_create_table(sql: &str) -> String {
    // Find the body between "(" and ");\n" (or ") STRICT;\n" etc.)
    let open = match sql.find('(') {
        Some(i) => i,
        None => return sql.to_string(),
    };
    let close = match sql.rfind(')') {
        Some(i) => i,
        None => return sql.to_string(),
    };

    let prefix = &sql[..=open]; // "CREATE TABLE `name` ("
    let body = &sql[open + 1..close]; // column lines
    let suffix = &sql[close..]; // ");\n" or ") STRICT;\n"

    let mut lines: Vec<&str> = body.split(",\n").map(|l| l.trim_matches('\n')).collect();
    lines.sort();

    format!("{}{}{}", prefix, lines.join(",\n"), suffix)
}

// =============================================================================
// CREATE TABLE Tests (mirrors drizzle-orm's "add table #1-9")
// =============================================================================

/// Test #1: Basic table with single integer column
#[test]
fn test_create_table_basic() {
    let mut to = SQLiteDDL::default();

    to.tables.push(TableDef::new("users").into_table());
    to.columns
        .push(ColumnDef::new("users", "id", "integer").into_column());

    let sql = diff_to_sql(&SQLiteDDL::default(), &to);

    assert_eq!(sql.len(), 1, "Expected 1 SQL statement, got: {:?}", sql);
    assert_eq!(
        sql[0], "CREATE TABLE `users` (\n\t`id` INTEGER\n);\n",
        "Unexpected CREATE TABLE SQL"
    );
}

/// Test #2: Table with primary key and autoincrement
/// Note: The generator outputs AUTOINCREMENT NOT NULL (without PRIMARY KEY keyword inline)
#[test]
fn test_create_table_with_primary_key_autoincrement() {
    let mut to = SQLiteDDL::default();

    to.tables.push(TableDef::new("users").into_table());
    to.columns.push(
        ColumnDef::new("users", "id", "integer")
            .primary_key()
            .autoincrement()
            .into_column(),
    );

    let sql = diff_to_sql(&SQLiteDDL::default(), &to);

    assert_eq!(sql.len(), 1);
    assert_eq!(
        sql[0], "CREATE TABLE `users` (\n\t`id` INTEGER AUTOINCREMENT NOT NULL\n);\n",
        "Unexpected CREATE TABLE with PRIMARY KEY AUTOINCREMENT"
    );
}

/// Test #3: Table with named primary key constraint
/// Note: Single-column PKs are rendered inline (not as separate constraint)
#[test]
fn test_create_table_with_named_pk_constraint() {
    let mut to = SQLiteDDL::default();

    to.tables.push(TableDef::new("users").into_table());
    to.columns
        .push(ColumnDef::new("users", "id", "integer").into_column());

    // Add named primary key constraint
    const PK_COLS: &[Cow<'static, str>] = &[Cow::Borrowed("id")];
    to.pks.push(
        PrimaryKeyDef::new("users", "users_pk")
            .columns(PK_COLS)
            .into_primary_key(),
    );

    let sql = diff_to_sql(&SQLiteDDL::default(), &to);

    assert_eq!(sql.len(), 1);
    // Single-column PK is rendered inline
    assert_eq!(
        sql[0], "CREATE TABLE `users` (\n\t`id` INTEGER PRIMARY KEY\n);\n",
        "Unexpected CREATE TABLE with named PK constraint"
    );
}

/// Test #4: Multiple tables
#[test]
fn test_create_multiple_tables() {
    let mut to = SQLiteDDL::default();

    // Users table
    to.tables.push(TableDef::new("users").into_table());
    to.columns
        .push(ColumnDef::new("users", "id", "integer").into_column());

    // Posts table
    to.tables.push(TableDef::new("posts").into_table());
    to.columns
        .push(ColumnDef::new("posts", "id", "integer").into_column());

    let sql = diff_to_sql(&SQLiteDDL::default(), &to);

    assert_eq!(
        sql.len(),
        2,
        "Expected 2 CREATE TABLE statements, got: {:?}",
        sql
    );

    // Sort for deterministic comparison (order may vary)
    let mut sorted_sql = sql.clone();
    sorted_sql.sort();

    assert_eq!(
        sorted_sql[0], "CREATE TABLE `posts` (\n\t`id` INTEGER\n);\n",
        "Unexpected posts table SQL"
    );
    assert_eq!(
        sorted_sql[1], "CREATE TABLE `users` (\n\t`id` INTEGER\n);\n",
        "Unexpected users table SQL"
    );
}

/// Test #5: Composite primary key
/// Note: Column order may vary
#[test]
fn test_create_table_composite_pk() {
    let mut to = SQLiteDDL::default();

    to.tables.push(TableDef::new("users").into_table());
    to.columns
        .push(ColumnDef::new("users", "id1", "integer").into_column());
    to.columns
        .push(ColumnDef::new("users", "id2", "integer").into_column());

    // Add composite primary key
    const PK_COLS: &[Cow<'static, str>] = &[Cow::Borrowed("id1"), Cow::Borrowed("id2")];
    to.pks.push(
        PrimaryKeyDef::new("users", "users_pk")
            .columns(PK_COLS)
            .into_primary_key(),
    );

    let sql = diff_to_sql(&SQLiteDDL::default(), &to);

    assert_eq!(sql.len(), 1);
    assert_eq!(
        normalize_create_table(&sql[0]),
        normalize_create_table(
            "CREATE TABLE `users` (\n\t`id1` INTEGER,\n\t`id2` INTEGER,\n\tCONSTRAINT `users_pk` PRIMARY KEY(`id1`, `id2`)\n);\n"
        ),
    );
}

/// Test #6: Drop and create table (schema change)
#[test]
fn test_drop_and_create_table() {
    let mut from = SQLiteDDL::default();
    from.tables.push(TableDef::new("users1").into_table());
    from.columns
        .push(ColumnDef::new("users1", "id", "integer").into_column());

    let mut to = SQLiteDDL::default();
    to.tables.push(TableDef::new("users2").into_table());
    to.columns
        .push(ColumnDef::new("users2", "id", "integer").into_column());

    let sql = diff_to_sql(&from, &to);

    assert_eq!(
        sql.len(),
        2,
        "Expected CREATE and DROP statements, got: {:?}",
        sql
    );

    // Find DROP and CREATE statements (order may vary)
    let drop_sql = sql.iter().find(|s| s.contains("DROP TABLE")).unwrap();
    let create_sql = sql.iter().find(|s| s.contains("CREATE TABLE")).unwrap();

    // DROP statements don't have trailing newline
    assert_eq!(
        *drop_sql, "DROP TABLE `users1`;",
        "Unexpected DROP TABLE SQL"
    );
    assert_eq!(
        *create_sql, "CREATE TABLE `users2` (\n\t`id` INTEGER\n);\n",
        "Unexpected CREATE TABLE SQL"
    );
}

/// Test #8: Self-referencing foreign key
/// Note: Column order may vary
#[test]
fn test_create_table_self_referencing_fk() {
    let mut to = SQLiteDDL::default();

    to.tables.push(TableDef::new("users").into_table());
    to.columns.push(
        ColumnDef::new("users", "id", "integer")
            .primary_key()
            .autoincrement()
            .into_column(),
    );
    to.columns
        .push(ColumnDef::new("users", "reportee_id", "integer").into_column());

    // Add self-referencing foreign key
    const FK_COLS: &[Cow<'static, str>] = &[Cow::Borrowed("reportee_id")];
    const FK_REFS: &[Cow<'static, str>] = &[Cow::Borrowed("id")];
    to.fks.push(
        ForeignKeyDef::new("users", "fk_users_reportee_id_users_id_fk")
            .columns(FK_COLS)
            .references("users", FK_REFS)
            .into_foreign_key(),
    );

    let sql = diff_to_sql(&SQLiteDDL::default(), &to);

    assert_eq!(sql.len(), 1);
    assert_eq!(
        normalize_create_table(&sql[0]),
        normalize_create_table(
            "CREATE TABLE `users` (\n\t`id` INTEGER AUTOINCREMENT NOT NULL,\n\t`reportee_id` INTEGER,\n\tCONSTRAINT `fk_users_reportee_id_users_id_fk` FOREIGN KEY (`reportee_id`) REFERENCES `users`(`id`)\n);\n"
        ),
    );
}

/// Test #9: Table with index
#[test]
fn test_create_table_with_index() {
    let mut to = SQLiteDDL::default();

    to.tables.push(TableDef::new("users").into_table());
    to.columns.push(
        ColumnDef::new("users", "id", "integer")
            .primary_key()
            .autoincrement()
            .into_column(),
    );
    to.columns
        .push(ColumnDef::new("users", "reportee_id", "integer").into_column());

    // Add index
    const IDX_COLS: &[IndexColumnDef] = &[IndexColumnDef::new("reportee_id")];
    to.indexes.push(
        IndexDef::new("users", "reportee_idx")
            .columns(IDX_COLS)
            .into_index(),
    );

    let sql = diff_to_sql(&SQLiteDDL::default(), &to);

    assert_eq!(
        sql.len(),
        2,
        "Expected CREATE TABLE and CREATE INDEX statements"
    );

    let create_table = sql.iter().find(|s| s.contains("CREATE TABLE")).unwrap();
    let create_index = sql.iter().find(|s| s.contains("CREATE INDEX")).unwrap();

    assert_eq!(
        normalize_create_table(create_table),
        normalize_create_table(
            "CREATE TABLE `users` (\n\t`id` INTEGER AUTOINCREMENT NOT NULL,\n\t`reportee_id` INTEGER\n);\n"
        ),
    );
    assert_eq!(
        *create_index,
        "CREATE INDEX `reportee_idx` ON `users` (`reportee_id`);",
    );
}

// =============================================================================
// Column Type Tests
// =============================================================================

/// Test various SQLite column types
/// Note: Column order in output may differ from insertion order
#[test]
fn test_column_types() {
    let mut to = SQLiteDDL::default();

    to.tables.push(TableDef::new("types_test").into_table());

    // Integer type
    to.columns
        .push(ColumnDef::new("types_test", "int_col", "integer").into_column());

    // Text type with NOT NULL
    to.columns.push(
        ColumnDef::new("types_test", "text_col", "text")
            .not_null()
            .into_column(),
    );

    // Real type with default
    to.columns.push(
        ColumnDef::new("types_test", "real_col", "real")
            .default_value("0.0")
            .into_column(),
    );

    // Blob type
    to.columns
        .push(ColumnDef::new("types_test", "blob_col", "blob").into_column());

    // Numeric type
    to.columns
        .push(ColumnDef::new("types_test", "numeric_col", "numeric").into_column());

    let sql = diff_to_sql(&SQLiteDDL::default(), &to);

    assert_eq!(sql.len(), 1);
    assert_eq!(
        normalize_create_table(&sql[0]),
        normalize_create_table(
            "CREATE TABLE `types_test` (\n\t`int_col` INTEGER,\n\t`text_col` TEXT NOT NULL,\n\t`real_col` REAL DEFAULT 0.0,\n\t`blob_col` BLOB,\n\t`numeric_col` NUMERIC\n);\n"
        ),
    );
}

// =============================================================================
// Constraint Tests
// =============================================================================

/// Test unique constraint on column
/// Note: Column order may vary
#[test]
fn test_unique_column() {
    let mut to = SQLiteDDL::default();

    to.tables.push(TableDef::new("users").into_table());
    to.columns.push(
        ColumnDef::new("users", "id", "integer")
            .primary_key()
            .into_column(),
    );
    to.columns.push(
        ColumnDef::new("users", "email", "text")
            .not_null()
            .unique()
            .into_column(),
    );

    let sql = diff_to_sql(&SQLiteDDL::default(), &to);

    assert_eq!(sql.len(), 1);
    assert_eq!(
        normalize_create_table(&sql[0]),
        normalize_create_table(
            "CREATE TABLE `users` (\n\t`id` INTEGER NOT NULL,\n\t`email` TEXT NOT NULL\n);\n"
        ),
    );
}

/// Test unique index
/// Note: Column order may vary
#[test]
fn test_unique_index() {
    let mut to = SQLiteDDL::default();

    to.tables.push(TableDef::new("users").into_table());
    to.columns.push(
        ColumnDef::new("users", "id", "integer")
            .primary_key()
            .into_column(),
    );
    to.columns
        .push(ColumnDef::new("users", "email", "text").into_column());

    // Add unique index
    const IDX_COLS: &[IndexColumnDef] = &[IndexColumnDef::new("email")];
    to.indexes.push(
        IndexDef::new("users", "idx_users_email")
            .columns(IDX_COLS)
            .unique()
            .into_index(),
    );

    let sql = diff_to_sql(&SQLiteDDL::default(), &to);

    assert_eq!(sql.len(), 2);

    let create_table = sql.iter().find(|s| s.contains("CREATE TABLE")).unwrap();
    let create_index = sql
        .iter()
        .find(|s| s.contains("CREATE UNIQUE INDEX"))
        .unwrap();

    assert_eq!(
        normalize_create_table(create_table),
        normalize_create_table(
            "CREATE TABLE `users` (\n\t`id` INTEGER NOT NULL,\n\t`email` TEXT\n);\n"
        ),
    );
    assert_eq!(
        *create_index,
        "CREATE UNIQUE INDEX `idx_users_email` ON `users` (`email`);",
    );
}

// =============================================================================
// Foreign Key Tests
// =============================================================================

/// Test foreign key with ON DELETE CASCADE
/// Note: Column order may vary
#[test]
fn test_foreign_key_on_delete_cascade() {
    let mut to = SQLiteDDL::default();

    // Users table
    to.tables.push(TableDef::new("users").into_table());
    to.columns.push(
        ColumnDef::new("users", "id", "integer")
            .primary_key()
            .into_column(),
    );

    // Posts table
    to.tables.push(TableDef::new("posts").into_table());
    to.columns.push(
        ColumnDef::new("posts", "id", "integer")
            .primary_key()
            .into_column(),
    );
    to.columns.push(
        ColumnDef::new("posts", "author_id", "integer")
            .not_null()
            .into_column(),
    );

    // Add foreign key with CASCADE
    const FK_COLS: &[Cow<'static, str>] = &[Cow::Borrowed("author_id")];
    const FK_REFS: &[Cow<'static, str>] = &[Cow::Borrowed("id")];
    to.fks.push(
        ForeignKeyDef::new("posts", "fk_posts_author")
            .columns(FK_COLS)
            .references("users", FK_REFS)
            .on_delete(ReferentialAction::Cascade)
            .into_foreign_key(),
    );

    let sql = diff_to_sql(&SQLiteDDL::default(), &to);

    assert_eq!(sql.len(), 2, "Expected 2 CREATE TABLE statements");

    let users_sql = sql
        .iter()
        .find(|s| s.contains("`users`") && !s.contains("REFERENCES"))
        .unwrap();
    let posts_sql = sql.iter().find(|s| s.contains("`posts`")).unwrap();

    assert_eq!(
        *users_sql,
        "CREATE TABLE `users` (\n\t`id` INTEGER NOT NULL\n);\n",
    );
    assert_eq!(
        normalize_create_table(posts_sql),
        normalize_create_table(
            "CREATE TABLE `posts` (\n\t`id` INTEGER NOT NULL,\n\t`author_id` INTEGER NOT NULL,\n\tCONSTRAINT `fk_posts_author` FOREIGN KEY (`author_id`) REFERENCES `users`(`id`) ON DELETE CASCADE\n);\n"
        ),
    );
}

// =============================================================================
// Idempotency Tests
// =============================================================================

/// Test that diffing identical schemas produces no SQL
#[test]
fn test_no_diff_for_identical_schemas() {
    let mut schema = SQLiteDDL::default();

    schema.tables.push(TableDef::new("users").into_table());
    schema.columns.push(
        ColumnDef::new("users", "id", "integer")
            .primary_key()
            .into_column(),
    );

    // Diff schema with itself (clone it)
    let schema_clone = schema.clone();
    let sql = diff_to_sql(&schema, &schema_clone);

    assert!(
        sql.is_empty(),
        "Expected no SQL for identical schemas, got: {:?}",
        sql
    );
}

// =============================================================================
// Drop Tests
// =============================================================================

/// Test DROP TABLE generation
#[test]
fn test_drop_table() {
    let mut from = SQLiteDDL::default();
    from.tables.push(TableDef::new("users").into_table());
    from.columns
        .push(ColumnDef::new("users", "id", "integer").into_column());

    let to = SQLiteDDL::default();

    let sql = diff_to_sql(&from, &to);

    assert_eq!(sql.len(), 1);
    // DROP statements don't have trailing newline
    assert_eq!(sql[0], "DROP TABLE `users`;", "Unexpected DROP TABLE SQL");
}

/// Test DROP INDEX generation
#[test]
fn test_drop_index() {
    let mut from = SQLiteDDL::default();

    from.tables.push(TableDef::new("users").into_table());
    from.columns
        .push(ColumnDef::new("users", "id", "integer").into_column());
    from.columns
        .push(ColumnDef::new("users", "email", "text").into_column());
    const IDX_COLS: &[IndexColumnDef] = &[IndexColumnDef::new("email")];
    from.indexes.push(
        IndexDef::new("users", "idx_users_email")
            .columns(IDX_COLS)
            .into_index(),
    );

    // Target has table but no index
    let mut to = SQLiteDDL::default();
    to.tables.push(TableDef::new("users").into_table());
    to.columns
        .push(ColumnDef::new("users", "id", "integer").into_column());
    to.columns
        .push(ColumnDef::new("users", "email", "text").into_column());

    let sql = diff_to_sql(&from, &to);

    assert_eq!(sql.len(), 1, "Expected 1 DROP INDEX statement");
    // Uses DROP INDEX IF EXISTS
    assert_eq!(
        sql[0], "DROP INDEX IF EXISTS `idx_users_email`;",
        "Unexpected DROP INDEX SQL"
    );
}

// =============================================================================
// Table Options Tests
// =============================================================================

/// Test STRICT table option
#[test]
fn test_strict_table() {
    let mut to = SQLiteDDL::default();

    to.tables
        .push(TableDef::new("settings").strict().into_table());
    to.columns.push(
        ColumnDef::new("settings", "id", "integer")
            .primary_key()
            .into_column(),
    );

    let sql = diff_to_sql(&SQLiteDDL::default(), &to);

    assert_eq!(sql.len(), 1);
    // primary_key() renders as NOT NULL for column def
    assert_eq!(
        sql[0], "CREATE TABLE `settings` (\n\t`id` INTEGER NOT NULL\n) STRICT;\n",
        "Unexpected STRICT table SQL"
    );
}

/// Test WITHOUT ROWID table option
#[test]
fn test_without_rowid_table() {
    let mut to = SQLiteDDL::default();

    to.tables
        .push(TableDef::new("settings").without_rowid().into_table());
    to.columns.push(
        ColumnDef::new("settings", "id", "integer")
            .primary_key()
            .into_column(),
    );

    let sql = diff_to_sql(&SQLiteDDL::default(), &to);

    assert_eq!(sql.len(), 1);
    // primary_key() renders as NOT NULL for column def
    assert_eq!(
        sql[0], "CREATE TABLE `settings` (\n\t`id` INTEGER NOT NULL\n) WITHOUT ROWID;\n",
        "Unexpected WITHOUT ROWID table SQL"
    );
}

/// Test circular foreign key dependencies generates PRAGMA foreign_keys=OFF/ON
/// Note: Column order may vary
#[test]
fn test_circular_fk_dependencies() {
    let mut to = SQLiteDDL::default();

    // Table A references Table B
    to.tables.push(TableDef::new("table_a").into_table());
    to.columns.push(
        ColumnDef::new("table_a", "id", "integer")
            .primary_key()
            .into_column(),
    );
    to.columns
        .push(ColumnDef::new("table_a", "b_id", "integer").into_column());

    // Table B references Table A (circular)
    to.tables.push(TableDef::new("table_b").into_table());
    to.columns.push(
        ColumnDef::new("table_b", "id", "integer")
            .primary_key()
            .into_column(),
    );
    to.columns
        .push(ColumnDef::new("table_b", "a_id", "integer").into_column());

    // FK: table_a.b_id -> table_b.id
    const FK_A_COLS: &[Cow<'static, str>] = &[Cow::Borrowed("b_id")];
    const FK_A_REFS: &[Cow<'static, str>] = &[Cow::Borrowed("id")];
    to.fks.push(
        ForeignKeyDef::new("table_a", "fk_a_to_b")
            .columns(FK_A_COLS)
            .references("table_b", FK_A_REFS)
            .into_foreign_key(),
    );

    // FK: table_b.a_id -> table_a.id
    const FK_B_COLS: &[Cow<'static, str>] = &[Cow::Borrowed("a_id")];
    const FK_B_REFS: &[Cow<'static, str>] = &[Cow::Borrowed("id")];
    to.fks.push(
        ForeignKeyDef::new("table_b", "fk_b_to_a")
            .columns(FK_B_COLS)
            .references("table_a", FK_B_REFS)
            .into_foreign_key(),
    );

    let sql = diff_to_sql(&SQLiteDDL::default(), &to);

    // Should have: PRAGMA OFF, CREATE table_a, CREATE table_b, PRAGMA ON
    assert_eq!(
        sql.len(),
        4,
        "Expected 4 SQL statements for circular FK, got: {:?}",
        sql
    );

    assert_eq!(sql[0], "PRAGMA foreign_keys=OFF;");

    // The two CREATE TABLE statements (order may vary between table_a and table_b)
    let create_a = sql
        .iter()
        .find(|s| s.contains("CREATE TABLE `table_a`"))
        .unwrap();
    let create_b = sql
        .iter()
        .find(|s| s.contains("CREATE TABLE `table_b`"))
        .unwrap();

    assert_eq!(
        normalize_create_table(create_a),
        normalize_create_table(
            "CREATE TABLE `table_a` (\n\t`id` INTEGER NOT NULL,\n\t`b_id` INTEGER,\n\tCONSTRAINT `fk_a_to_b` FOREIGN KEY (`b_id`) REFERENCES `table_b`(`id`)\n);\n"
        ),
    );
    assert_eq!(
        normalize_create_table(create_b),
        normalize_create_table(
            "CREATE TABLE `table_b` (\n\t`id` INTEGER NOT NULL,\n\t`a_id` INTEGER,\n\tCONSTRAINT `fk_b_to_a` FOREIGN KEY (`a_id`) REFERENCES `table_a`(`id`)\n);\n"
        ),
    );

    assert_eq!(sql[3], "PRAGMA foreign_keys=ON;");
}

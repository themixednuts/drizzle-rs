//! SQLite Column Alteration Tests
//!
//! These tests mirror the patterns from drizzle-orm's drizzle-kit/tests/sqlite/sqlite-columns.test.ts
//! They verify that column alterations generate proper table recreation SQL since SQLite
//! doesn't support ALTER COLUMN.
//!
//! SQLite has limited ALTER TABLE support:
//! - ADD COLUMN (with restrictions)
//! - DROP COLUMN (SQLite 3.35.0+)
//! - RENAME COLUMN
//!
//! For changes to column properties (type, notNull, default, autoincrement), we must
//! recreate the entire table using the "12-step" migration pattern:
//! 1. PRAGMA foreign_keys=OFF
//! 2. CREATE TABLE __new_tablename (with new schema)
//! 3. INSERT INTO __new_tablename SELECT ... FROM tablename
//! 4. DROP TABLE tablename
//! 5. ALTER TABLE __new_tablename RENAME TO tablename
//! 6. PRAGMA foreign_keys=ON

use std::borrow::Cow;

use drizzle_migrations::sqlite::{
    SQLiteDDL, compute_migration,
    ddl::{ColumnDef, ForeignKeyDef, IndexColumnDef, IndexDef, PrimaryKeyDef, TableDef},
    statements::JsonStatement,
};

// =============================================================================
// Helper Functions
// =============================================================================

/// Compute migration SQL statements from two DDL states
fn diff_sql(from: &SQLiteDDL, to: &SQLiteDDL) -> Vec<String> {
    let migration = compute_migration(from, to);
    migration.sql_statements
}

/// Check if statements include table recreation pattern
fn has_recreate_pattern(sql: &[String]) -> bool {
    let combined = sql.join("\n");
    combined.contains("PRAGMA foreign_keys=OFF")
        && combined.contains("__new_")
        && combined.contains("DROP TABLE")
        && combined.contains("RENAME TO")
        && combined.contains("PRAGMA foreign_keys=ON")
}

/// Check if migration has RecreateTable statement
fn has_recreate_table_statement(from: &SQLiteDDL, to: &SQLiteDDL) -> bool {
    let migration = compute_migration(from, to);
    migration
        .statements
        .iter()
        .any(|s| matches!(s, JsonStatement::RecreateTable(_)))
}

// =============================================================================
// ADD COLUMN Tests (matches drizzle-orm "add columns #1-6")
// =============================================================================

/// Test #1: Add single column NOT NULL
#[test]
fn test_add_column_not_null() {
    let mut from = SQLiteDDL::default();
    from.tables.push(TableDef::new("users").into_table());
    from.columns.push(
        ColumnDef::new("users", "id", "integer")
            .primary_key()
            .autoincrement()
            .into_column(),
    );

    let mut to = SQLiteDDL::default();
    to.tables.push(TableDef::new("users").into_table());
    to.columns.push(
        ColumnDef::new("users", "id", "integer")
            .primary_key()
            .autoincrement()
            .into_column(),
    );
    to.columns.push(
        ColumnDef::new("users", "name", "text")
            .not_null()
            .into_column(),
    );

    let sql = diff_sql(&from, &to);

    assert_eq!(sql.len(), 1, "Expected 1 SQL statement, got: {:?}", sql);
    assert_eq!(
        sql[0], "ALTER TABLE `users` ADD `name` TEXT NOT NULL;",
        "Unexpected ALTER TABLE ADD SQL"
    );
}

/// Test #2: Add multiple columns
#[test]
fn test_add_multiple_columns() {
    let mut from = SQLiteDDL::default();
    from.tables.push(TableDef::new("users").into_table());
    from.columns.push(
        ColumnDef::new("users", "id", "integer")
            .primary_key()
            .autoincrement()
            .into_column(),
    );

    let mut to = SQLiteDDL::default();
    to.tables.push(TableDef::new("users").into_table());
    to.columns.push(
        ColumnDef::new("users", "id", "integer")
            .primary_key()
            .autoincrement()
            .into_column(),
    );
    to.columns
        .push(ColumnDef::new("users", "name", "text").into_column());
    to.columns
        .push(ColumnDef::new("users", "email", "text").into_column());

    let sql = diff_sql(&from, &to);

    assert_eq!(sql.len(), 2, "Expected 2 SQL statements, got: {:?}", sql);

    // Order may vary
    let has_name = sql
        .iter()
        .any(|s| *s == "ALTER TABLE `users` ADD `name` TEXT;");
    let has_email = sql
        .iter()
        .any(|s| *s == "ALTER TABLE `users` ADD `email` TEXT;");

    assert!(
        has_name,
        "Should have ALTER TABLE ADD `name`, got: {:?}",
        sql
    );
    assert!(
        has_email,
        "Should have ALTER TABLE ADD `email`, got: {:?}",
        sql
    );
}

/// Test #3: Add columns with various modifiers
#[test]
fn test_add_columns_with_modifiers() {
    let mut from = SQLiteDDL::default();
    from.tables.push(TableDef::new("users").into_table());
    from.columns.push(
        ColumnDef::new("users", "id", "integer")
            .primary_key()
            .autoincrement()
            .into_column(),
    );

    let mut to = SQLiteDDL::default();
    to.tables.push(TableDef::new("users").into_table());
    to.columns.push(
        ColumnDef::new("users", "id", "integer")
            .primary_key()
            .autoincrement()
            .into_column(),
    );
    to.columns.push(
        ColumnDef::new("users", "name1", "text")
            .default_value("'name'")
            .into_column(),
    );
    to.columns.push(
        ColumnDef::new("users", "name2", "text")
            .not_null()
            .into_column(),
    );
    to.columns.push(
        ColumnDef::new("users", "name3", "text")
            .default_value("'name'")
            .not_null()
            .into_column(),
    );

    let sql = diff_sql(&from, &to);

    assert_eq!(sql.len(), 3, "Expected 3 SQL statements, got: {:?}", sql);

    let has_name1 = sql
        .iter()
        .any(|s| *s == "ALTER TABLE `users` ADD `name1` TEXT DEFAULT 'name';");
    let has_name2 = sql
        .iter()
        .any(|s| *s == "ALTER TABLE `users` ADD `name2` TEXT NOT NULL;");
    let has_name3 = sql
        .iter()
        .any(|s| *s == "ALTER TABLE `users` ADD `name3` TEXT DEFAULT 'name' NOT NULL;");

    assert!(has_name1, "Should have name1 with DEFAULT, got: {:?}", sql);
    assert!(has_name2, "Should have name2 with NOT NULL, got: {:?}", sql);
    assert!(
        has_name3,
        "Should have name3 with DEFAULT and NOT NULL, got: {:?}",
        sql
    );
}

/// Test #6: Add column to table that already has unique column
#[test]
fn test_add_column_to_table_with_unique() {
    let mut from = SQLiteDDL::default();
    from.tables.push(TableDef::new("users").into_table());
    from.columns.push(
        ColumnDef::new("users", "id", "integer")
            .primary_key()
            .autoincrement()
            .into_column(),
    );
    from.columns
        .push(ColumnDef::new("users", "name", "text").into_column());
    from.columns.push(
        ColumnDef::new("users", "email", "text")
            .unique()
            .not_null()
            .into_column(),
    );

    let mut to = SQLiteDDL::default();
    to.tables.push(TableDef::new("users").into_table());
    to.columns.push(
        ColumnDef::new("users", "id", "integer")
            .primary_key()
            .autoincrement()
            .into_column(),
    );
    to.columns
        .push(ColumnDef::new("users", "name", "text").into_column());
    to.columns.push(
        ColumnDef::new("users", "email", "text")
            .unique()
            .not_null()
            .into_column(),
    );
    to.columns.push(
        ColumnDef::new("users", "password", "text")
            .not_null()
            .into_column(),
    );

    let sql = diff_sql(&from, &to);

    assert_eq!(sql.len(), 1, "Expected 1 SQL statement, got: {:?}", sql);
    assert_eq!(
        sql[0], "ALTER TABLE `users` ADD `password` TEXT NOT NULL;",
        "Unexpected ALTER TABLE ADD SQL"
    );
}

// =============================================================================
// DROP COLUMN Tests
// =============================================================================

/// Test: Drop column
#[test]
fn test_drop_column() {
    let mut from = SQLiteDDL::default();
    from.tables.push(TableDef::new("users").into_table());
    from.columns.push(
        ColumnDef::new("users", "id", "integer")
            .primary_key()
            .autoincrement()
            .into_column(),
    );
    from.columns
        .push(ColumnDef::new("users", "name", "text").into_column());

    let mut to = SQLiteDDL::default();
    to.tables.push(TableDef::new("users").into_table());
    to.columns.push(
        ColumnDef::new("users", "id", "integer")
            .primary_key()
            .autoincrement()
            .into_column(),
    );

    let sql = diff_sql(&from, &to);

    assert_eq!(sql.len(), 1, "Expected 1 SQL statement, got: {:?}", sql);
    assert_eq!(
        sql[0], "ALTER TABLE `users` DROP COLUMN `name`;",
        "Unexpected DROP COLUMN SQL"
    );
}

// =============================================================================
// ALTER COLUMN Tests - These require table recreation
// =============================================================================

/// Test: Alter column drop NOT NULL (make nullable)
#[test]
fn test_alter_column_drop_not_null() {
    let mut from = SQLiteDDL::default();
    from.tables.push(TableDef::new("table").into_table());
    from.columns.push(
        ColumnDef::new("table", "name", "text")
            .not_null()
            .into_column(),
    );

    let mut to = SQLiteDDL::default();
    to.tables.push(TableDef::new("table").into_table());
    to.columns
        .push(ColumnDef::new("table", "name", "text").into_column()); // nullable

    let sql = diff_sql(&from, &to);

    assert!(
        has_recreate_pattern(&sql),
        "Expected table recreation pattern, got: {:?}",
        sql
    );
    assert!(
        has_recreate_table_statement(&from, &to),
        "Expected RecreateTable statement"
    );

    // Verify the complete sequence
    assert_eq!(
        sql.len(),
        6,
        "Expected 6 SQL statements for table recreation, got: {:?}",
        sql
    );
    assert_eq!(
        sql[0], "PRAGMA foreign_keys=OFF;",
        "Should start with PRAGMA OFF"
    );
    assert!(
        sql[1].contains("CREATE TABLE `__new_table`"),
        "Second should be CREATE TABLE __new_table"
    );
    assert!(
        sql[1].contains("`name` TEXT"),
        "Should have nullable name column"
    );
    assert!(!sql[1].contains("NOT NULL"), "Should NOT have NOT NULL");
    assert!(
        sql[2].contains("INSERT INTO `__new_table`"),
        "Third should be INSERT"
    );
    assert!(
        sql[3].contains("DROP TABLE `table`"),
        "Fourth should be DROP TABLE"
    );
    assert!(
        sql[4].contains("ALTER TABLE `__new_table` RENAME TO `table`"),
        "Fifth should be RENAME"
    );
    assert_eq!(
        sql[5], "PRAGMA foreign_keys=ON;",
        "Should end with PRAGMA ON"
    );
}

/// Test: Alter column add NOT NULL (make non-nullable)
#[test]
fn test_alter_column_add_not_null() {
    let mut from = SQLiteDDL::default();
    from.tables.push(TableDef::new("table").into_table());
    from.columns
        .push(ColumnDef::new("table", "name", "text").into_column()); // nullable

    let mut to = SQLiteDDL::default();
    to.tables.push(TableDef::new("table").into_table());
    to.columns.push(
        ColumnDef::new("table", "name", "text")
            .not_null()
            .into_column(),
    );

    let sql = diff_sql(&from, &to);

    assert!(
        has_recreate_pattern(&sql),
        "Expected table recreation pattern, got: {:?}",
        sql
    );

    // Verify NOT NULL is in the recreated table
    let create_stmt = sql
        .iter()
        .find(|s| s.contains("CREATE TABLE `__new_table`"))
        .unwrap();
    assert!(
        create_stmt.contains("NOT NULL"),
        "Expected NOT NULL in recreated table, got: {}",
        create_stmt
    );
}

/// Test: Alter column add default
#[test]
fn test_alter_column_add_default() {
    let mut from = SQLiteDDL::default();
    from.tables.push(TableDef::new("table").into_table());
    from.columns
        .push(ColumnDef::new("table", "name", "text").into_column());

    let mut to = SQLiteDDL::default();
    to.tables.push(TableDef::new("table").into_table());
    to.columns.push(
        ColumnDef::new("table", "name", "text")
            .default_value("'dan'")
            .into_column(),
    );

    let sql = diff_sql(&from, &to);

    assert!(
        has_recreate_pattern(&sql),
        "Expected table recreation pattern, got: {:?}",
        sql
    );

    let create_stmt = sql
        .iter()
        .find(|s| s.contains("CREATE TABLE `__new_table`"))
        .unwrap();
    assert!(
        create_stmt.contains("DEFAULT 'dan'"),
        "Expected DEFAULT 'dan' in recreated table, got: {}",
        create_stmt
    );
}

/// Test: Alter column drop default
#[test]
fn test_alter_column_drop_default() {
    let mut from = SQLiteDDL::default();
    from.tables.push(TableDef::new("table").into_table());
    from.columns.push(
        ColumnDef::new("table", "name", "text")
            .default_value("'dan'")
            .into_column(),
    );

    let mut to = SQLiteDDL::default();
    to.tables.push(TableDef::new("table").into_table());
    to.columns
        .push(ColumnDef::new("table", "name", "text").into_column());

    let sql = diff_sql(&from, &to);

    assert!(
        has_recreate_pattern(&sql),
        "Expected table recreation pattern, got: {:?}",
        sql
    );

    let create_stmt = sql
        .iter()
        .find(|s| s.contains("CREATE TABLE `__new_table`"))
        .unwrap();
    // The new table should NOT have DEFAULT for the name column
    assert!(
        !create_stmt.contains("DEFAULT 'dan'"),
        "Should NOT have DEFAULT 'dan' in recreated table, got: {}",
        create_stmt
    );
}

/// Test: Alter column add default and NOT NULL together
#[test]
fn test_alter_column_add_default_not_null() {
    let mut from = SQLiteDDL::default();
    from.tables.push(TableDef::new("table").into_table());
    from.columns
        .push(ColumnDef::new("table", "name", "text").into_column());

    let mut to = SQLiteDDL::default();
    to.tables.push(TableDef::new("table").into_table());
    to.columns.push(
        ColumnDef::new("table", "name", "text")
            .not_null()
            .default_value("'dan'")
            .into_column(),
    );

    let sql = diff_sql(&from, &to);

    assert!(
        has_recreate_pattern(&sql),
        "Expected table recreation pattern, got: {:?}",
        sql
    );

    let create_stmt = sql
        .iter()
        .find(|s| s.contains("CREATE TABLE `__new_table`"))
        .unwrap();
    assert!(
        create_stmt.contains("DEFAULT 'dan'") && create_stmt.contains("NOT NULL"),
        "Expected DEFAULT 'dan' NOT NULL in recreated table, got: {}",
        create_stmt
    );
}

/// Test: Alter column drop both default and NOT NULL
#[test]
fn test_alter_column_drop_default_not_null() {
    let mut from = SQLiteDDL::default();
    from.tables.push(TableDef::new("table").into_table());
    from.columns.push(
        ColumnDef::new("table", "name", "text")
            .not_null()
            .default_value("'dan'")
            .into_column(),
    );

    let mut to = SQLiteDDL::default();
    to.tables.push(TableDef::new("table").into_table());
    to.columns
        .push(ColumnDef::new("table", "name", "text").into_column());

    let sql = diff_sql(&from, &to);

    assert!(
        has_recreate_pattern(&sql),
        "Expected table recreation pattern, got: {:?}",
        sql
    );
}

/// Test: Alter column type change
#[test]
fn test_alter_column_type_change() {
    let mut from = SQLiteDDL::default();
    from.tables.push(TableDef::new("users").into_table());
    from.columns
        .push(ColumnDef::new("users", "age", "text").into_column());

    let mut to = SQLiteDDL::default();
    to.tables.push(TableDef::new("users").into_table());
    to.columns
        .push(ColumnDef::new("users", "age", "integer").into_column());

    let sql = diff_sql(&from, &to);

    assert!(
        has_recreate_pattern(&sql),
        "Expected table recreation pattern for type change, got: {:?}",
        sql
    );

    let create_stmt = sql
        .iter()
        .find(|s| s.contains("CREATE TABLE `__new_users`"))
        .unwrap();
    assert!(
        create_stmt.to_lowercase().contains("integer"),
        "Expected INTEGER type in recreated table, got: {}",
        create_stmt
    );
}

/// Test: Drop autoincrement requires table recreation
#[test]
fn test_drop_autoincrement() {
    let mut from = SQLiteDDL::default();
    from.tables.push(TableDef::new("companies").into_table());
    from.columns.push(
        ColumnDef::new("companies", "id", "integer")
            .primary_key()
            .autoincrement()
            .into_column(),
    );
    from.columns
        .push(ColumnDef::new("companies", "name", "text").into_column());

    let mut to = SQLiteDDL::default();
    to.tables.push(TableDef::new("companies").into_table());
    to.columns.push(
        ColumnDef::new("companies", "id", "integer")
            .primary_key()
            .into_column(),
    );

    let sql = diff_sql(&from, &to);

    assert!(
        has_recreate_pattern(&sql),
        "Expected table recreation pattern for dropping autoincrement, got: {:?}",
        sql
    );

    let create_stmt = sql
        .iter()
        .find(|s| s.contains("CREATE TABLE `__new_companies`"))
        .unwrap();
    assert!(
        create_stmt.contains("__new_companies"),
        "Expected __new_companies, got: {}",
        create_stmt
    );
}

// =============================================================================
// FOREIGN KEY Tests - These require table recreation
// =============================================================================

/// Test: Add foreign key (requires table recreation)
#[test]
fn test_add_foreign_key() {
    let mut from = SQLiteDDL::default();
    from.tables.push(TableDef::new("users").into_table());
    from.columns.push(
        ColumnDef::new("users", "id", "integer")
            .primary_key()
            .autoincrement()
            .into_column(),
    );
    from.columns
        .push(ColumnDef::new("users", "report_to", "integer").into_column());

    let users = TableDef::new("users").into_table();
    let mut to = SQLiteDDL::default();
    to.tables.push(users);
    to.columns.push(
        ColumnDef::new("users", "id", "integer")
            .primary_key()
            .autoincrement()
            .into_column(),
    );
    to.columns
        .push(ColumnDef::new("users", "report_to", "integer").into_column());

    // Add self-referencing foreign key
    const FK_COLS: &[Cow<'static, str>] = &[Cow::Borrowed("report_to")];
    const FK_REFS: &[Cow<'static, str>] = &[Cow::Borrowed("id")];
    to.fks.push(
        ForeignKeyDef::new("users", "fk_users_report_to_users_id_fk")
            .columns(FK_COLS)
            .references("users", FK_REFS)
            .into_foreign_key(),
    );

    let sql = diff_sql(&from, &to);

    assert!(
        has_recreate_pattern(&sql),
        "Expected table recreation pattern for adding FK, got: {:?}",
        sql
    );

    let create_stmt = sql
        .iter()
        .find(|s| s.contains("CREATE TABLE `__new_users`"))
        .unwrap();
    assert!(
        create_stmt.contains("FOREIGN KEY") && create_stmt.contains("REFERENCES"),
        "Expected FOREIGN KEY REFERENCES in recreated table, got: {}",
        create_stmt
    );
}

// =============================================================================
// PRIMARY KEY Tests - These require table recreation
// =============================================================================

/// Test: Add composite primary key (requires table recreation)
#[test]
fn test_add_composite_pk() {
    let mut from = SQLiteDDL::default();
    from.tables.push(TableDef::new("table").into_table());
    from.columns
        .push(ColumnDef::new("table", "id1", "integer").into_column());
    from.columns
        .push(ColumnDef::new("table", "id2", "integer").into_column());

    let mut to = SQLiteDDL::default();
    to.tables.push(TableDef::new("table").into_table());
    to.columns
        .push(ColumnDef::new("table", "id1", "integer").into_column());
    to.columns
        .push(ColumnDef::new("table", "id2", "integer").into_column());

    // Add composite primary key
    const PK_COLS: &[Cow<'static, str>] = &[Cow::Borrowed("id1"), Cow::Borrowed("id2")];
    to.pks.push(
        PrimaryKeyDef::new("table", "table_pk")
            .columns(PK_COLS)
            .into_primary_key(),
    );

    let sql = diff_sql(&from, &to);

    assert!(
        has_recreate_pattern(&sql),
        "Expected table recreation pattern for adding PK, got: {:?}",
        sql
    );

    let create_stmt = sql
        .iter()
        .find(|s| s.contains("CREATE TABLE `__new_table`"))
        .unwrap();
    assert!(
        create_stmt.contains("PRIMARY KEY"),
        "Expected PRIMARY KEY in recreated table, got: {}",
        create_stmt
    );
}

// =============================================================================
// Generated Column Tests
// =============================================================================

/// Test: Add generated stored column (requires table recreation)
#[test]
fn test_add_generated_stored_column() {
    let mut from = SQLiteDDL::default();
    from.tables.push(TableDef::new("users").into_table());
    from.columns
        .push(ColumnDef::new("users", "id", "integer").into_column());

    let mut to = SQLiteDDL::default();
    to.tables.push(TableDef::new("users").into_table());
    to.columns
        .push(ColumnDef::new("users", "id", "integer").into_column());
    to.columns.push(
        ColumnDef::new("users", "gen_name", "text")
            .generated_stored("123")
            .into_column(),
    );

    let sql = diff_sql(&from, &to);

    assert!(
        has_recreate_pattern(&sql),
        "Expected table recreation pattern for STORED generated column, got: {:?}",
        sql
    );

    let create_stmt = sql
        .iter()
        .find(|s| s.contains("CREATE TABLE `__new_users`"))
        .unwrap();
    assert!(
        create_stmt.contains("GENERATED ALWAYS AS") && create_stmt.contains("STORED"),
        "Expected GENERATED ALWAYS AS ... STORED, got: {}",
        create_stmt
    );
}

/// Test: Add generated virtual column (can use ALTER TABLE ADD)
#[test]
fn test_add_generated_virtual_column() {
    let mut from = SQLiteDDL::default();
    from.tables.push(TableDef::new("users").into_table());
    from.columns
        .push(ColumnDef::new("users", "id", "integer").into_column());

    let mut to = SQLiteDDL::default();
    to.tables.push(TableDef::new("users").into_table());
    to.columns
        .push(ColumnDef::new("users", "id", "integer").into_column());
    to.columns.push(
        ColumnDef::new("users", "gen_name", "text")
            .generated_virtual("123")
            .into_column(),
    );

    let sql = diff_sql(&from, &to);

    assert_eq!(
        sql.len(),
        1,
        "Virtual columns can be added via ALTER TABLE ADD"
    );
    assert!(
        sql[0].contains("ALTER TABLE") && sql[0].contains("ADD"),
        "Expected ALTER TABLE ADD for VIRTUAL column, got: {}",
        sql[0]
    );
    assert!(
        sql[0].contains("VIRTUAL"),
        "Expected VIRTUAL keyword, got: {}",
        sql[0]
    );
}

// =============================================================================
// Combined alteration tests (multiple tables)
// =============================================================================

/// Test: Alter multiple tables simultaneously
#[test]
fn test_alter_column_multiple_tables() {
    let mut from = SQLiteDDL::default();
    from.tables.push(TableDef::new("users").into_table());
    from.columns.push(
        ColumnDef::new("users", "id", "integer")
            .primary_key()
            .autoincrement()
            .into_column(),
    );
    from.columns.push(
        ColumnDef::new("users", "name", "text")
            .not_null()
            .into_column(),
    );

    from.tables.push(TableDef::new("posts").into_table());
    from.columns.push(
        ColumnDef::new("posts", "id", "integer")
            .primary_key()
            .autoincrement()
            .into_column(),
    );
    from.columns
        .push(ColumnDef::new("posts", "name", "text").into_column());
    from.columns
        .push(ColumnDef::new("posts", "user_id", "integer").into_column());

    let mut to = SQLiteDDL::default();
    // users: make name nullable
    to.tables.push(TableDef::new("users").into_table());
    to.columns.push(
        ColumnDef::new("users", "id", "integer")
            .primary_key()
            .autoincrement()
            .into_column(),
    );
    to.columns
        .push(ColumnDef::new("users", "name", "text").into_column()); // nullable now

    // posts: make name NOT NULL
    to.tables.push(TableDef::new("posts").into_table());
    to.columns.push(
        ColumnDef::new("posts", "id", "integer")
            .primary_key()
            .autoincrement()
            .into_column(),
    );
    to.columns.push(
        ColumnDef::new("posts", "name", "text")
            .not_null()
            .into_column(),
    ); // NOT NULL now
    to.columns
        .push(ColumnDef::new("posts", "user_id", "integer").into_column());

    let sql = diff_sql(&from, &to);

    let combined = sql.join("\n");

    // Both tables should be recreated
    assert!(
        combined.contains("__new_users"),
        "Expected __new_users recreation, got: {}",
        combined
    );
    assert!(
        combined.contains("__new_posts"),
        "Expected __new_posts recreation, got: {}",
        combined
    );
}

// =============================================================================
// Data Preservation Tests
// =============================================================================

/// Test: Recreation preserves data columns
#[test]
fn test_recreate_preserves_columns() {
    let mut from = SQLiteDDL::default();
    from.tables.push(TableDef::new("users").into_table());
    from.columns
        .push(ColumnDef::new("users", "id", "integer").into_column());
    from.columns
        .push(ColumnDef::new("users", "name", "text").into_column());
    from.columns
        .push(ColumnDef::new("users", "age", "integer").into_column());

    let mut to = SQLiteDDL::default();
    to.tables.push(TableDef::new("users").into_table());
    to.columns
        .push(ColumnDef::new("users", "id", "integer").into_column());
    to.columns.push(
        ColumnDef::new("users", "name", "text")
            .not_null()
            .into_column(),
    ); // Change
    to.columns
        .push(ColumnDef::new("users", "age", "integer").into_column());

    let sql = diff_sql(&from, &to);

    // Find the INSERT statement
    let insert_stmt = sql
        .iter()
        .find(|s| s.contains("INSERT INTO `__new_users`"))
        .unwrap();

    // Should include column references in the INSERT
    assert!(
        insert_stmt.contains("`id`")
            && insert_stmt.contains("`name`")
            && insert_stmt.contains("`age`"),
        "Expected all columns in INSERT, got: {}",
        insert_stmt
    );
}

// =============================================================================
// Index Recreation Tests
// =============================================================================

/// Test: Indexes are recreated after table recreation
#[test]
fn test_recreate_with_indexes() {
    let mut from = SQLiteDDL::default();
    from.tables.push(TableDef::new("table").into_table());
    from.columns
        .push(ColumnDef::new("table", "name", "text").into_column());

    const IDX_COLS: &[IndexColumnDef] = &[IndexColumnDef::new("name")];
    from.indexes.push(
        IndexDef::new("table", "index_name")
            .columns(IDX_COLS)
            .into_index(),
    );

    let mut to = SQLiteDDL::default();
    to.tables.push(TableDef::new("table").into_table());
    to.columns.push(
        ColumnDef::new("table", "name", "text")
            .not_null()
            .default_value("'dan'")
            .into_column(),
    );

    const IDX_COLS2: &[IndexColumnDef] = &[IndexColumnDef::new("name")];
    to.indexes.push(
        IndexDef::new("table", "index_name")
            .columns(IDX_COLS2)
            .into_index(),
    );

    let sql = diff_sql(&from, &to);
    let combined = sql.join("\n");

    // Table should be recreated
    assert!(
        combined.contains("__new_table"),
        "Expected table recreation, got: {}",
        combined
    );

    // Index should be recreated after table recreation
    assert!(
        combined.contains("CREATE INDEX `index_name`"),
        "Expected CREATE INDEX after recreation, got: {}",
        combined
    );
}

// =============================================================================
// Edge Cases
// =============================================================================

/// Test: No changes produces no SQL
#[test]
fn test_no_changes_no_sql() {
    let mut schema = SQLiteDDL::default();
    schema.tables.push(TableDef::new("users").into_table());
    schema.columns.push(
        ColumnDef::new("users", "id", "integer")
            .primary_key()
            .into_column(),
    );
    schema.columns.push(
        ColumnDef::new("users", "name", "text")
            .not_null()
            .into_column(),
    );

    let sql = diff_sql(&schema, &schema.clone());

    assert!(
        sql.is_empty(),
        "Expected no SQL for identical schemas, got: {:?}",
        sql
    );
}

/// Test: Recreate table with nested references
#[test]
fn test_recreate_table_with_nested_references() {
    // Create schema with nested FK references:
    // subscriptions_metadata -> subscriptions -> users

    let mut from = SQLiteDDL::default();

    // Users table
    from.tables.push(TableDef::new("users").into_table());
    from.columns.push(
        ColumnDef::new("users", "id", "integer")
            .primary_key()
            .autoincrement()
            .into_column(),
    );
    from.columns
        .push(ColumnDef::new("users", "name", "text").into_column());
    from.columns
        .push(ColumnDef::new("users", "age", "integer").into_column());

    // Subscriptions table
    from.tables
        .push(TableDef::new("subscriptions").into_table());
    from.columns.push(
        ColumnDef::new("subscriptions", "id", "integer")
            .primary_key()
            .autoincrement()
            .into_column(),
    );
    from.columns
        .push(ColumnDef::new("subscriptions", "user_id", "integer").into_column());
    from.columns
        .push(ColumnDef::new("subscriptions", "customer_id", "text").into_column());

    // Subscriptions metadata table
    from.tables
        .push(TableDef::new("subscriptions_metadata").into_table());
    from.columns.push(
        ColumnDef::new("subscriptions_metadata", "id", "integer")
            .primary_key()
            .autoincrement()
            .into_column(),
    );
    from.columns
        .push(ColumnDef::new("subscriptions_metadata", "subscription_id", "text").into_column());

    // Add FK: subscriptions.user_id -> users.id
    const SUB_FK_COLS: &[Cow<'static, str>] = &[Cow::Borrowed("user_id")];
    const SUB_FK_REFS: &[Cow<'static, str>] = &[Cow::Borrowed("id")];
    from.fks.push(
        ForeignKeyDef::new("subscriptions", "fk_subscriptions_user_id_users_id_fk")
            .columns(SUB_FK_COLS)
            .references("users", SUB_FK_REFS)
            .into_foreign_key(),
    );

    // Add FK: subscriptions_metadata.subscription_id -> subscriptions.id
    const META_FK_COLS: &[Cow<'static, str>] = &[Cow::Borrowed("subscription_id")];
    const META_FK_REFS: &[Cow<'static, str>] = &[Cow::Borrowed("id")];
    from.fks.push(
        ForeignKeyDef::new(
            "subscriptions_metadata",
            "fk_subscriptions_metadata_subscription_id_subscriptions_id_fk",
        )
        .columns(META_FK_COLS)
        .references("subscriptions", META_FK_REFS)
        .into_foreign_key(),
    );

    // Create "to" schema: change users.id autoincrement to false
    let mut to = from.clone();
    // Find and update the users id column
    for col in to.columns.list_mut() {
        if col.table == "users" && col.name == "id" {
            col.autoincrement = None; // Remove autoincrement
        }
    }

    let sql = diff_sql(&from, &to);
    let combined = sql.join("\n");

    // Users table should be recreated
    assert!(
        combined.contains("__new_users"),
        "Expected users table recreation, got: {}",
        combined
    );

    // Should have PRAGMA wrappers
    assert!(
        combined.contains("PRAGMA foreign_keys=OFF"),
        "Expected PRAGMA foreign_keys=OFF, got: {}",
        combined
    );
}

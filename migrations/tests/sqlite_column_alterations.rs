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
        has_recreate_table_statement(&from, &to),
        "Expected RecreateTable statement"
    );

    // Verify the complete recreation sequence
    assert_eq!(
        sql.len(),
        6,
        "Expected 6 SQL statements for table recreation, got: {:?}",
        sql
    );
    assert_eq!(sql[0], "PRAGMA foreign_keys=OFF;");
    assert_eq!(sql[1], "CREATE TABLE `__new_table` (\n\t`name` TEXT\n);\n");
    assert_eq!(
        sql[2],
        "INSERT INTO `__new_table`(`name`) SELECT `name` FROM `table`;"
    );
    assert_eq!(sql[3], "DROP TABLE `table`;");
    assert_eq!(sql[4], "ALTER TABLE `__new_table` RENAME TO `table`;");
    assert_eq!(sql[5], "PRAGMA foreign_keys=ON;");
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

    assert_eq!(sql.len(), 6, "Expected 6 SQL statements, got: {:?}", sql);
    assert_eq!(sql[0], "PRAGMA foreign_keys=OFF;");
    assert_eq!(
        sql[1],
        "CREATE TABLE `__new_table` (\n\t`name` TEXT NOT NULL\n);\n"
    );
    assert_eq!(
        sql[2],
        "INSERT INTO `__new_table`(`name`) SELECT `name` FROM `table`;"
    );
    assert_eq!(sql[3], "DROP TABLE `table`;");
    assert_eq!(sql[4], "ALTER TABLE `__new_table` RENAME TO `table`;");
    assert_eq!(sql[5], "PRAGMA foreign_keys=ON;");
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

    assert_eq!(sql.len(), 6, "Expected 6 SQL statements, got: {:?}", sql);
    assert_eq!(sql[0], "PRAGMA foreign_keys=OFF;");
    assert_eq!(
        sql[1],
        "CREATE TABLE `__new_table` (\n\t`name` TEXT DEFAULT 'dan'\n);\n"
    );
    assert_eq!(
        sql[2],
        "INSERT INTO `__new_table`(`name`) SELECT `name` FROM `table`;"
    );
    assert_eq!(sql[3], "DROP TABLE `table`;");
    assert_eq!(sql[4], "ALTER TABLE `__new_table` RENAME TO `table`;");
    assert_eq!(sql[5], "PRAGMA foreign_keys=ON;");
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

    assert_eq!(sql.len(), 6, "Expected 6 SQL statements, got: {:?}", sql);
    assert_eq!(sql[0], "PRAGMA foreign_keys=OFF;");
    assert_eq!(sql[1], "CREATE TABLE `__new_table` (\n\t`name` TEXT\n);\n");
    assert_eq!(
        sql[2],
        "INSERT INTO `__new_table`(`name`) SELECT `name` FROM `table`;"
    );
    assert_eq!(sql[3], "DROP TABLE `table`;");
    assert_eq!(sql[4], "ALTER TABLE `__new_table` RENAME TO `table`;");
    assert_eq!(sql[5], "PRAGMA foreign_keys=ON;");
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

    assert_eq!(sql.len(), 6, "Expected 6 SQL statements, got: {:?}", sql);
    assert_eq!(sql[0], "PRAGMA foreign_keys=OFF;");
    assert_eq!(
        sql[1],
        "CREATE TABLE `__new_table` (\n\t`name` TEXT DEFAULT 'dan' NOT NULL\n);\n"
    );
    assert_eq!(
        sql[2],
        "INSERT INTO `__new_table`(`name`) SELECT `name` FROM `table`;"
    );
    assert_eq!(sql[3], "DROP TABLE `table`;");
    assert_eq!(sql[4], "ALTER TABLE `__new_table` RENAME TO `table`;");
    assert_eq!(sql[5], "PRAGMA foreign_keys=ON;");
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

    assert_eq!(sql.len(), 6, "Expected 6 SQL statements, got: {:?}", sql);
    assert_eq!(sql[0], "PRAGMA foreign_keys=OFF;");
    assert_eq!(sql[1], "CREATE TABLE `__new_table` (\n\t`name` TEXT\n);\n");
    assert_eq!(
        sql[2],
        "INSERT INTO `__new_table`(`name`) SELECT `name` FROM `table`;"
    );
    assert_eq!(sql[3], "DROP TABLE `table`;");
    assert_eq!(sql[4], "ALTER TABLE `__new_table` RENAME TO `table`;");
    assert_eq!(sql[5], "PRAGMA foreign_keys=ON;");
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

    assert_eq!(sql.len(), 6, "Expected 6 SQL statements, got: {:?}", sql);
    assert_eq!(sql[0], "PRAGMA foreign_keys=OFF;");
    assert_eq!(
        sql[1],
        "CREATE TABLE `__new_users` (\n\t`age` INTEGER\n);\n"
    );
    assert_eq!(
        sql[2],
        "INSERT INTO `__new_users`(`age`) SELECT `age` FROM `users`;"
    );
    assert_eq!(sql[3], "DROP TABLE `users`;");
    assert_eq!(sql[4], "ALTER TABLE `__new_users` RENAME TO `users`;");
    assert_eq!(sql[5], "PRAGMA foreign_keys=ON;");
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

    assert_eq!(sql.len(), 6, "Expected 6 SQL statements, got: {:?}", sql);
    assert_eq!(sql[0], "PRAGMA foreign_keys=OFF;");
    assert_eq!(
        sql[1],
        "CREATE TABLE `__new_companies` (\n\t`id` INTEGER NOT NULL\n);\n"
    );
    assert_eq!(
        sql[2],
        "INSERT INTO `__new_companies`(`id`) SELECT `id` FROM `companies`;"
    );
    assert_eq!(sql[3], "DROP TABLE `companies`;");
    assert_eq!(
        sql[4],
        "ALTER TABLE `__new_companies` RENAME TO `companies`;"
    );
    assert_eq!(sql[5], "PRAGMA foreign_keys=ON;");
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

    assert_eq!(sql.len(), 6, "Expected 6 SQL statements, got: {:?}", sql);
    assert_eq!(sql[0], "PRAGMA foreign_keys=OFF;");
    assert_eq!(
        sql[1],
        "CREATE TABLE `__new_users` (\n\t`id` INTEGER AUTOINCREMENT NOT NULL,\n\t`report_to` INTEGER,\n\tCONSTRAINT `fk_users_report_to_users_id_fk` FOREIGN KEY (`report_to`) REFERENCES `users`(`id`)\n);\n"
    );
    assert_eq!(
        sql[2],
        "INSERT INTO `__new_users`(`id`, `report_to`) SELECT `id`, `report_to` FROM `users`;"
    );
    assert_eq!(sql[3], "DROP TABLE `users`;");
    assert_eq!(sql[4], "ALTER TABLE `__new_users` RENAME TO `users`;");
    assert_eq!(sql[5], "PRAGMA foreign_keys=ON;");
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

    assert_eq!(sql.len(), 6, "Expected 6 SQL statements, got: {:?}", sql);
    assert_eq!(sql[0], "PRAGMA foreign_keys=OFF;");
    assert_eq!(
        sql[1],
        "CREATE TABLE `__new_table` (\n\t`id1` INTEGER,\n\t`id2` INTEGER,\n\tCONSTRAINT `table_pk` PRIMARY KEY(`id1`, `id2`)\n);\n"
    );
    assert_eq!(
        sql[2],
        "INSERT INTO `__new_table`(`id1`, `id2`) SELECT `id1`, `id2` FROM `table`;"
    );
    assert_eq!(sql[3], "DROP TABLE `table`;");
    assert_eq!(sql[4], "ALTER TABLE `__new_table` RENAME TO `table`;");
    assert_eq!(sql[5], "PRAGMA foreign_keys=ON;");
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

    assert_eq!(sql.len(), 6, "Expected 6 SQL statements, got: {:?}", sql);
    assert_eq!(sql[0], "PRAGMA foreign_keys=OFF;");
    assert_eq!(
        sql[1],
        "CREATE TABLE `__new_users` (\n\t`id` INTEGER,\n\t`gen_name` TEXT GENERATED ALWAYS AS 123 STORED\n);\n"
    );
    assert_eq!(
        sql[2],
        "INSERT INTO `__new_users`(`id`) SELECT `id` FROM `users`;"
    );
    assert_eq!(sql[3], "DROP TABLE `users`;");
    assert_eq!(sql[4], "ALTER TABLE `__new_users` RENAME TO `users`;");
    assert_eq!(sql[5], "PRAGMA foreign_keys=ON;");
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
    assert_eq!(
        sql[0],
        "ALTER TABLE `users` ADD `gen_name` TEXT GENERATED ALWAYS AS 123 VIRTUAL;"
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

    // Both tables are recreated: 6 statements per table = 12 total
    assert_eq!(sql.len(), 12, "Expected 12 SQL statements, got: {:?}", sql);

    // Table order between posts/users is non-deterministic; find each group by content
    let posts_start = sql
        .iter()
        .position(|s| s.contains("__new_posts"))
        .expect("Should contain __new_posts recreation")
        - 1; // PRAGMA OFF is one before the CREATE TABLE
    let users_start = sql
        .iter()
        .position(|s| s.contains("__new_users"))
        .expect("Should contain __new_users recreation")
        - 1;

    // Posts recreation
    assert_eq!(sql[posts_start], "PRAGMA foreign_keys=OFF;");
    assert_eq!(
        sql[posts_start + 1],
        "CREATE TABLE `__new_posts` (\n\t`id` INTEGER AUTOINCREMENT NOT NULL,\n\t`name` TEXT NOT NULL,\n\t`user_id` INTEGER\n);\n"
    );
    assert_eq!(
        sql[posts_start + 2],
        "INSERT INTO `__new_posts`(`id`, `name`, `user_id`) SELECT `id`, `name`, `user_id` FROM `posts`;"
    );
    assert_eq!(sql[posts_start + 3], "DROP TABLE `posts`;");
    assert_eq!(
        sql[posts_start + 4],
        "ALTER TABLE `__new_posts` RENAME TO `posts`;"
    );
    assert_eq!(sql[posts_start + 5], "PRAGMA foreign_keys=ON;");

    // Users recreation
    assert_eq!(sql[users_start], "PRAGMA foreign_keys=OFF;");
    assert_eq!(
        sql[users_start + 1],
        "CREATE TABLE `__new_users` (\n\t`id` INTEGER AUTOINCREMENT NOT NULL,\n\t`name` TEXT\n);\n"
    );
    assert_eq!(
        sql[users_start + 2],
        "INSERT INTO `__new_users`(`id`, `name`) SELECT `id`, `name` FROM `users`;"
    );
    assert_eq!(sql[users_start + 3], "DROP TABLE `users`;");
    assert_eq!(
        sql[users_start + 4],
        "ALTER TABLE `__new_users` RENAME TO `users`;"
    );
    assert_eq!(sql[users_start + 5], "PRAGMA foreign_keys=ON;");
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

    assert_eq!(sql.len(), 6, "Expected 6 SQL statements, got: {:?}", sql);
    assert_eq!(sql[0], "PRAGMA foreign_keys=OFF;");
    assert_eq!(
        sql[1],
        "CREATE TABLE `__new_users` (\n\t`id` INTEGER,\n\t`name` TEXT NOT NULL,\n\t`age` INTEGER\n);\n"
    );
    assert_eq!(
        sql[2],
        "INSERT INTO `__new_users`(`id`, `name`, `age`) SELECT `id`, `name`, `age` FROM `users`;"
    );
    assert_eq!(sql[3], "DROP TABLE `users`;");
    assert_eq!(sql[4], "ALTER TABLE `__new_users` RENAME TO `users`;");
    assert_eq!(sql[5], "PRAGMA foreign_keys=ON;");
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

    assert_eq!(sql.len(), 7, "Expected 7 SQL statements, got: {:?}", sql);
    assert_eq!(sql[0], "PRAGMA foreign_keys=OFF;");
    assert_eq!(
        sql[1],
        "CREATE TABLE `__new_table` (\n\t`name` TEXT DEFAULT 'dan' NOT NULL\n);\n"
    );
    assert_eq!(
        sql[2],
        "INSERT INTO `__new_table`(`name`) SELECT `name` FROM `table`;"
    );
    assert_eq!(sql[3], "DROP TABLE `table`;");
    assert_eq!(sql[4], "ALTER TABLE `__new_table` RENAME TO `table`;");
    assert_eq!(sql[5], "PRAGMA foreign_keys=ON;");
    assert_eq!(sql[6], "CREATE INDEX `index_name` ON `table` (`name`);");
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

    assert_eq!(sql.len(), 6, "Expected 6 SQL statements, got: {:?}", sql);
    assert_eq!(sql[0], "PRAGMA foreign_keys=OFF;");
    assert_eq!(
        sql[1],
        "CREATE TABLE `__new_users` (\n\t`id` INTEGER NOT NULL,\n\t`name` TEXT,\n\t`age` INTEGER\n);\n"
    );
    assert_eq!(
        sql[2],
        "INSERT INTO `__new_users`(`id`, `name`, `age`) SELECT `id`, `name`, `age` FROM `users`;"
    );
    assert_eq!(sql[3], "DROP TABLE `users`;");
    assert_eq!(sql[4], "ALTER TABLE `__new_users` RENAME TO `users`;");
    assert_eq!(sql[5], "PRAGMA foreign_keys=ON;");
}

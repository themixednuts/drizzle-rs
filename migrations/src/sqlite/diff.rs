//! Schema diff types and logic for SQLite v7 DDL format
//!
//! This module provides diffing between DDL collections and
//! generates migration statements from schema changes.

use super::SQLiteSnapshot;
use super::collection::{DiffType, EntityDiff, SQLiteDDL, diff_ddl};
use super::ddl::SqliteEntity;
use super::statements::{
    AddColumnStatement, CreateIndexStatement, CreateTableStatement, CreateViewStatement,
    DropColumnStatement, DropIndexStatement, DropTableStatement, DropViewStatement, JsonStatement,
    RecreateTableStatement, TableFull, from_json,
};
use crate::traits::EntityKind;
use std::collections::HashSet;

// Re-export diff types from collection
pub use super::collection::{DiffType as SchemaDiffType, EntityDiff as SchemaEntityDiff};

/// Complete schema diff between two snapshots
#[derive(Debug, Clone, Default)]
pub struct SchemaDiff {
    /// All entity diffs
    pub diffs: Vec<EntityDiff>,
}

impl SchemaDiff {
    /// Check if there are any changes
    pub fn has_changes(&self) -> bool {
        !self.diffs.is_empty()
    }

    /// Check if this diff is empty (no changes)
    pub fn is_empty(&self) -> bool {
        self.diffs.is_empty()
    }

    /// Get created entities
    pub fn created(&self) -> Vec<&EntityDiff> {
        self.diffs
            .iter()
            .filter(|d| d.diff_type == DiffType::Create)
            .collect()
    }

    /// Get dropped entities
    pub fn dropped(&self) -> Vec<&EntityDiff> {
        self.diffs
            .iter()
            .filter(|d| d.diff_type == DiffType::Drop)
            .collect()
    }

    /// Get altered entities
    pub fn altered(&self) -> Vec<&EntityDiff> {
        self.diffs
            .iter()
            .filter(|d| d.diff_type == DiffType::Alter)
            .collect()
    }

    /// Get diffs filtered by entity kind
    pub fn by_kind(&self, kind: EntityKind) -> Vec<&EntityDiff> {
        self.diffs.iter().filter(|d| d.kind == kind).collect()
    }

    /// Get created tables
    pub fn created_tables(&self) -> Vec<&EntityDiff> {
        self.diffs
            .iter()
            .filter(|d| d.diff_type == DiffType::Create && d.kind == EntityKind::Table)
            .collect()
    }

    /// Get dropped tables
    pub fn dropped_tables(&self) -> Vec<&EntityDiff> {
        self.diffs
            .iter()
            .filter(|d| d.diff_type == DiffType::Drop && d.kind == EntityKind::Table)
            .collect()
    }
}

/// Compare two SQLite snapshots and return the diff
pub fn diff_snapshots(prev: &SQLiteSnapshot, cur: &SQLiteSnapshot) -> SchemaDiff {
    let prev_ddl = SQLiteDDL::from_entities(prev.ddl.clone());
    let cur_ddl = SQLiteDDL::from_entities(cur.ddl.clone());

    SchemaDiff {
        diffs: diff_ddl(&prev_ddl, &cur_ddl),
    }
}

/// Compare two DDL collections directly
pub fn diff_collections(prev: &SQLiteDDL, cur: &SQLiteDDL) -> SchemaDiff {
    SchemaDiff {
        diffs: diff_ddl(prev, cur),
    }
}

// =============================================================================
// Migration Diff Result
// =============================================================================

/// A table rename operation
#[derive(Debug, Clone)]
pub struct TableRename {
    pub from: String,
    pub to: String,
}

/// A column rename operation
#[derive(Debug, Clone)]
pub struct ColumnRename {
    pub table: String,
    pub from: String,
    pub to: String,
}

/// Result of computing a migration diff
#[derive(Debug, Clone, Default)]
pub struct MigrationDiff {
    /// JSON statements for the migration
    pub statements: Vec<JsonStatement>,
    /// Generated SQL statements
    pub sql_statements: Vec<String>,
    /// Renames that occurred (for tracking in snapshot)
    pub renames: Vec<String>,
    /// Warning messages
    pub warnings: Vec<String>,
}

/// Build a TableFull from DDL for a given table name
pub fn table_from_ddl(table_name: &str, ddl: &SQLiteDDL) -> TableFull {
    let entities = ddl.table_entities(table_name);

    // Get table-level options (strict, without_rowid)
    let (strict, without_rowid) = ddl
        .tables
        .one(table_name)
        .map(|t| (t.strict, t.without_rowid))
        .unwrap_or((false, false));

    TableFull {
        name: table_name.to_string(),
        columns: entities.columns.into_iter().cloned().collect(),
        pk: entities.pk.cloned(),
        fks: entities.fks.into_iter().cloned().collect(),
        uniques: entities.uniques.into_iter().cloned().collect(),
        checks: entities.checks.into_iter().cloned().collect(),
        strict,
        without_rowid,
    }
}

/// Compute a full migration diff between two DDL states
///
/// This is a simplified version of the TypeScript ddlDiff function.
/// For a fully interactive migration with rename detection, you would
/// need to provide resolver callbacks.
pub fn compute_migration(prev: &SQLiteDDL, cur: &SQLiteDDL) -> MigrationDiff {
    let schema_diff = diff_collections(prev, cur);
    let mut statements = Vec::new();
    let mut warnings = Vec::new();
    let renames = Vec::new();

    // Track created/dropped table names
    let created_table_names: HashSet<String> = schema_diff
        .created_tables()
        .iter()
        .map(|d| d.name.clone())
        .collect();

    let dropped_table_names: HashSet<String> = schema_diff
        .dropped_tables()
        .iter()
        .map(|d| d.name.clone())
        .collect();

    // Collect tables that need recreation due to column alterations
    // SQLite doesn't support ALTER COLUMN, so we need to recreate the table
    let mut tables_to_recreate: HashSet<String> = HashSet::new();

    for col_diff in schema_diff.by_kind(EntityKind::Column) {
        if col_diff.diff_type == DiffType::Alter {
            // Extract table name from the column diff
            if let Some(SqliteEntity::Column(col)) = &col_diff.right {
                // Skip tables that are being created or dropped
                if !created_table_names.contains(col.table.as_ref())
                    && !dropped_table_names.contains(col.table.as_ref())
                {
                    tables_to_recreate.insert(col.table.to_string());
                }
            }
        }
    }

    // Also check for FK, PK, unique, check constraint changes that require recreation
    for diff in schema_diff.by_kind(EntityKind::ForeignKey) {
        if diff.diff_type == DiffType::Create
            || diff.diff_type == DiffType::Drop
            || diff.diff_type == DiffType::Alter
        {
            if let Some(table) = &diff.table {
                if !created_table_names.contains(table) && !dropped_table_names.contains(table) {
                    tables_to_recreate.insert(table.clone());
                }
            }
        }
    }

    for diff in schema_diff.by_kind(EntityKind::PrimaryKey) {
        if diff.diff_type == DiffType::Alter {
            if let Some(table) = &diff.table {
                if !created_table_names.contains(table) && !dropped_table_names.contains(table) {
                    tables_to_recreate.insert(table.clone());
                }
            }
        }
    }

    for diff in schema_diff.by_kind(EntityKind::UniqueConstraint) {
        if diff.diff_type == DiffType::Create
            || diff.diff_type == DiffType::Drop
            || diff.diff_type == DiffType::Alter
        {
            if let Some(table) = &diff.table {
                if !created_table_names.contains(table) && !dropped_table_names.contains(table) {
                    tables_to_recreate.insert(table.clone());
                }
            }
        }
    }

    for diff in schema_diff.by_kind(EntityKind::CheckConstraint) {
        if diff.diff_type == DiffType::Create
            || diff.diff_type == DiffType::Drop
            || diff.diff_type == DiffType::Alter
        {
            if let Some(table) = &diff.table {
                if !created_table_names.contains(table) && !dropped_table_names.contains(table) {
                    tables_to_recreate.insert(table.clone());
                }
            }
        }
    }

    // 1. Create tables
    for table_diff in schema_diff.created_tables() {
        if let Some(SqliteEntity::Table(table)) = &table_diff.right {
            let table_full = table_from_ddl(&table.name, cur);
            statements.push(JsonStatement::CreateTable(CreateTableStatement {
                table: table_full,
            }));
        }
    }

    // 2. Recreate tables that have column alterations
    for table_name in &tables_to_recreate {
        let from_table = table_from_ddl(table_name, prev);
        let to_table = table_from_ddl(table_name, cur);
        statements.push(JsonStatement::RecreateTable(RecreateTableStatement {
            from: from_table,
            to: to_table,
        }));
    }

    // 3. Add columns (for existing tables only, skip tables being recreated)
    for col_diff in schema_diff.by_kind(EntityKind::Column) {
        if col_diff.diff_type == DiffType::Create
            && let Some(SqliteEntity::Column(col)) = &col_diff.right
            // Skip columns for newly created tables
            && !created_table_names.contains(col.table.as_ref())
            // Skip columns for tables being recreated
            && !tables_to_recreate.contains(col.table.as_ref())
        {
            // Find associated FK if any
            let fk = cur
                .fks
                .for_table(&col.table)
                .into_iter()
                .find(|fk| fk.columns.len() == 1 && fk.columns[0] == col.name)
                .cloned();

            statements.push(JsonStatement::AddColumn(AddColumnStatement {
                column: col.clone(),
                fk,
            }));
        }
    }

    // 4. Drop indexes (skip tables being recreated - indexes will be recreated with table)
    for idx_diff in schema_diff.by_kind(EntityKind::Index) {
        if idx_diff.diff_type == DiffType::Drop
            && let Some(SqliteEntity::Index(idx)) = &idx_diff.left
            && !tables_to_recreate.contains(idx.table.as_ref())
        {
            statements.push(JsonStatement::DropIndex(DropIndexStatement {
                index: idx.clone(),
            }));
        }
    }

    // 5. Create indexes (including for newly created tables, skip tables being recreated)
    for idx_diff in schema_diff.by_kind(EntityKind::Index) {
        if idx_diff.diff_type == DiffType::Create
            && let Some(SqliteEntity::Index(idx)) = &idx_diff.right
            && !tables_to_recreate.contains(idx.table.as_ref())
        {
            statements.push(JsonStatement::CreateIndex(CreateIndexStatement {
                index: idx.clone(),
            }));
        }
    }

    // 6. Alter indexes (drop old, create new, skip tables being recreated)
    for idx_diff in schema_diff.by_kind(EntityKind::Index) {
        if idx_diff.diff_type == DiffType::Alter {
            if let Some(SqliteEntity::Index(old_idx)) = &idx_diff.left {
                if !tables_to_recreate.contains(old_idx.table.as_ref()) {
                    statements.push(JsonStatement::DropIndex(DropIndexStatement {
                        index: old_idx.clone(),
                    }));
                }
            }
            if let Some(SqliteEntity::Index(new_idx)) = &idx_diff.right {
                if !tables_to_recreate.contains(new_idx.table.as_ref()) {
                    statements.push(JsonStatement::CreateIndex(CreateIndexStatement {
                        index: new_idx.clone(),
                    }));
                }
            }
        }
    }

    // 7. Drop columns (for non-dropped tables, skip tables being recreated)
    for col_diff in schema_diff.by_kind(EntityKind::Column) {
        if col_diff.diff_type == DiffType::Drop
            && let Some(SqliteEntity::Column(col)) = &col_diff.left
            // Skip columns for dropped tables
            && !dropped_table_names.contains(col.table.as_ref())
            // Skip columns for tables being recreated
            && !tables_to_recreate.contains(col.table.as_ref())
        {
            statements.push(JsonStatement::DropColumn(DropColumnStatement {
                column: col.clone(),
            }));
        }
    }

    // 8. Drop views
    for view_diff in schema_diff.by_kind(EntityKind::View) {
        if view_diff.diff_type == DiffType::Drop
            && let Some(SqliteEntity::View(view)) = &view_diff.left
            && !view.is_existing
        {
            statements.push(JsonStatement::DropView(DropViewStatement {
                view: view.clone(),
            }));
        }
    }

    // 9. Create views
    for view_diff in schema_diff.by_kind(EntityKind::View) {
        if view_diff.diff_type == DiffType::Create
            && let Some(SqliteEntity::View(view)) = &view_diff.right
            && !view.is_existing
        {
            statements.push(JsonStatement::CreateView(CreateViewStatement {
                view: view.clone(),
            }));
        }
    }

    // 10. Alter views (drop and recreate)
    for view_diff in schema_diff.by_kind(EntityKind::View) {
        if view_diff.diff_type == DiffType::Alter {
            if let Some(SqliteEntity::View(old_view)) = &view_diff.left {
                statements.push(JsonStatement::DropView(DropViewStatement {
                    view: old_view.clone(),
                }));
            }
            if let Some(SqliteEntity::View(new_view)) = &view_diff.right {
                statements.push(JsonStatement::CreateView(CreateViewStatement {
                    view: new_view.clone(),
                }));
            }
        }
    }

    // 11. Drop tables
    for table_diff in schema_diff.dropped_tables() {
        statements.push(JsonStatement::DropTable(DropTableStatement {
            table_name: table_diff.name.clone(),
        }));
    }

    // Add warnings for STORED generated columns
    for col_diff in schema_diff.by_kind(EntityKind::Column) {
        if col_diff.diff_type == DiffType::Alter
            && let Some(SqliteEntity::Column(col)) = &col_diff.right
            && col
                .generated
                .as_ref()
                .is_some_and(|g| g.gen_type == super::ddl::GeneratedType::Stored)
        {
            warnings.push(format!(
                "Column '{}' in table '{}' has STORED generated column which requires table recreation",
                col.name, col.table
            ));
        }
    }

    // Convert to SQL
    let result = from_json(statements.clone());

    MigrationDiff {
        statements,
        sql_statements: result.sql_statements,
        renames,
        warnings,
    }
}

/// Prepare rename tracking strings for snapshot storage
pub fn prepare_migration_renames(
    table_renames: &[TableRename],
    column_renames: &[ColumnRename],
) -> Vec<String> {
    let mut renames = Vec::new();

    for tr in table_renames {
        renames.push(format!("table:{}:{}", tr.from, tr.to));
    }

    for cr in column_renames {
        renames.push(format!("column:{}:{}:{}", cr.table, cr.from, cr.to));
    }

    renames
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sqlite::ddl::{Column, SqliteEntity, Table};

    #[test]
    fn test_empty_diff() {
        let prev = SQLiteSnapshot::new();
        let cur = SQLiteSnapshot::new();

        let diff = diff_snapshots(&prev, &cur);
        assert!(!diff.has_changes());
    }

    #[test]
    fn test_table_creation() {
        let prev = SQLiteSnapshot::new();
        let mut cur = SQLiteSnapshot::new();

        cur.add_entity(SqliteEntity::Table(Table::new("users")));
        cur.add_entity(SqliteEntity::Column(
            Column::new("users", "id", "integer").not_null(),
        ));

        let diff = diff_snapshots(&prev, &cur);
        assert!(diff.has_changes());
        assert_eq!(diff.created_tables().len(), 1);
    }

    #[test]
    fn test_table_deletion() {
        let mut prev = SQLiteSnapshot::new();
        let cur = SQLiteSnapshot::new();

        prev.add_entity(SqliteEntity::Table(Table::new("users")));

        let diff = diff_snapshots(&prev, &cur);
        assert!(diff.has_changes());
        assert_eq!(diff.dropped_tables().len(), 1);
    }

    #[test]
    fn test_column_nullable_change() {
        // Test that changing Option<String> to String (nullable to not null) is detected
        let mut prev = SQLiteSnapshot::new();
        prev.add_entity(SqliteEntity::Table(Table::new("users")));
        prev.add_entity(SqliteEntity::Column(Column::new("users", "email", "text"))); // nullable

        let mut cur = SQLiteSnapshot::new();
        cur.add_entity(SqliteEntity::Table(Table::new("users")));
        cur.add_entity(SqliteEntity::Column(
            Column::new("users", "email", "text").not_null(),
        )); // not null

        let diff = diff_snapshots(&prev, &cur);
        assert!(diff.has_changes(), "Should detect nullable change");

        // Should be an Alter diff for the column
        let altered = diff.altered();
        assert_eq!(altered.len(), 1, "Should have one altered entity");
        assert_eq!(altered[0].kind, crate::traits::EntityKind::Column);
        assert_eq!(altered[0].name, "users:email");
    }

    #[test]
    fn test_column_not_null_to_nullable() {
        // Test that changing String to Option<String> (not null to nullable) is detected
        let mut prev = SQLiteSnapshot::new();
        prev.add_entity(SqliteEntity::Table(Table::new("users")));
        prev.add_entity(SqliteEntity::Column(
            Column::new("users", "email", "text").not_null(),
        )); // not null

        let mut cur = SQLiteSnapshot::new();
        cur.add_entity(SqliteEntity::Table(Table::new("users")));
        cur.add_entity(SqliteEntity::Column(Column::new("users", "email", "text"))); // nullable

        let diff = diff_snapshots(&prev, &cur);
        assert!(diff.has_changes(), "Should detect nullable change");

        // Should be an Alter diff for the column
        let altered = diff.altered();
        assert_eq!(altered.len(), 1, "Should have one altered entity");
        assert_eq!(altered[0].kind, crate::traits::EntityKind::Column);
    }

    #[test]
    fn test_column_nullable_change_generates_sql() {
        // Test that changing nullable to not null generates RecreateTable SQL
        let mut prev_ddl = SQLiteDDL::new();
        prev_ddl.tables.push(Table::new("users"));
        prev_ddl
            .columns
            .push(Column::new("users", "id", "integer").not_null());
        prev_ddl.columns.push(Column::new("users", "email", "text")); // nullable

        let mut cur_ddl = SQLiteDDL::new();
        cur_ddl.tables.push(Table::new("users"));
        cur_ddl
            .columns
            .push(Column::new("users", "id", "integer").not_null());
        cur_ddl
            .columns
            .push(Column::new("users", "email", "text").not_null()); // not null

        let migration = compute_migration(&prev_ddl, &cur_ddl);

        // Should have generated SQL statements
        assert!(
            !migration.sql_statements.is_empty(),
            "Should generate SQL statements"
        );

        // Should have a RecreateTable statement
        let has_recreate = migration
            .statements
            .iter()
            .any(|s| matches!(s, JsonStatement::RecreateTable(_)));
        assert!(
            has_recreate,
            "Should have RecreateTable statement for column alteration"
        );

        // SQL should contain table recreation pattern
        let sql = migration.sql_statements.join("\n");
        assert!(
            sql.contains("PRAGMA foreign_keys=OFF"),
            "Should disable foreign keys"
        );
        assert!(sql.contains("__new_users"), "Should create temp table");
        assert!(sql.contains("DROP TABLE"), "Should drop old table");
        assert!(sql.contains("RENAME TO"), "Should rename temp table");
        assert!(
            sql.contains("NOT NULL"),
            "New table should have NOT NULL column"
        );
    }

    #[test]
    fn test_column_type_change_generates_recreate() {
        // Test that changing column type generates RecreateTable
        let mut prev_ddl = SQLiteDDL::new();
        prev_ddl.tables.push(Table::new("users"));
        prev_ddl.columns.push(Column::new("users", "age", "text")); // text

        let mut cur_ddl = SQLiteDDL::new();
        cur_ddl.tables.push(Table::new("users"));
        cur_ddl.columns.push(Column::new("users", "age", "integer")); // integer

        let migration = compute_migration(&prev_ddl, &cur_ddl);

        // Should have a RecreateTable statement
        let has_recreate = migration
            .statements
            .iter()
            .any(|s| matches!(s, JsonStatement::RecreateTable(_)));
        assert!(
            has_recreate,
            "Should have RecreateTable statement for type change"
        );
    }
}

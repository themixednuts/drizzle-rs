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
    TableFull, from_json,
};
use crate::traits::EntityKind;
use std::collections::{HashMap, HashSet};

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

/// Group diffs by table name
#[allow(dead_code)]
fn group_diffs_by_table<'a>(diffs: &[&'a EntityDiff]) -> HashMap<String, Vec<&'a EntityDiff>> {
    let mut grouped: HashMap<String, Vec<&'a EntityDiff>> = HashMap::new();

    for diff in diffs {
        if let Some(table) = &diff.table {
            grouped.entry(table.clone()).or_default().push(*diff);
        } else if diff.kind == EntityKind::Column
            && let Some(table) = diff.name.split(':').next()
        {
            // Extract table from name for columns (format: "table:column")
            grouped.entry(table.to_string()).or_default().push(*diff);
        }
    }

    grouped
}

/// Build a TableFull from DDL for a given table name
pub fn table_from_ddl(table_name: &str, ddl: &SQLiteDDL) -> TableFull {
    let entities = ddl.table_entities(table_name);

    TableFull {
        name: table_name.to_string(),
        columns: entities.columns.into_iter().cloned().collect(),
        pk: entities.pk.cloned(),
        fks: entities.fks.into_iter().cloned().collect(),
        uniques: entities.uniques.into_iter().cloned().collect(),
        checks: entities.checks.into_iter().cloned().collect(),
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

    // 1. Create tables
    for table_diff in schema_diff.created_tables() {
        if let Some(SqliteEntity::Table(table)) = &table_diff.right {
            let table_full = table_from_ddl(&table.name, cur);
            statements.push(JsonStatement::CreateTable(CreateTableStatement {
                table: table_full,
            }));
        }
    }

    // 2. Add columns (for existing tables only)
    for col_diff in schema_diff.by_kind(EntityKind::Column) {
        if col_diff.diff_type == DiffType::Create
            && let Some(SqliteEntity::Column(col)) = &col_diff.right
            // Skip columns for newly created tables
            && !created_table_names.contains(col.table.as_ref())
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

    // 3. Drop indexes
    for idx_diff in schema_diff.by_kind(EntityKind::Index) {
        if idx_diff.diff_type == DiffType::Drop
            && let Some(SqliteEntity::Index(idx)) = &idx_diff.left
        {
            statements.push(JsonStatement::DropIndex(DropIndexStatement {
                index: idx.clone(),
            }));
        }
    }

    // 4. Create indexes (including for newly created tables)
    for idx_diff in schema_diff.by_kind(EntityKind::Index) {
        if idx_diff.diff_type == DiffType::Create
            && let Some(SqliteEntity::Index(idx)) = &idx_diff.right
        {
            statements.push(JsonStatement::CreateIndex(CreateIndexStatement {
                index: idx.clone(),
            }));
        }
    }

    // 5. Alter indexes (drop old, create new)
    for idx_diff in schema_diff.by_kind(EntityKind::Index) {
        if idx_diff.diff_type == DiffType::Alter {
            if let Some(SqliteEntity::Index(old_idx)) = &idx_diff.left {
                statements.push(JsonStatement::DropIndex(DropIndexStatement {
                    index: old_idx.clone(),
                }));
            }
            if let Some(SqliteEntity::Index(new_idx)) = &idx_diff.right {
                statements.push(JsonStatement::CreateIndex(CreateIndexStatement {
                    index: new_idx.clone(),
                }));
            }
        }
    }

    // 6. Drop columns (for non-dropped tables)
    for col_diff in schema_diff.by_kind(EntityKind::Column) {
        if col_diff.diff_type == DiffType::Drop
            && let Some(SqliteEntity::Column(col)) = &col_diff.left
            // Skip columns for dropped tables
            && !dropped_table_names.contains(col.table.as_ref())
        {
            statements.push(JsonStatement::DropColumn(DropColumnStatement {
                column: col.clone(),
            }));
        }
    }

    // 7. Drop views
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

    // 8. Create views
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

    // 9. Alter views (drop and recreate)
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

    // 10. Drop tables
    for table_diff in schema_diff.dropped_tables() {
        statements.push(JsonStatement::DropTable(DropTableStatement {
            table_name: table_diff.name.clone(),
        }));
    }

    // Check for column alterations that require table recreation
    // (SQLite doesn't support many ALTER TABLE operations)
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
}

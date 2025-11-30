//! Schema diff types and logic for SQLite

use super::{
    CheckConstraint, Column, CompositePK, ForeignKey, Index, SQLiteSnapshot, Table,
    UniqueConstraint,
};
use std::collections::HashMap;

/// Complete schema diff between two snapshots
#[derive(Debug, Clone, Default)]
pub struct SchemaDiff {
    /// Table-level changes
    pub tables: TablesDiff,
    /// View-level changes (not yet implemented)
    pub views: ViewsDiff,
}

impl SchemaDiff {
    /// Check if there are any changes
    pub fn has_changes(&self) -> bool {
        self.tables.has_changes() || self.views.has_changes()
    }

    /// Check if this diff is empty (no changes)
    pub fn is_empty(&self) -> bool {
        !self.has_changes()
    }
}

/// Table-level diff
#[derive(Debug, Clone, Default)]
pub struct TablesDiff {
    /// Tables that need to be created
    pub created: Vec<Table>,
    /// Table names that need to be dropped
    pub deleted: Vec<String>,
    /// Tables that have been altered
    pub altered: Vec<AlteredTable>,
}

impl TablesDiff {
    /// Check if there are any table changes
    pub fn has_changes(&self) -> bool {
        !self.created.is_empty() || !self.deleted.is_empty() || !self.altered.is_empty()
    }
}

/// An altered table with detailed changes
#[derive(Debug, Clone)]
pub struct AlteredTable {
    /// Table name
    pub name: String,
    /// Column changes
    pub columns: ColumnsDiff,
    /// Index changes
    pub indexes: IndexesDiff,
    /// Foreign key changes
    pub foreign_keys: ForeignKeysDiff,
    /// Unique constraint changes
    pub unique_constraints: UniqueConstraintsDiff,
    /// Check constraint changes
    pub check_constraints: CheckConstraintsDiff,
    /// Composite primary key changes
    pub composite_pks: CompositePKsDiff,
}

impl AlteredTable {
    /// Check if there are any changes to this table
    pub fn has_changes(&self) -> bool {
        self.columns.has_changes()
            || self.indexes.has_changes()
            || self.foreign_keys.has_changes()
            || self.unique_constraints.has_changes()
            || self.check_constraints.has_changes()
            || self.composite_pks.has_changes()
    }
}

/// Column-level diff
#[derive(Debug, Clone, Default)]
pub struct ColumnsDiff {
    /// Columns to add
    pub added: Vec<Column>,
    /// Column names to drop
    pub deleted: Vec<String>,
    /// Columns that were altered (old, new)
    pub altered: Vec<AlteredColumn>,
}

impl ColumnsDiff {
    /// Check if there are any column changes
    pub fn has_changes(&self) -> bool {
        !self.added.is_empty() || !self.deleted.is_empty() || !self.altered.is_empty()
    }
}

/// An altered column with before/after state
#[derive(Debug, Clone)]
pub struct AlteredColumn {
    /// Column name
    pub name: String,
    /// Old column definition
    pub old: Column,
    /// New column definition
    pub new: Column,
}

impl AlteredColumn {
    /// Check if the type changed
    pub fn type_changed(&self) -> bool {
        self.old.sql_type != self.new.sql_type
    }

    /// Check if nullability changed
    pub fn nullability_changed(&self) -> bool {
        self.old.not_null != self.new.not_null
    }

    /// Check if primary key status changed
    pub fn primary_key_changed(&self) -> bool {
        self.old.primary_key != self.new.primary_key
    }

    /// Check if autoincrement changed
    pub fn autoincrement_changed(&self) -> bool {
        self.old.autoincrement != self.new.autoincrement
    }

    /// Check if default value changed
    pub fn default_changed(&self) -> bool {
        self.old.default != self.new.default
    }
}

/// Index-level diff
#[derive(Debug, Clone, Default)]
pub struct IndexesDiff {
    /// Indexes to create
    pub added: Vec<Index>,
    /// Index names to drop
    pub deleted: Vec<String>,
    /// Indexes that were altered (old, new)
    pub altered: Vec<(Index, Index)>,
}

impl IndexesDiff {
    /// Check if there are any index changes
    pub fn has_changes(&self) -> bool {
        !self.added.is_empty() || !self.deleted.is_empty() || !self.altered.is_empty()
    }
}

/// Foreign key diff
#[derive(Debug, Clone, Default)]
pub struct ForeignKeysDiff {
    /// Foreign keys to add
    pub added: Vec<ForeignKey>,
    /// Foreign key names to drop
    pub deleted: Vec<String>,
    /// Foreign keys that were altered (old, new)
    pub altered: Vec<(ForeignKey, ForeignKey)>,
}

impl ForeignKeysDiff {
    /// Check if there are any foreign key changes
    pub fn has_changes(&self) -> bool {
        !self.added.is_empty() || !self.deleted.is_empty() || !self.altered.is_empty()
    }
}

/// Unique constraint diff
#[derive(Debug, Clone, Default)]
pub struct UniqueConstraintsDiff {
    pub added: Vec<UniqueConstraint>,
    pub deleted: Vec<String>,
}

impl UniqueConstraintsDiff {
    pub fn has_changes(&self) -> bool {
        !self.added.is_empty() || !self.deleted.is_empty()
    }
}

/// Check constraint diff
#[derive(Debug, Clone, Default)]
pub struct CheckConstraintsDiff {
    pub added: Vec<CheckConstraint>,
    pub deleted: Vec<String>,
}

impl CheckConstraintsDiff {
    pub fn has_changes(&self) -> bool {
        !self.added.is_empty() || !self.deleted.is_empty()
    }
}

/// Composite primary key diff
#[derive(Debug, Clone, Default)]
pub struct CompositePKsDiff {
    pub added: Vec<CompositePK>,
    pub deleted: Vec<String>,
    pub altered: Vec<(CompositePK, CompositePK)>,
}

impl CompositePKsDiff {
    pub fn has_changes(&self) -> bool {
        !self.added.is_empty() || !self.deleted.is_empty() || !self.altered.is_empty()
    }
}

/// View-level diff (placeholder for future implementation)
#[derive(Debug, Clone, Default)]
pub struct ViewsDiff {
    pub created: Vec<String>,
    pub deleted: Vec<String>,
}

impl ViewsDiff {
    pub fn has_changes(&self) -> bool {
        !self.created.is_empty() || !self.deleted.is_empty()
    }
}

// =============================================================================
// Diff Functions
// =============================================================================

/// Compare two SQLite snapshots and return the diff
pub fn diff_snapshots(prev: &SQLiteSnapshot, cur: &SQLiteSnapshot) -> SchemaDiff {
    SchemaDiff {
        tables: diff_tables(&prev.tables, &cur.tables),
        views: ViewsDiff::default(), // TODO: Implement view diffing
    }
}

/// Compare two sets of tables
fn diff_tables(prev: &HashMap<String, Table>, cur: &HashMap<String, Table>) -> TablesDiff {
    let mut diff = TablesDiff::default();

    // Find created tables (in cur but not in prev)
    for (name, table) in cur {
        if !prev.contains_key(name) {
            diff.created.push(table.clone());
        }
    }

    // Find deleted tables (in prev but not in cur)
    for name in prev.keys() {
        if !cur.contains_key(name) {
            diff.deleted.push(name.clone());
        }
    }

    // Find altered tables (in both, but different)
    for (name, cur_table) in cur {
        if let Some(prev_table) = prev.get(name) {
            let altered = diff_table(prev_table, cur_table);
            if altered.has_changes() {
                diff.altered.push(altered);
            }
        }
    }

    diff
}

/// Compare two tables and return the diff
fn diff_table(prev: &Table, cur: &Table) -> AlteredTable {
    AlteredTable {
        name: cur.name.clone(),
        columns: diff_columns(&prev.columns, &cur.columns),
        indexes: diff_indexes(&prev.indexes, &cur.indexes),
        foreign_keys: diff_foreign_keys(&prev.foreign_keys, &cur.foreign_keys),
        unique_constraints: diff_unique_constraints(
            &prev.unique_constraints,
            &cur.unique_constraints,
        ),
        check_constraints: diff_check_constraints(&prev.check_constraints, &cur.check_constraints),
        composite_pks: diff_composite_pks(
            &prev.composite_primary_keys,
            &cur.composite_primary_keys,
        ),
    }
}

/// Compare two sets of columns
fn diff_columns(prev: &HashMap<String, Column>, cur: &HashMap<String, Column>) -> ColumnsDiff {
    let mut diff = ColumnsDiff::default();

    // Find added columns
    for (name, column) in cur {
        if !prev.contains_key(name) {
            diff.added.push(column.clone());
        }
    }

    // Find deleted columns
    for name in prev.keys() {
        if !cur.contains_key(name) {
            diff.deleted.push(name.clone());
        }
    }

    // Find altered columns
    for (name, cur_col) in cur {
        if let Some(prev_col) = prev.get(name)
            && prev_col != cur_col
        {
            diff.altered.push(AlteredColumn {
                name: name.clone(),
                old: prev_col.clone(),
                new: cur_col.clone(),
            });
        }
    }

    diff
}

/// Compare two sets of indexes
fn diff_indexes(prev: &HashMap<String, Index>, cur: &HashMap<String, Index>) -> IndexesDiff {
    let mut diff = IndexesDiff::default();

    for (name, index) in cur {
        if !prev.contains_key(name) {
            diff.added.push(index.clone());
        }
    }

    for name in prev.keys() {
        if !cur.contains_key(name) {
            diff.deleted.push(name.clone());
        }
    }

    for (name, cur_idx) in cur {
        if let Some(prev_idx) = prev.get(name)
            && prev_idx != cur_idx
        {
            diff.altered.push((prev_idx.clone(), cur_idx.clone()));
        }
    }

    diff
}

/// Compare two sets of foreign keys
fn diff_foreign_keys(
    prev: &HashMap<String, ForeignKey>,
    cur: &HashMap<String, ForeignKey>,
) -> ForeignKeysDiff {
    let mut diff = ForeignKeysDiff::default();

    for (name, fk) in cur {
        if !prev.contains_key(name) {
            diff.added.push(fk.clone());
        }
    }

    for name in prev.keys() {
        if !cur.contains_key(name) {
            diff.deleted.push(name.clone());
        }
    }

    for (name, cur_fk) in cur {
        if let Some(prev_fk) = prev.get(name)
            && prev_fk != cur_fk
        {
            diff.altered.push((prev_fk.clone(), cur_fk.clone()));
        }
    }

    diff
}

/// Compare unique constraints
fn diff_unique_constraints(
    prev: &HashMap<String, UniqueConstraint>,
    cur: &HashMap<String, UniqueConstraint>,
) -> UniqueConstraintsDiff {
    let mut diff = UniqueConstraintsDiff::default();

    for (name, uc) in cur {
        if !prev.contains_key(name) {
            diff.added.push(uc.clone());
        }
    }

    for name in prev.keys() {
        if !cur.contains_key(name) {
            diff.deleted.push(name.clone());
        }
    }

    diff
}

/// Compare check constraints
fn diff_check_constraints(
    prev: &HashMap<String, CheckConstraint>,
    cur: &HashMap<String, CheckConstraint>,
) -> CheckConstraintsDiff {
    let mut diff = CheckConstraintsDiff::default();

    for (name, cc) in cur {
        if !prev.contains_key(name) {
            diff.added.push(cc.clone());
        }
    }

    for name in prev.keys() {
        if !cur.contains_key(name) {
            diff.deleted.push(name.clone());
        }
    }

    diff
}

/// Compare composite primary keys
fn diff_composite_pks(
    prev: &HashMap<String, CompositePK>,
    cur: &HashMap<String, CompositePK>,
) -> CompositePKsDiff {
    let mut diff = CompositePKsDiff::default();

    for (name, pk) in cur {
        if !prev.contains_key(name) {
            diff.added.push(pk.clone());
        }
    }

    for name in prev.keys() {
        if !cur.contains_key(name) {
            diff.deleted.push(name.clone());
        }
    }

    for (name, cur_pk) in cur {
        if let Some(prev_pk) = prev.get(name)
            && prev_pk != cur_pk
        {
            diff.altered.push((prev_pk.clone(), cur_pk.clone()));
        }
    }

    diff
}

#[cfg(test)]
mod tests {
    use super::*;

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

        let mut table = Table::new("users");
        table.add_column(Column::new("id", "integer").primary_key());
        cur.add_table(table);

        let diff = diff_snapshots(&prev, &cur);
        assert!(diff.has_changes());
        assert_eq!(diff.tables.created.len(), 1);
        assert_eq!(diff.tables.created[0].name, "users");
    }

    #[test]
    fn test_table_deletion() {
        let mut prev = SQLiteSnapshot::new();
        let cur = SQLiteSnapshot::new();

        prev.add_table(Table::new("users"));

        let diff = diff_snapshots(&prev, &cur);
        assert!(diff.has_changes());
        assert_eq!(diff.tables.deleted.len(), 1);
        assert_eq!(diff.tables.deleted[0], "users");
    }

    #[test]
    fn test_column_addition() {
        let mut prev = SQLiteSnapshot::new();
        let mut cur = SQLiteSnapshot::new();

        let mut prev_table = Table::new("users");
        prev_table.add_column(Column::new("id", "integer"));
        prev.add_table(prev_table);

        let mut cur_table = Table::new("users");
        cur_table.add_column(Column::new("id", "integer"));
        cur_table.add_column(Column::new("name", "text"));
        cur.add_table(cur_table);

        let diff = diff_snapshots(&prev, &cur);
        assert!(diff.has_changes());
        assert_eq!(diff.tables.altered.len(), 1);
        assert_eq!(diff.tables.altered[0].columns.added.len(), 1);
        assert_eq!(diff.tables.altered[0].columns.added[0].name, "name");
    }

    #[test]
    fn test_column_modification() {
        let mut prev = SQLiteSnapshot::new();
        let mut cur = SQLiteSnapshot::new();

        let mut prev_table = Table::new("users");
        prev_table.add_column(Column::new("name", "text"));
        prev.add_table(prev_table);

        let mut cur_table = Table::new("users");
        cur_table.add_column(Column::new("name", "text").not_null());
        cur.add_table(cur_table);

        let diff = diff_snapshots(&prev, &cur);
        assert!(diff.has_changes());
        assert_eq!(diff.tables.altered[0].columns.altered.len(), 1);
        assert!(diff.tables.altered[0].columns.altered[0].nullability_changed());
    }
}

//! Schema diff types and logic for `SQLite` v7 DDL format
//!
//! This module provides diffing between DDL collections and
//! generates migration statements from schema changes.

use super::SQLiteSnapshot;
use super::collection::{DiffType, EntityDiff, SQLiteDDL, diff_ddl};
use super::ddl::SqliteEntity;
use super::statements::{
    AddColumnStatement, CreateIndexStatement, CreateTableStatement, CreateViewStatement,
    DropColumnStatement, DropIndexStatement, DropTableStatement, DropViewStatement, JsonStatement,
    RecreateTableStatement, RenameColumnStatement, RenameTableStatement, TableFull, from_json,
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
    #[must_use]
    pub const fn has_changes(&self) -> bool {
        !self.diffs.is_empty()
    }

    /// Check if this diff is empty (no changes)
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.diffs.is_empty()
    }

    /// Get created entities
    #[must_use]
    pub fn created(&self) -> Vec<&EntityDiff> {
        self.diffs
            .iter()
            .filter(|d| d.diff_type == DiffType::Create)
            .collect()
    }

    /// Get dropped entities
    #[must_use]
    pub fn dropped(&self) -> Vec<&EntityDiff> {
        self.diffs
            .iter()
            .filter(|d| d.diff_type == DiffType::Drop)
            .collect()
    }

    /// Get altered entities
    #[must_use]
    pub fn altered(&self) -> Vec<&EntityDiff> {
        self.diffs
            .iter()
            .filter(|d| d.diff_type == DiffType::Alter)
            .collect()
    }

    /// Get diffs filtered by entity kind
    #[must_use]
    pub fn by_kind(&self, kind: EntityKind) -> Vec<&EntityDiff> {
        self.diffs.iter().filter(|d| d.kind == kind).collect()
    }

    /// Get created tables
    #[must_use]
    pub fn created_tables(&self) -> Vec<&EntityDiff> {
        self.diffs
            .iter()
            .filter(|d| d.diff_type == DiffType::Create && d.kind == EntityKind::Table)
            .collect()
    }

    /// Get dropped tables
    #[must_use]
    pub fn dropped_tables(&self) -> Vec<&EntityDiff> {
        self.diffs
            .iter()
            .filter(|d| d.diff_type == DiffType::Drop && d.kind == EntityKind::Table)
            .collect()
    }
}

/// Compare two `SQLite` snapshots and return the diff
#[must_use]
pub fn diff_snapshots(prev: &SQLiteSnapshot, cur: &SQLiteSnapshot) -> SchemaDiff {
    let prev_ddl = SQLiteDDL::from_entities(prev.ddl.clone());
    let cur_ddl = SQLiteDDL::from_entities(cur.ddl.clone());

    SchemaDiff {
        diffs: diff_ddl(&prev_ddl, &cur_ddl),
    }
}

/// Compare two DDL collections directly
#[must_use]
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

/// Build a `TableFull` from DDL for a given table name
#[must_use]
pub fn table_from_ddl(table_name: &str, ddl: &SQLiteDDL) -> TableFull {
    let entities = ddl.table_entities(table_name);

    // Get table-level options (strict, without_rowid)
    let (strict, without_rowid) = ddl
        .tables
        .one(table_name)
        .map_or((false, false), |t| (t.strict, t.without_rowid));

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

fn entity_table_name(entity: &SqliteEntity) -> Option<String> {
    match entity {
        SqliteEntity::Column(c) => Some(c.table.to_string()),
        SqliteEntity::ForeignKey(fk) => Some(fk.table.to_string()),
        SqliteEntity::PrimaryKey(pk) => Some(pk.table.to_string()),
        SqliteEntity::UniqueConstraint(uc) => Some(uc.table.to_string()),
        SqliteEntity::CheckConstraint(cc) => Some(cc.table.to_string()),
        _ => None,
    }
}

fn collect_tables_to_recreate(
    schema_diff: &SchemaDiff,
    created: &HashSet<String>,
    dropped: &HashSet<String>,
) -> HashSet<String> {
    let mut out: HashSet<String> = HashSet::new();

    // Column alterations trigger recreation (SQLite has no ALTER COLUMN).
    for col_diff in schema_diff.by_kind(EntityKind::Column) {
        if col_diff.diff_type == DiffType::Alter
            && let Some(SqliteEntity::Column(col)) = &col_diff.right
            && !created.contains(col.table.as_ref())
            && !dropped.contains(col.table.as_ref())
        {
            out.insert(col.table.to_string());
        }
    }

    // New STORED generated columns - SQLite doesn't allow ALTER TABLE ADD COLUMN for STORED
    // See: https://www.sqlite.org/gencol.html
    for col_diff in schema_diff.by_kind(EntityKind::Column) {
        if col_diff.diff_type == DiffType::Create
            && let Some(SqliteEntity::Column(col)) = &col_diff.right
            && col
                .generated
                .as_ref()
                .is_some_and(|g| g.gen_type == super::ddl::GeneratedType::Stored)
            && !created.contains(col.table.as_ref())
            && !dropped.contains(col.table.as_ref())
        {
            out.insert(col.table.to_string());
        }
    }

    // FK, PK, unique, check constraint changes all require recreation.
    for kind in [
        EntityKind::ForeignKey,
        EntityKind::PrimaryKey,
        EntityKind::UniqueConstraint,
        EntityKind::CheckConstraint,
    ] {
        for diff in schema_diff.by_kind(kind) {
            if !matches!(
                diff.diff_type,
                DiffType::Create | DiffType::Drop | DiffType::Alter
            ) {
                continue;
            }
            let table = diff
                .right
                .as_ref()
                .and_then(entity_table_name)
                .or_else(|| diff.left.as_ref().and_then(entity_table_name));
            if let Some(table) = table
                && !created.contains(&table)
                && !dropped.contains(&table)
            {
                out.insert(table);
            }
        }
    }

    out
}

/// Compute a full migration diff between two DDL states
///
/// This is a simplified version of the TypeScript ddlDiff function.
/// For a fully interactive migration with rename detection, you would
/// need to provide resolver callbacks.
#[must_use]
pub fn compute_migration(prev: &SQLiteDDL, cur: &SQLiteDDL) -> MigrationDiff {
    // Heuristic rename detection (non-interactive):
    // - detect exact table renames (same schema, identical entities)
    // - detect exact column renames (same table, identical column properties)
    let mut prev_normalized = prev.clone();
    let mut rename_statements: Vec<JsonStatement> = Vec::new();
    let mut table_renames: Vec<TableRename> = Vec::new();
    let mut column_renames: Vec<ColumnRename> = Vec::new();

    detect_and_apply_sqlite_renames(
        &mut prev_normalized,
        cur,
        &mut rename_statements,
        &mut table_renames,
        &mut column_renames,
    );

    let schema_diff = diff_collections(&prev_normalized, cur);
    let mut statements = Vec::new();
    let mut warnings = Vec::new();
    let renames = prepare_migration_renames(&table_renames, &column_renames);

    // Emit rename statements first so subsequent diffs apply to the renamed schema.
    statements.extend(rename_statements);

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
    let tables_to_recreate =
        collect_tables_to_recreate(&schema_diff, &created_table_names, &dropped_table_names);

    append_table_create_recreate_stmts(
        &mut statements,
        &schema_diff,
        prev,
        cur,
        &tables_to_recreate,
    );
    append_add_column_stmts(
        &mut statements,
        &schema_diff,
        cur,
        &created_table_names,
        &tables_to_recreate,
    );
    append_index_stmts(&mut statements, &schema_diff, cur, &tables_to_recreate);
    append_drop_column_and_view_stmts(
        &mut statements,
        &schema_diff,
        &dropped_table_names,
        &tables_to_recreate,
    );
    append_drop_table_stmts(&mut statements, &schema_diff);
    collect_stored_generated_warnings(&mut warnings, &schema_diff);

    // Convert to SQL
    let result = from_json(statements.clone());

    MigrationDiff {
        statements,
        sql_statements: result.sql_statements,
        renames,
        warnings,
    }
}

fn append_table_create_recreate_stmts(
    statements: &mut Vec<JsonStatement>,
    schema_diff: &SchemaDiff,
    prev: &SQLiteDDL,
    cur: &SQLiteDDL,
    tables_to_recreate: &HashSet<String>,
) {
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
    for table_name in tables_to_recreate {
        let from_table = table_from_ddl(table_name, prev);
        let to_table = table_from_ddl(table_name, cur);
        statements.push(JsonStatement::RecreateTable(RecreateTableStatement {
            from: from_table,
            to: to_table,
        }));
    }
}

fn append_add_column_stmts(
    statements: &mut Vec<JsonStatement>,
    schema_diff: &SchemaDiff,
    cur: &SQLiteDDL,
    created_table_names: &HashSet<String>,
    tables_to_recreate: &HashSet<String>,
) {
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
}

fn append_index_stmts(
    statements: &mut Vec<JsonStatement>,
    schema_diff: &SchemaDiff,
    cur: &SQLiteDDL,
    tables_to_recreate: &HashSet<String>,
) {
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

    // 5b. Recreate indexes for tables that were recreated
    // When a table is recreated, all its indexes are dropped, so we need to recreate them
    for table_name in tables_to_recreate {
        for idx in cur.indexes.for_table(table_name) {
            statements.push(JsonStatement::CreateIndex(CreateIndexStatement {
                index: idx.clone(),
            }));
        }
    }

    // 6. Alter indexes (drop old, create new, skip tables being recreated)
    for idx_diff in schema_diff.by_kind(EntityKind::Index) {
        if idx_diff.diff_type == DiffType::Alter {
            if let Some(SqliteEntity::Index(old_idx)) = &idx_diff.left
                && !tables_to_recreate.contains(old_idx.table.as_ref())
            {
                statements.push(JsonStatement::DropIndex(DropIndexStatement {
                    index: old_idx.clone(),
                }));
            }
            if let Some(SqliteEntity::Index(new_idx)) = &idx_diff.right
                && !tables_to_recreate.contains(new_idx.table.as_ref())
            {
                statements.push(JsonStatement::CreateIndex(CreateIndexStatement {
                    index: new_idx.clone(),
                }));
            }
        }
    }
}

fn append_drop_column_and_view_stmts(
    statements: &mut Vec<JsonStatement>,
    schema_diff: &SchemaDiff,
    dropped_table_names: &HashSet<String>,
    tables_to_recreate: &HashSet<String>,
) {
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
}

fn append_drop_table_stmts(statements: &mut Vec<JsonStatement>, schema_diff: &SchemaDiff) {
    // 11. Drop tables
    for table_diff in schema_diff.dropped_tables() {
        statements.push(JsonStatement::DropTable(DropTableStatement {
            table_name: table_diff.name.clone(),
        }));
    }
}

fn collect_stored_generated_warnings(warnings: &mut Vec<String>, schema_diff: &SchemaDiff) {
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
}

fn sqlite_table_signature(table_name: &str, ddl: &SQLiteDDL) -> Vec<SqliteEntity> {
    // A stable-ish signature for rename matching: table + its entities (columns/constraints/indexes)
    // using the existing entity shapes, sorted by their EntityKey ordering via serialization key.
    // We intentionally exclude views (not tied to a table by name reliably).
    let mut entities = Vec::new();

    if let Some(t) = ddl.tables.one(table_name) {
        entities.push(SqliteEntity::Table(t.clone()));
    }
    for c in ddl.columns.for_table(table_name) {
        entities.push(SqliteEntity::Column(c.clone()));
    }
    if let Some(pk) = ddl.pks.for_table(table_name) {
        entities.push(SqliteEntity::PrimaryKey(pk.clone()));
    }
    for u in ddl.uniques.for_table(table_name) {
        entities.push(SqliteEntity::UniqueConstraint(u.clone()));
    }
    for fk in ddl.fks.for_table(table_name) {
        entities.push(SqliteEntity::ForeignKey(fk.clone()));
    }
    for idx in ddl.indexes.for_table(table_name) {
        entities.push(SqliteEntity::Index(idx.clone()));
    }
    for chk in ddl.checks.for_table(table_name) {
        entities.push(SqliteEntity::CheckConstraint(chk.clone()));
    }

    // Sort by debug string as a simple stable ordering for comparisons.
    entities.sort_by(|a, b| format!("{a:?}").cmp(&format!("{b:?}")));
    entities
}

fn detect_and_apply_sqlite_renames(
    prev: &mut SQLiteDDL,
    cur: &SQLiteDDL,
    rename_statements: &mut Vec<JsonStatement>,
    table_renames: &mut Vec<TableRename>,
    column_renames: &mut Vec<ColumnRename>,
) {
    // Table renames: exact match of signatures, different table name.
    let prev_tables: Vec<String> = prev
        .tables
        .list()
        .iter()
        .map(|t| t.name.to_string())
        .collect();
    let cur_tables: Vec<String> = cur
        .tables
        .list()
        .iter()
        .map(|t| t.name.to_string())
        .collect();

    let dropped: Vec<String> = prev_tables
        .iter()
        .filter(|t| !cur_tables.contains(t))
        .cloned()
        .collect();
    let created: Vec<String> = cur_tables
        .iter()
        .filter(|t| !prev_tables.contains(t))
        .cloned()
        .collect();

    let mut used_created: HashSet<String> = HashSet::new();
    for from in &dropped {
        let from_sig = sqlite_table_signature(from, prev);
        let mut best: Option<String> = None;
        for to in &created {
            if used_created.contains(to) {
                continue;
            }
            let to_sig = sqlite_table_signature(to, cur);
            if from_sig == to_sig {
                best = Some(to.clone());
                break;
            }
        }
        if let Some(to) = best {
            used_created.insert(to.clone());
            // Record and emit rename
            table_renames.push(TableRename {
                from: from.clone(),
                to: to.clone(),
            });
            rename_statements.push(JsonStatement::RenameTable(RenameTableStatement {
                from: from.clone(),
                to: to.clone(),
            }));
            apply_sqlite_table_rename(prev, from, &to);
        }
    }

    // Column renames (within tables that exist in both): exact property match, different name.
    let common_tables: Vec<String> = prev
        .tables
        .list()
        .iter()
        .map(|t| t.name.to_string())
        .filter(|t| cur.tables.one(t).is_some())
        .collect();

    for table in common_tables {
        let prev_cols: Vec<_> = prev.columns.for_table(&table);
        let cur_cols: Vec<_> = cur.columns.for_table(&table);

        let prev_names: Vec<String> = prev_cols.iter().map(|c| c.name.to_string()).collect();
        let cur_names: Vec<String> = cur_cols.iter().map(|c| c.name.to_string()).collect();

        let dropped_cols: Vec<String> = prev_names
            .iter()
            .filter(|c| !cur_names.contains(c))
            .cloned()
            .collect();
        let created_cols: Vec<String> = cur_names
            .iter()
            .filter(|c| !prev_names.contains(c))
            .cloned()
            .collect();

        if dropped_cols.len() != 1 || created_cols.len() != 1 {
            continue;
        }

        let from = &dropped_cols[0];
        let to = &created_cols[0];

        let prev_col = prev.columns.one(&table, from);
        let cur_col = cur.columns.one(&table, to);
        if let (Some(prev_col), Some(cur_col)) = (prev_col, cur_col) {
            let mut prev_cmp = prev_col.clone();
            prev_cmp.name.clone_from(&cur_col.name);
            if prev_cmp == *cur_col {
                column_renames.push(ColumnRename {
                    table: table.clone(),
                    from: from.clone(),
                    to: to.clone(),
                });
                rename_statements.push(JsonStatement::RenameColumn(RenameColumnStatement {
                    table: table.clone(),
                    from: from.clone(),
                    to: to.clone(),
                }));
                apply_sqlite_column_rename(prev, &table, from, to);
            }
        }
    }
}

fn apply_sqlite_table_rename(ddl: &mut SQLiteDDL, from: &str, to: &str) {
    let to = to.to_string();
    // Tables
    if let Some(t) = ddl
        .tables
        .list_mut()
        .iter_mut()
        .find(|t| t.name.as_ref() == from)
    {
        t.name = to.clone().into();
    }
    // Columns
    for c in ddl
        .columns
        .list_mut()
        .iter_mut()
        .filter(|c| c.table.as_ref() == from)
    {
        c.table = to.clone().into();
    }
    // PKs
    for pk in ddl
        .pks
        .list_mut()
        .iter_mut()
        .filter(|pk| pk.table.as_ref() == from)
    {
        pk.table = to.clone().into();
    }
    // Uniques
    for u in ddl
        .uniques
        .list_mut()
        .iter_mut()
        .filter(|u| u.table.as_ref() == from)
    {
        u.table = to.clone().into();
    }
    // FKs (table side and referenced side)
    for fk in ddl.fks.list_mut().iter_mut() {
        if fk.table.as_ref() == from {
            fk.table = to.clone().into();
        }
        if fk.table_to.as_ref() == from {
            fk.table_to = to.clone().into();
        }
    }
    // Indexes
    for idx in ddl
        .indexes
        .list_mut()
        .iter_mut()
        .filter(|i| i.table.as_ref() == from)
    {
        idx.table = to.clone().into();
    }
    // Checks
    for chk in ddl
        .checks
        .list_mut()
        .iter_mut()
        .filter(|c| c.table.as_ref() == from)
    {
        chk.table = to.clone().into();
    }
}

fn apply_sqlite_column_rename(ddl: &mut SQLiteDDL, table: &str, from: &str, to: &str) {
    let to = to.to_string();
    // Columns
    if let Some(c) = ddl
        .columns
        .list_mut()
        .iter_mut()
        .find(|c| c.table.as_ref() == table && c.name.as_ref() == from)
    {
        c.name = to.clone().into();
    }
    // PK columns
    for pk in ddl
        .pks
        .list_mut()
        .iter_mut()
        .filter(|pk| pk.table.as_ref() == table)
    {
        for col in pk.columns.to_mut().iter_mut() {
            if col.as_ref() == from {
                *col = to.clone().into();
            }
        }
    }
    // Unique columns
    for u in ddl
        .uniques
        .list_mut()
        .iter_mut()
        .filter(|u| u.table.as_ref() == table)
    {
        for col in u.columns.to_mut().iter_mut() {
            if col.as_ref() == from {
                *col = to.clone().into();
            }
        }
    }
    // FK columns
    for fk in ddl.fks.list_mut().iter_mut() {
        if fk.table.as_ref() == table {
            for col in fk.columns.to_mut().iter_mut() {
                if col.as_ref() == from {
                    *col = to.clone().into();
                }
            }
        }
        if fk.table_to.as_ref() == table {
            for col in fk.columns_to.to_mut().iter_mut() {
                if col.as_ref() == from {
                    *col = to.clone().into();
                }
            }
        }
    }
    // Index columns (only non-expression)
    for idx in ddl
        .indexes
        .list_mut()
        .iter_mut()
        .filter(|i| i.table.as_ref() == table)
    {
        for col in &mut idx.columns {
            if !col.is_expression && col.value.as_ref() == from {
                col.value = to.clone().into();
            }
        }
    }
}

/// Prepare rename tracking strings for snapshot storage
#[must_use]
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

        // Verify individual SQL statements for table recreation pattern
        assert_eq!(migration.sql_statements[0], "PRAGMA foreign_keys=OFF;");
        assert!(
            migration.sql_statements[1].starts_with("CREATE TABLE `__new_users`"),
            "Expected CREATE TABLE `__new_users`, got: {}",
            migration.sql_statements[1]
        );
        assert!(
            migration.sql_statements[1].contains("`email` TEXT NOT NULL"),
            "New table should have NOT NULL on email: {}",
            migration.sql_statements[1]
        );
        assert_eq!(
            migration.sql_statements[2],
            "INSERT INTO `__new_users`(`id`, `email`) SELECT `id`, `email` FROM `users`;"
        );
        assert_eq!(migration.sql_statements[3], "DROP TABLE `users`;");
        assert_eq!(
            migration.sql_statements[4],
            "ALTER TABLE `__new_users` RENAME TO `users`;"
        );
        assert_eq!(migration.sql_statements[5], "PRAGMA foreign_keys=ON;");
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

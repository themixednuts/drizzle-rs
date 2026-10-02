//! Diffs two SQLite DDL collections (v7 format) into migration statements.

use super::SQLiteSnapshot;
use super::collection::{DiffType, EntityDiff, SQLiteDDL, diff_ddl};
use super::ddl::{SqliteEntity, View};
use super::statements::{
    AddColumnStatement, CreateIndexStatement, CreateTableStatement, CreateViewStatement,
    DropColumnStatement, DropIndexStatement, DropTableStatement, DropViewStatement, JsonStatement,
    RecreateTableStatement, RenameColumnStatement, RenameTableStatement, TableFull, from_json,
};
use crate::traits::EntityKind;
use std::collections::{BTreeMap, BTreeSet, HashSet};

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

/// Returns the entity-level differences between two SQLite snapshots.
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
) -> BTreeSet<String> {
    // BTreeSet: iteration order reaches the emitted SQL, so it must be
    // deterministic.
    let mut out: BTreeSet<String> = BTreeSet::new();

    // Table-level option changes (STRICT / WITHOUT ROWID) can only be applied
    // by recreating the table.
    for table_diff in schema_diff.by_kind(EntityKind::Table) {
        if table_diff.diff_type == DiffType::Alter
            && let Some(SqliteEntity::Table(table)) = &table_diff.right
            && !created.contains(table.name.as_ref())
            && !dropped.contains(table.name.as_ref())
        {
            out.insert(table.name.to_string());
        }
    }

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

/// Computes the migration (diff plus SQL) between two SQLite DDL states.
///
/// A non-interactive port of drizzle-kit's `ddlDiff`: renames are detected
/// heuristically, never by prompting. Use rename hints
/// ([`DiffOptions`](crate::DiffOptions)) for renames it cannot infer.
#[must_use]
pub fn compute_migration(prev: &SQLiteDDL, cur: &SQLiteDDL) -> MigrationDiff {
    // Heuristic rename detection (non-interactive):
    // - detect exact table renames (same schema, identical entities)
    // - detect exact column renames (same table, identical column properties)
    let mut prev_normalized = prev.clone();
    let mut rename_statements: Vec<JsonStatement> = Vec::new();
    let mut table_renames: Vec<TableRename> = Vec::new();
    let mut column_renames: Vec<ColumnRename> = Vec::new();
    let mut warnings = Vec::new();

    detect_and_apply_renames(
        &mut prev_normalized,
        cur,
        &mut rename_statements,
        &mut table_renames,
        &mut column_renames,
        &mut warnings,
    );

    let schema_diff = diff_collections(&prev_normalized, cur);
    let mut statements = Vec::new();
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

    let view_plan = plan_views(&schema_diff, &prev_normalized, cur, &tables_to_recreate);
    statements.extend(
        view_plan
            .drops
            .into_iter()
            .map(|view| JsonStatement::DropView(DropViewStatement { view })),
    );

    // The rebuild copies from the table as it is after the renames above,
    // so a renamed table or column is found under its new name.
    append_table_create_recreate_stmts(
        &mut statements,
        &schema_diff,
        &prev_normalized,
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
    append_drop_column_stmts(
        &mut statements,
        &schema_diff,
        &dropped_table_names,
        &tables_to_recreate,
    );
    append_drop_table_stmts(
        &mut statements,
        &schema_diff,
        &prev_normalized,
        &mut warnings,
    );
    statements.extend(
        view_plan
            .creates
            .into_iter()
            .map(|view| JsonStatement::CreateView(CreateViewStatement { view })),
    );
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
    tables_to_recreate: &BTreeSet<String>,
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
            data: None,
        }));
    }
}

fn append_add_column_stmts(
    statements: &mut Vec<JsonStatement>,
    schema_diff: &SchemaDiff,
    cur: &SQLiteDDL,
    created_table_names: &HashSet<String>,
    tables_to_recreate: &BTreeSet<String>,
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
    tables_to_recreate: &BTreeSet<String>,
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

fn append_drop_column_stmts(
    statements: &mut Vec<JsonStatement>,
    schema_diff: &SchemaDiff,
    dropped_table_names: &HashSet<String>,
    tables_to_recreate: &BTreeSet<String>,
) {
    // Drop columns (for non-dropped tables, skip tables being recreated)
    for col_diff in schema_diff.by_kind(EntityKind::Column) {
        if col_diff.diff_type == DiffType::Drop
            && let Some(SqliteEntity::Column(col)) = &col_diff.left
            && !dropped_table_names.contains(col.table.as_ref())
            && !tables_to_recreate.contains(col.table.as_ref())
        {
            statements.push(JsonStatement::DropColumn(DropColumnStatement {
                column: col.clone(),
            }));
        }
    }
}

/// Whether `definition` mentions `table` as a whole identifier (quoted or
/// not, any case). A false match only recreates a view unnecessarily.
fn view_mentions_table(definition: &str, table: &str) -> bool {
    let definition = definition.to_ascii_lowercase();
    let table = table.to_ascii_lowercase();
    let is_word = |c: char| c.is_ascii_alphanumeric() || c == '_' || c == '$';
    definition.match_indices(&table).any(|(index, _)| {
        let before = definition[..index].chars().next_back();
        let after = definition[index + table.len()..].chars().next();
        !before.is_some_and(is_word) && !after.is_some_and(is_word)
    })
}

/// Views to drop before the table changes and create after them.
///
/// SQLite checks every view when a table is renamed or a column dropped, so
/// a view over a rebuilt table, or one still selecting a dropped column,
/// makes those statements fail. Such views are dropped first and created
/// again (from the new schema) once the tables are in place. Views declared
/// `existing` are never touched.
struct ViewPlan {
    drops: Vec<View>,
    creates: Vec<View>,
}

fn plan_views(
    schema_diff: &SchemaDiff,
    prev: &SQLiteDDL,
    cur: &SQLiteDDL,
    tables_to_recreate: &BTreeSet<String>,
) -> ViewPlan {
    let mut drops: Vec<View> = Vec::new();
    let mut creates: Vec<View> = Vec::new();
    for view_diff in schema_diff.by_kind(EntityKind::View) {
        let left = match &view_diff.left {
            Some(SqliteEntity::View(view)) if !view.is_existing => Some(view),
            _ => None,
        };
        let right = match &view_diff.right {
            Some(SqliteEntity::View(view)) if !view.is_existing => Some(view),
            _ => None,
        };
        match view_diff.diff_type {
            DiffType::Drop => drops.extend(left.cloned()),
            DiffType::Create => creates.extend(right.cloned()),
            // An `existing` view on either side is managed outside drizzle.
            DiffType::Alter => {
                if let (Some(left), Some(right)) = (left, right) {
                    drops.push(left.clone());
                    creates.push(right.clone());
                }
            }
        }
    }

    // Unchanged views over a table that is rebuilt or loses a column.
    let mut affected: BTreeSet<String> = tables_to_recreate.clone();
    for col_diff in schema_diff.by_kind(EntityKind::Column) {
        if col_diff.diff_type == DiffType::Drop
            && let Some(SqliteEntity::Column(col)) = &col_diff.left
        {
            affected.insert(col.table.to_string());
        }
    }
    for view in prev.views.list() {
        if view.is_existing || drops.iter().any(|dropped| dropped.name == view.name) {
            continue;
        }
        let Some(current) = cur.views.one(&view.name) else {
            continue;
        };
        if current.is_existing {
            continue;
        }
        let definition = view.definition.as_deref().unwrap_or_default();
        if affected
            .iter()
            .any(|table| view_mentions_table(definition, table))
        {
            drops.push(view.clone());
            creates.push(current.clone());
        }
    }
    ViewPlan { drops, creates }
}

/// Drops tables children first: dropping a parent while a child still
/// references its rows fails with `PRAGMA foreign_keys` on.
fn append_drop_table_stmts(
    statements: &mut Vec<JsonStatement>,
    schema_diff: &SchemaDiff,
    prev: &SQLiteDDL,
    warnings: &mut Vec<String>,
) {
    let mut pending: Vec<String> = schema_diff
        .dropped_tables()
        .iter()
        .map(|table_diff| table_diff.name.clone())
        .collect();
    // `parents[t]`: the other dropped tables `t` references.
    let parents: BTreeMap<String, BTreeSet<String>> = pending
        .iter()
        .map(|table| {
            let referenced = prev
                .fks
                .for_table(table)
                .into_iter()
                .map(|fk| fk.table_to.to_string())
                .filter(|parent| parent != table && pending.contains(parent))
                .collect();
            (table.clone(), referenced)
        })
        .collect();

    while !pending.is_empty() {
        // A table no remaining dropped table references can go now.
        let ready: Vec<String> = pending
            .iter()
            .filter(|table| {
                !pending
                    .iter()
                    .any(|other| other != *table && parents[other].contains(*table))
            })
            .cloned()
            .collect();
        let batch = if ready.is_empty() {
            warnings.push(format!(
                "Dropped tables [{}] reference each other; dropping them fails while their rows reference each other and PRAGMA foreign_keys is on",
                pending.join(", ")
            ));
            std::mem::take(&mut pending)
        } else {
            pending.retain(|table| !ready.contains(table));
            ready
        };
        for table_name in batch {
            statements.push(JsonStatement::DropTable(DropTableStatement { table_name }));
        }
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

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct TableColumnFingerprint {
    name: String,
    sql_type: String,
    not_null: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct TableFingerprint {
    columns: Vec<TableColumnFingerprint>,
    pk_columns: Vec<String>,
}

fn table_fingerprint(table_name: &str, ddl: &SQLiteDDL) -> TableFingerprint {
    let mut columns: Vec<_> = ddl
        .columns
        .for_table(table_name)
        .into_iter()
        .map(|c| TableColumnFingerprint {
            name: c.name.to_string(),
            sql_type: c.sql_type.to_string(),
            not_null: c.not_null,
        })
        .collect();
    columns.sort();

    let pk_columns = if let Some(pk) = ddl.pks.for_table(table_name) {
        pk.columns.iter().map(ToString::to_string).collect()
    } else {
        let mut inline_pk_columns: Vec<_> = ddl
            .columns
            .for_table(table_name)
            .into_iter()
            .filter(|c| c.primary_key.unwrap_or(false))
            .map(|c| (c.ordinal_position.unwrap_or(i32::MAX), c.name.to_string()))
            .collect();
        inline_pk_columns.sort();
        inline_pk_columns
            .into_iter()
            .map(|(_, name)| name)
            .collect()
    };

    TableFingerprint {
        columns,
        pk_columns,
    }
}

fn detect_and_apply_renames(
    prev: &mut SQLiteDDL,
    cur: &SQLiteDDL,
    rename_statements: &mut Vec<JsonStatement>,
    table_renames: &mut Vec<TableRename>,
    column_renames: &mut Vec<ColumnRename>,
    warnings: &mut Vec<String>,
) {
    // Table renames: exact match of columns (name/type/nullability) and PK shape.
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

    let mut candidates: BTreeMap<TableFingerprint, (Vec<String>, Vec<String>)> = BTreeMap::new();
    for from in dropped {
        candidates
            .entry(table_fingerprint(&from, prev))
            .or_default()
            .0
            .push(from);
    }
    for to in created {
        candidates
            .entry(table_fingerprint(&to, cur))
            .or_default()
            .1
            .push(to);
    }

    for (_, (mut dropped, mut created)) in candidates {
        if dropped.is_empty() || created.is_empty() {
            continue;
        }

        dropped.sort();
        created.sort();

        if dropped.len() == 1 && created.len() == 1 {
            let from = &dropped[0];
            let to = &created[0];
            table_renames.push(TableRename {
                from: from.clone(),
                to: to.clone(),
            });
            rename_statements.push(JsonStatement::RenameTable(RenameTableStatement {
                from: from.clone(),
                to: to.clone(),
            }));
            apply_table_rename(prev, from, to);
        } else {
            warnings.push(format!(
                "Ambiguous SQLite table rename candidates between dropped tables [{}] and created tables [{}]; no rename was inferred. Use DiffOptions::rename_table(...) with diff_with or diff_schemas_with to provide an explicit rename hint.",
                dropped.join(", "),
                created.join(", ")
            ));
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
                apply_column_rename(prev, &table, from, to);
            }
        }
    }
}

fn apply_table_rename(ddl: &mut SQLiteDDL, from: &str, to: &str) {
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

fn apply_column_rename(ddl: &mut SQLiteDDL, table: &str, from: &str, to: &str) {
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
    use crate::sqlite::ddl::{Column, ForeignKey, Index, IndexColumn, SqliteEntity, Table};
    use std::borrow::Cow;

    fn ddl(entities: Vec<SqliteEntity>) -> SQLiteDDL {
        SQLiteDDL::from_entities(entities)
    }

    fn pk(table: &'static str, name: &'static str) -> Column {
        let mut column = Column::new(table, name, "integer").not_null();
        column.primary_key = Some(true);
        column
    }

    /// A column rename detected alongside a change that rebuilds the table
    /// must copy the renamed column's data into the new table.
    #[test]
    fn rebuild_after_detected_column_rename_copies_the_renamed_column() {
        let prev = ddl(vec![
            SqliteEntity::Table(Table::new("t")),
            SqliteEntity::Column(pk("t", "id")),
            SqliteEntity::Column(Column::new("t", "a", "text")),
            SqliteEntity::Column(Column::new("t", "b", "integer")),
        ]);
        let cur = ddl(vec![
            SqliteEntity::Table(Table::new("t")),
            SqliteEntity::Column(pk("t", "id")),
            SqliteEntity::Column(Column::new("t", "a2", "text")),
            SqliteEntity::Column(Column::new("t", "b", "integer").not_null()),
        ]);
        let sql = compute_migration(&prev, &cur).sql_statements.join("\n");
        assert!(sql.contains("RENAME COLUMN `a` TO `a2`"), "{sql}");
        assert!(
            sql.contains("INSERT INTO `__new_t`(`id`, `a2`, `b`) SELECT `id`, `a2`, `b` FROM `t`"),
            "{sql}"
        );
    }

    /// A table rename detected alongside a change that rebuilds the table
    /// must copy every column, not emit an empty column list.
    #[test]
    fn rebuild_after_detected_table_rename_copies_every_column() {
        let prev = ddl(vec![
            SqliteEntity::Table(Table::new("old")),
            SqliteEntity::Column(pk("old", "id")),
            SqliteEntity::Column(Column::new("old", "v", "text")),
        ]);
        let mut strict = Table::new("new");
        strict.strict = true;
        let cur = ddl(vec![
            SqliteEntity::Table(strict),
            SqliteEntity::Column(pk("new", "id")),
            SqliteEntity::Column(Column::new("new", "v", "text")),
        ]);
        let sql = compute_migration(&prev, &cur).sql_statements.join("\n");
        assert!(sql.contains("RENAME TO `new`"), "{sql}");
        assert!(
            sql.contains("INSERT INTO `__new_new`(`id`, `v`) SELECT `id`, `v` FROM `new`"),
            "{sql}"
        );
    }

    #[test]
    fn dropped_tables_go_children_first() {
        let fk = ForeignKey::new(
            "posts",
            "posts_user_fk",
            vec!["user_id".into()],
            "users",
            vec!["id".into()],
        );
        let prev = ddl(vec![
            SqliteEntity::Table(Table::new("users")),
            SqliteEntity::Column(pk("users", "id")),
            SqliteEntity::Table(Table::new("posts")),
            SqliteEntity::Column(pk("posts", "id")),
            SqliteEntity::Column(Column::new("posts", "user_id", "integer")),
            SqliteEntity::ForeignKey(fk),
        ]);
        let sql = compute_migration(&prev, &ddl(Vec::new())).sql_statements;
        assert_eq!(sql, ["DROP TABLE `posts`;", "DROP TABLE `users`;"]);
    }

    fn view(name: &'static str, definition: &'static str) -> SqliteEntity {
        let mut view = View::new(name);
        view.definition = Some(definition.into());
        SqliteEntity::View(view)
    }

    /// A view over a table that is rebuilt is dropped first and created
    /// again after, because SQLite rejects the rebuild's rename while the
    /// view points at the table.
    #[test]
    fn views_over_rebuilt_tables_are_dropped_first_and_recreated_last() {
        let base = |not_null: bool| {
            let email = Column::new("users", "email", "text");
            let email = if not_null { email.not_null() } else { email };
            vec![
                SqliteEntity::Table(Table::new("users")),
                SqliteEntity::Column(pk("users", "id")),
                SqliteEntity::Column(email),
                view("v_users", "SELECT id, email FROM users"),
                view("v_other", "SELECT 1 AS users_count"),
            ]
        };
        let sql = compute_migration(&ddl(base(false)), &ddl(base(true))).sql_statements;
        let position = |needle: &str| {
            sql.iter()
                .position(|statement| statement.contains(needle))
                .unwrap_or_else(|| panic!("{needle} missing from {sql:#?}"))
        };
        assert!(position("DROP VIEW `v_users`") < position("CREATE TABLE `__new_users`"));
        assert!(position("CREATE VIEW `v_users`") > position("RENAME TO `users`"));
        assert!(
            !sql.iter().any(|statement| statement.contains("v_other")),
            "{sql:#?}"
        );
    }

    /// Altering a view that selects a dropped column drops it before the
    /// column, and an `existing` view is never dropped.
    #[test]
    fn view_alters_drop_before_columns_and_existing_views_are_left_alone() {
        let prev = ddl(vec![
            SqliteEntity::Table(Table::new("users")),
            SqliteEntity::Column(pk("users", "id")),
            SqliteEntity::Column(Column::new("users", "email", "text")),
            view("v", "SELECT id, email FROM users"),
            view("legacy", "SELECT id FROM users"),
        ]);
        let mut existing = View::new("legacy");
        existing.is_existing = true;
        let cur = ddl(vec![
            SqliteEntity::Table(Table::new("users")),
            SqliteEntity::Column(pk("users", "id")),
            view("v", "SELECT id FROM users"),
            SqliteEntity::View(existing),
        ]);
        let sql = compute_migration(&prev, &cur).sql_statements;
        let drop_view = sql
            .iter()
            .position(|s| s.contains("DROP VIEW `v`"))
            .unwrap();
        let drop_column = sql.iter().position(|s| s.contains("DROP COLUMN")).unwrap();
        assert!(drop_view < drop_column, "{sql:#?}");
        assert!(!sql.iter().any(|s| s.contains("legacy")), "{sql:#?}");
    }

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

    fn sqlite_table_with_id(table: &str) -> SQLiteDDL {
        let mut ddl = SQLiteDDL::new();
        ddl.tables.push(Table::new(table.to_string()));
        ddl.columns
            .push(Column::new(table.to_string(), "id", "integer").not_null());
        ddl
    }

    #[test]
    fn pure_table_rename_emits_single_rename_statement() {
        let prev = sqlite_table_with_id("users");
        let cur = sqlite_table_with_id("accounts");

        let migration = compute_migration(&prev, &cur);

        assert_eq!(migration.statements.len(), 1);
        assert!(matches!(
            migration.statements[0],
            JsonStatement::RenameTable(_)
        ));
        assert_eq!(
            migration.sql_statements,
            vec!["ALTER TABLE `users` RENAME TO `accounts`;"]
        );
        assert!(
            !migration
                .statements
                .iter()
                .any(|statement| matches!(statement, JsonStatement::DropTable(_)))
        );
    }

    #[test]
    fn table_rename_rewrites_indexes_and_foreign_keys() {
        let mut prev = sqlite_table_with_id("users");
        prev.tables.push(Table::new("posts"));
        prev.columns
            .push(Column::new("posts", "id", "integer").not_null());
        prev.columns
            .push(Column::new("posts", "user_id", "integer").not_null());
        prev.indexes.push(Index::new(
            "users",
            "idx_users_id",
            vec![IndexColumn::new("id")],
        ));
        prev.fks.push(ForeignKey::new(
            "posts",
            "fk_posts_user",
            vec![Cow::Borrowed("user_id")],
            "users",
            vec![Cow::Borrowed("id")],
        ));

        let mut cur = sqlite_table_with_id("accounts");
        cur.tables.push(Table::new("posts"));
        cur.columns
            .push(Column::new("posts", "id", "integer").not_null());
        cur.columns
            .push(Column::new("posts", "user_id", "integer").not_null());
        cur.indexes.push(Index::new(
            "accounts",
            "idx_users_id",
            vec![IndexColumn::new("id")],
        ));
        cur.fks.push(ForeignKey::new(
            "posts",
            "fk_posts_user",
            vec![Cow::Borrowed("user_id")],
            "accounts",
            vec![Cow::Borrowed("id")],
        ));

        let migration = compute_migration(&prev, &cur);

        assert_eq!(
            migration.sql_statements,
            vec!["ALTER TABLE `users` RENAME TO `accounts`;"]
        );
        assert!(
            !migration.sql_statements.iter().any(|statement| {
                statement.starts_with("DROP")
                    || statement.starts_with("CREATE INDEX")
                    || statement.contains("fk_posts_user")
            }),
            "unexpected dependent churn: {:?}",
            migration.sql_statements
        );
    }

    #[test]
    fn ambiguous_table_rename_does_not_guess_and_warns() {
        let mut prev = sqlite_table_with_id("users");
        let mut admins = sqlite_table_with_id("admins");
        prev.tables.list_mut().append(admins.tables.list_mut());
        prev.columns.list_mut().append(admins.columns.list_mut());
        let cur = sqlite_table_with_id("accounts");

        let migration = compute_migration(&prev, &cur);

        assert!(
            migration.warnings.iter().any(|warning| warning
                .contains("Ambiguous SQLite table rename candidates")
                && warning.contains("rename_table")),
            "expected ambiguous rename warning, got {:?}",
            migration.warnings
        );
        assert!(
            !migration
                .statements
                .iter()
                .any(|statement| matches!(statement, JsonStatement::RenameTable(_)))
        );
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
    fn strict_toggle_generates_table_recreate() {
        let mut prev = SQLiteDDL::new();
        prev.tables.push(Table::new("users"));
        prev.columns
            .push(Column::new("users", "id", "integer").not_null());

        let mut cur = SQLiteDDL::new();
        cur.tables.push(Table::new("users").strict());
        cur.columns
            .push(Column::new("users", "id", "integer").not_null());

        let migration = compute_migration(&prev, &cur);

        let has_recreate = migration
            .statements
            .iter()
            .any(|s| matches!(s, JsonStatement::RecreateTable(_)));
        assert!(
            has_recreate,
            "toggling STRICT must recreate the table, got: {:?}",
            migration.statements
        );
        assert!(
            migration.sql_statements.iter().any(|sql| sql
                .starts_with("CREATE TABLE `__new_users`")
                && sql.ends_with("STRICT;")),
            "recreated table must carry STRICT: {:?}",
            migration.sql_statements
        );
    }

    #[test]
    fn partial_index_predicate_change_recreates_index() {
        let mut prev = sqlite_table_with_id("jobs");
        let mut previous_index =
            Index::new("jobs", "idx_jobs_unclaimed", vec![IndexColumn::new("id")]);
        previous_index.where_clause = Some(Cow::Borrowed("builder IS NULL"));
        prev.indexes.push(previous_index);

        let mut cur = sqlite_table_with_id("jobs");
        let mut current_index =
            Index::new("jobs", "idx_jobs_unclaimed", vec![IndexColumn::new("id")]);
        current_index.where_clause = Some(Cow::Borrowed("builder IS NOT NULL"));
        cur.indexes.push(current_index);

        let migration = compute_migration(&prev, &cur);
        assert_eq!(
            migration.sql_statements,
            vec![
                "DROP INDEX IF EXISTS `idx_jobs_unclaimed`;",
                "CREATE INDEX `idx_jobs_unclaimed` ON `jobs`(`id`) WHERE builder IS NOT NULL;",
            ]
        );
    }

    #[test]
    fn without_rowid_toggle_generates_table_recreate() {
        let mut prev = SQLiteDDL::new();
        prev.tables.push(Table::new("kv"));
        prev.columns
            .push(Column::new("kv", "key", "text").not_null());

        let mut cur = SQLiteDDL::new();
        cur.tables.push(Table::new("kv").without_rowid());
        cur.columns
            .push(Column::new("kv", "key", "text").not_null());

        let migration = compute_migration(&prev, &cur);

        assert!(
            migration
                .statements
                .iter()
                .any(|s| matches!(s, JsonStatement::RecreateTable(_))),
            "toggling WITHOUT ROWID must recreate the table, got: {:?}",
            migration.statements
        );
    }

    #[test]
    fn multi_table_recreation_order_is_deterministic() {
        let make = |not_null: bool| {
            let mut ddl = SQLiteDDL::new();
            for table in ["zeta", "alpha", "midway"] {
                ddl.tables.push(Table::new(table.to_string()));
                let col = Column::new(table.to_string(), "name", "text");
                ddl.columns
                    .push(if not_null { col.not_null() } else { col });
            }
            ddl
        };

        let migration = compute_migration(&make(false), &make(true));
        let recreate_order: Vec<String> = migration
            .statements
            .iter()
            .filter_map(|s| match s {
                JsonStatement::RecreateTable(st) => Some(st.to.name.clone()),
                _ => None,
            })
            .collect();
        assert_eq!(
            recreate_order,
            vec!["alpha", "midway", "zeta"],
            "table recreation must be emitted in sorted order"
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

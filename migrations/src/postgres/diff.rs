//! Schema diff types and logic for `PostgreSQL`
//!
//! This module provides diffing between `PostgreSQL` DDL collections and
//! generates migration statements from schema changes.

use super::collection::{DiffType, EntityDiff, PostgresDDL, diff_ddl};
use super::statements::{JsonStatement, PostgresGenerator};
use crate::postgres::ddl::PostgresEntity;
use crate::postgres::snapshot::PostgresSnapshot;
use crate::traits::EntityKind;
use std::borrow::Cow;
use std::collections::{BTreeMap, HashSet};

/// Complete schema diff between two `PostgreSQL` snapshots
#[derive(Debug, Clone, Default)]
pub struct SchemaDiff {
    pub diffs: Vec<EntityDiff>,
}

impl SchemaDiff {
    #[must_use]
    pub const fn has_changes(&self) -> bool {
        !self.diffs.is_empty()
    }

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

    /// Get created schemas
    #[must_use]
    pub fn created_schemas(&self) -> Vec<&EntityDiff> {
        self.diffs
            .iter()
            .filter(|d| d.diff_type == DiffType::Create && d.kind == EntityKind::Schema)
            .collect()
    }

    /// Get created enums
    #[must_use]
    pub fn created_enums(&self) -> Vec<&EntityDiff> {
        self.diffs
            .iter()
            .filter(|d| d.diff_type == DiffType::Create && d.kind == EntityKind::Enum)
            .collect()
    }
}

/// Compare two `PostgreSQL` snapshots
#[must_use]
pub fn diff_snapshots(prev_ddl: &[PostgresEntity], cur_ddl: &[PostgresEntity]) -> SchemaDiff {
    let left = PostgresDDL::from_entities(prev_ddl.to_vec());
    let right = PostgresDDL::from_entities(cur_ddl.to_vec());
    let diffs = diff_ddl(&left, &right);

    SchemaDiff { diffs }
}

/// Compare two `PostgreSQL` DDL collections directly
#[must_use]
pub fn diff_collections(prev: &PostgresDDL, cur: &PostgresDDL) -> SchemaDiff {
    SchemaDiff {
        diffs: diff_ddl(prev, cur),
    }
}

/// Compare two full `PostgreSQL` snapshots
#[must_use]
pub fn diff_full_snapshots(prev: &PostgresSnapshot, cur: &PostgresSnapshot) -> SchemaDiff {
    diff_snapshots(&prev.ddl, &cur.ddl)
}

// =============================================================================
// Migration Diff Result
// =============================================================================

/// A schema rename operation
#[derive(Debug, Clone)]
pub struct SchemaRename {
    pub from: String,
    pub to: String,
}

/// A table rename operation
#[derive(Debug, Clone)]
pub struct TableRename {
    pub schema: String,
    pub from: String,
    pub to: String,
}

/// A column rename operation
#[derive(Debug, Clone)]
pub struct ColumnRename {
    pub schema: String,
    pub table: String,
    pub from: String,
    pub to: String,
}

/// Result of computing a migration diff
#[derive(Debug, Clone, Default)]
pub struct MigrationDiff {
    /// Generated SQL statements
    pub sql_statements: Vec<String>,
    /// Renames that occurred (for tracking in snapshot)
    pub renames: Vec<String>,
    /// Warning messages
    pub warnings: Vec<String>,
}

/// Compute a full migration diff between two `PostgreSQL` DDL states
#[must_use]
pub fn compute_migration(prev: &PostgresDDL, cur: &PostgresDDL) -> MigrationDiff {
    // Heuristic rename detection (non-interactive):
    // - detect exact schema/table renames before normal diffing
    // - detect simple column renames: one dropped + one created column in the same table
    //   with identical column properties (type/nullability/default/etc).
    let mut prev_normalized = prev.clone();
    let mut schema_renames: Vec<SchemaRename> = Vec::new();
    let mut table_renames: Vec<TableRename> = Vec::new();
    let mut column_renames: Vec<ColumnRename> = Vec::new();
    let mut rename_statements: Vec<JsonStatement> = Vec::new();
    let mut warnings = Vec::new();

    detect_and_apply_postgres_schema_renames(
        &mut prev_normalized,
        cur,
        &mut schema_renames,
        &mut rename_statements,
        &mut warnings,
    );
    detect_and_apply_postgres_table_renames(
        &mut prev_normalized,
        cur,
        &mut table_renames,
        &mut rename_statements,
        &mut warnings,
    );

    detect_and_apply_postgres_column_renames(
        &mut prev_normalized,
        cur,
        &mut column_renames,
        &mut rename_statements,
    );

    let schema_diff = diff_collections(&prev_normalized, cur);
    let generator = PostgresGenerator::new();
    let mut sql_statements = rename_statements
        .into_iter()
        .map(PostgresGenerator::statement_to_sql)
        .collect::<Vec<_>>();
    sql_statements.extend(generator.generate(&schema_diff.diffs));
    collect_enum_removal_warnings(&mut warnings, &schema_diff);
    collect_generated_recreate_warnings(&mut warnings, &schema_diff);
    collect_table_storage_warnings(&mut warnings, &schema_diff);

    MigrationDiff {
        sql_statements,
        renames: prepare_migration_renames(&schema_renames, &table_renames, &column_renames),
        warnings,
    }
}

fn quote_ident(ident: &str) -> String {
    format!("\"{}\"", ident.replace('"', "\"\""))
}

fn qualified_name(schema: &str, table: &str) -> String {
    if schema == "public" {
        quote_ident(table)
    } else {
        format!("{}.{}", quote_ident(schema), quote_ident(table))
    }
}

fn collect_enum_removal_warnings(warnings: &mut Vec<String>, schema_diff: &SchemaDiff) {
    for diff in schema_diff
        .diffs
        .iter()
        .filter(|diff| diff.diff_type == DiffType::Alter && diff.kind == EntityKind::Enum)
    {
        let (Some(PostgresEntity::Enum(old)), Some(PostgresEntity::Enum(new))) =
            (diff.left.as_ref(), diff.right.as_ref())
        else {
            continue;
        };

        for value in old
            .values
            .iter()
            .filter(|old_value| !new.values.iter().any(|new_value| new_value == *old_value))
        {
            warnings.push(format!(
                "PostgreSQL cannot drop enum value '{}.{}.{value}'; write a manual migration to rebuild dependent data safely.",
                old.schema, old.name
            ));
        }
    }
}

fn collect_generated_recreate_warnings(warnings: &mut Vec<String>, schema_diff: &SchemaDiff) {
    for diff in schema_diff
        .diffs
        .iter()
        .filter(|diff| diff.diff_type == DiffType::Alter && diff.kind == EntityKind::Column)
    {
        let (Some(PostgresEntity::Column(old)), Some(PostgresEntity::Column(new))) =
            (diff.left.as_ref(), diff.right.as_ref())
        else {
            continue;
        };

        if old.generated.is_none() && new.generated.is_some() {
            warnings.push(format!(
                "Adding a generated expression to {}.{} drops and recreates the column; existing column data will be lost.",
                qualified_name(&new.schema, &new.table),
                quote_ident(&new.name)
            ));
        }
    }
}

fn collect_table_storage_warnings(warnings: &mut Vec<String>, schema_diff: &SchemaDiff) {
    for diff in schema_diff
        .diffs
        .iter()
        .filter(|diff| diff.diff_type == DiffType::Alter && diff.kind == EntityKind::Table)
    {
        let (Some(PostgresEntity::Table(old)), Some(PostgresEntity::Table(new))) =
            (diff.left.as_ref(), diff.right.as_ref())
        else {
            continue;
        };

        if old.is_temporary.unwrap_or(false) != new.is_temporary.unwrap_or(false) {
            warnings.push(format!(
                "PostgreSQL cannot alter temporary table status for {}; write a manual migration to recreate the table if needed.",
                qualified_name(&new.schema, &new.name)
            ));
        }

        if old.inherits.as_deref() != new.inherits.as_deref() {
            warnings.push(format!(
                "PostgreSQL table inheritance changes for {} are not emitted automatically; write a manual migration if needed.",
                qualified_name(&new.schema, &new.name)
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

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct SchemaTableFingerprint {
    name: String,
    table: TableFingerprint,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct SchemaFingerprint {
    tables: Vec<SchemaTableFingerprint>,
}

fn postgres_table_fingerprint(schema: &str, table: &str, ddl: &PostgresDDL) -> TableFingerprint {
    let mut columns: Vec<_> = ddl
        .columns
        .for_table(schema, table)
        .into_iter()
        .map(|c| TableColumnFingerprint {
            name: c.name.to_string(),
            sql_type: c.sql_type.to_string(),
            not_null: c.not_null,
        })
        .collect();
    columns.sort();

    let pk_columns = ddl
        .pks
        .for_table(schema, table)
        .map_or_else(Vec::new, |pk| {
            pk.columns.iter().map(ToString::to_string).collect()
        });

    TableFingerprint {
        columns,
        pk_columns,
    }
}

fn postgres_schema_fingerprint(schema: &str, ddl: &PostgresDDL) -> SchemaFingerprint {
    let mut tables: Vec<_> = ddl
        .tables
        .list()
        .iter()
        .filter(|table| table.schema.as_ref() == schema)
        .map(|table| SchemaTableFingerprint {
            name: table.name.to_string(),
            table: postgres_table_fingerprint(schema, &table.name, ddl),
        })
        .collect();
    tables.sort();

    SchemaFingerprint { tables }
}

fn detect_and_apply_postgres_schema_renames(
    prev: &mut PostgresDDL,
    cur: &PostgresDDL,
    schema_renames: &mut Vec<SchemaRename>,
    rename_statements: &mut Vec<JsonStatement>,
    warnings: &mut Vec<String>,
) {
    let prev_schemas: Vec<String> = prev
        .schemas
        .list()
        .iter()
        .map(|schema| schema.name.to_string())
        .collect();
    let cur_schemas: Vec<String> = cur
        .schemas
        .list()
        .iter()
        .map(|schema| schema.name.to_string())
        .collect();

    let dropped: Vec<String> = prev_schemas
        .iter()
        .filter(|schema| !cur_schemas.contains(schema))
        .cloned()
        .collect();
    let created: Vec<String> = cur_schemas
        .iter()
        .filter(|schema| !prev_schemas.contains(schema))
        .cloned()
        .collect();

    let mut candidates: BTreeMap<SchemaFingerprint, (Vec<String>, Vec<String>)> = BTreeMap::new();
    for from in dropped {
        candidates
            .entry(postgres_schema_fingerprint(&from, prev))
            .or_default()
            .0
            .push(from);
    }
    for to in created {
        candidates
            .entry(postgres_schema_fingerprint(&to, cur))
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
            let (Some(from_schema), Some(to_schema)) = (
                prev.schemas.one(from).cloned(),
                cur.schemas.one(to).cloned(),
            ) else {
                continue;
            };

            schema_renames.push(SchemaRename {
                from: from.clone(),
                to: to.clone(),
            });
            rename_statements.push(JsonStatement::RenameSchema {
                from: from_schema,
                to: to_schema,
            });
            apply_postgres_schema_rename(prev, from, to);
        } else {
            warnings.push(format!(
                "Ambiguous PostgreSQL schema rename candidates between dropped schemas [{}] and created schemas [{}]; no rename was inferred. Use Options::rename_schema(...) with diff_with or diff_schemas_with to provide an explicit rename hint.",
                dropped.join(", "),
                created.join(", ")
            ));
        }
    }
}

fn detect_and_apply_postgres_table_renames(
    prev: &mut PostgresDDL,
    cur: &PostgresDDL,
    table_renames: &mut Vec<TableRename>,
    rename_statements: &mut Vec<JsonStatement>,
    warnings: &mut Vec<String>,
) {
    let prev_tables: HashSet<(String, String)> = prev
        .tables
        .list()
        .iter()
        .map(|table| (table.schema.to_string(), table.name.to_string()))
        .collect();
    let cur_tables: HashSet<(String, String)> = cur
        .tables
        .list()
        .iter()
        .map(|table| (table.schema.to_string(), table.name.to_string()))
        .collect();

    let dropped: Vec<(String, String)> = prev_tables
        .iter()
        .filter(|table| !cur_tables.contains(*table))
        .cloned()
        .collect();
    let created: Vec<(String, String)> = cur_tables
        .iter()
        .filter(|table| !prev_tables.contains(*table))
        .cloned()
        .collect();

    let mut candidates: BTreeMap<(String, TableFingerprint), (Vec<String>, Vec<String>)> =
        BTreeMap::new();
    for (schema, from) in dropped {
        candidates
            .entry((
                schema.clone(),
                postgres_table_fingerprint(&schema, &from, prev),
            ))
            .or_default()
            .0
            .push(from);
    }
    for (schema, to) in created {
        candidates
            .entry((
                schema.clone(),
                postgres_table_fingerprint(&schema, &to, cur),
            ))
            .or_default()
            .1
            .push(to);
    }

    for ((schema, _), (mut dropped, mut created)) in candidates {
        if dropped.is_empty() || created.is_empty() {
            continue;
        }

        dropped.sort();
        created.sort();

        if dropped.len() == 1 && created.len() == 1 {
            let from = &dropped[0];
            let to = &created[0];
            table_renames.push(TableRename {
                schema: schema.clone(),
                from: from.clone(),
                to: to.clone(),
            });
            rename_statements.push(JsonStatement::RenameTable {
                schema: schema.clone(),
                from: from.clone(),
                to: to.clone(),
            });
            apply_postgres_table_rename(prev, &schema, from, to);
        } else {
            warnings.push(format!(
                "Ambiguous PostgreSQL table rename candidates in schema '{}' between dropped tables [{}] and created tables [{}]; no rename was inferred. Use Options::rename_table_in(...) with diff_with or diff_schemas_with to provide an explicit rename hint.",
                schema,
                dropped.join(", "),
                created.join(", ")
            ));
        }
    }
}

fn detect_and_apply_postgres_column_renames(
    prev: &mut PostgresDDL,
    cur: &PostgresDDL,
    out: &mut Vec<ColumnRename>,
    rename_statements: &mut Vec<JsonStatement>,
) {
    let common_tables: Vec<(String, String)> = prev
        .tables
        .list()
        .iter()
        .map(|t| (t.schema.to_string(), t.name.to_string()))
        .filter(|(schema, table)| cur.tables.one(schema, table).is_some())
        .collect();

    for (schema, table) in common_tables {
        let prev_cols = prev.columns.for_table(&schema, &table);
        let cur_cols = cur.columns.for_table(&schema, &table);

        let prev_names: HashSet<String> = prev_cols.iter().map(|c| c.name.to_string()).collect();
        let cur_names: HashSet<String> = cur_cols.iter().map(|c| c.name.to_string()).collect();

        let dropped: Vec<String> = prev_names.difference(&cur_names).cloned().collect();
        let created: Vec<String> = cur_names.difference(&prev_names).cloned().collect();

        if dropped.len() != 1 || created.len() != 1 {
            continue;
        }

        let from = &dropped[0];
        let to = &created[0];

        let prev_col = prev.columns.one(&schema, &table, from);
        let cur_col = cur.columns.one(&schema, &table, to);
        if let (Some(prev_col), Some(cur_col)) = (prev_col, cur_col) {
            let mut prev_cmp = prev_col.clone();
            prev_cmp.name.clone_from(&cur_col.name);
            if prev_cmp == *cur_col {
                out.push(ColumnRename {
                    schema: schema.clone(),
                    table: table.clone(),
                    from: from.clone(),
                    to: to.clone(),
                });
                rename_statements.push(JsonStatement::RenameColumn {
                    from: Box::new(prev_col.clone()),
                    to: Box::new(cur_col.clone()),
                });
                apply_postgres_column_rename(prev, &schema, &table, from, to);
            }
        }
    }
}

fn rewrite_cow(value: &mut Cow<'static, str>, from: &str, to: &str) {
    if value.as_ref() == from {
        *value = to.to_string().into();
    }
}

fn rewrite_optional_cow(value: &mut Option<Cow<'static, str>>, from: &str, to: &str) {
    if value.as_deref() == Some(from) {
        *value = Some(to.to_string().into());
    }
}

fn rewrite_schema_qualified_value(value: &mut Option<Cow<'static, str>>, from: &str, to: &str) {
    let Some(current) = value.as_deref() else {
        return;
    };
    let Some(rest) = current
        .strip_prefix(from)
        .and_then(|rest| rest.strip_prefix('.'))
    else {
        return;
    };
    *value = Some(format!("{to}.{rest}").into());
}

fn apply_postgres_schema_rename(ddl: &mut PostgresDDL, from: &str, to: &str) {
    for schema in ddl.schemas.list_mut() {
        rewrite_cow(&mut schema.name, from, to);
    }

    for table in ddl.tables.list_mut() {
        rewrite_cow(&mut table.schema, from, to);
        rewrite_schema_qualified_value(&mut table.inherits, from, to);
    }

    for column in ddl.columns.list_mut() {
        rewrite_cow(&mut column.schema, from, to);
        rewrite_optional_cow(&mut column.type_schema, from, to);
        if let Some(identity) = &mut column.identity {
            rewrite_optional_cow(&mut identity.schema, from, to);
        }
    }

    for index in ddl.indexes.list_mut() {
        rewrite_cow(&mut index.schema, from, to);
    }

    for fk in ddl.fks.list_mut() {
        rewrite_cow(&mut fk.schema, from, to);
        rewrite_cow(&mut fk.schema_to, from, to);
    }

    for pk in ddl.pks.list_mut() {
        rewrite_cow(&mut pk.schema, from, to);
    }

    for unique in ddl.uniques.list_mut() {
        rewrite_cow(&mut unique.schema, from, to);
    }

    for check in ddl.checks.list_mut() {
        rewrite_cow(&mut check.schema, from, to);
    }

    for policy in ddl.policies.list_mut() {
        rewrite_cow(&mut policy.schema, from, to);
    }

    for enum_ in ddl.enums.list_mut() {
        rewrite_cow(&mut enum_.schema, from, to);
    }

    for sequence in ddl.sequences.list_mut() {
        rewrite_cow(&mut sequence.schema, from, to);
    }

    for view in ddl.views.list_mut() {
        rewrite_cow(&mut view.schema, from, to);
    }
}

fn apply_postgres_table_rename(ddl: &mut PostgresDDL, schema: &str, from: &str, to: &str) {
    for table in ddl.tables.list_mut() {
        if table.schema.as_ref() == schema && table.name.as_ref() == from {
            table.name = to.to_string().into();
        }

        if table.schema.as_ref() == schema
            && let Some(inherits) = &mut table.inherits
        {
            if inherits.as_ref() == from {
                *inherits = to.to_string().into();
            } else if inherits.as_ref() == format!("{schema}.{from}") {
                *inherits = format!("{schema}.{to}").into();
            }
        }
    }

    for column in ddl
        .columns
        .list_mut()
        .iter_mut()
        .filter(|column| column.schema.as_ref() == schema && column.table.as_ref() == from)
    {
        column.table = to.to_string().into();
    }

    for pk in ddl
        .pks
        .list_mut()
        .iter_mut()
        .filter(|pk| pk.schema.as_ref() == schema && pk.table.as_ref() == from)
    {
        pk.table = to.to_string().into();
    }

    for unique in ddl
        .uniques
        .list_mut()
        .iter_mut()
        .filter(|unique| unique.schema.as_ref() == schema && unique.table.as_ref() == from)
    {
        unique.table = to.to_string().into();
    }

    for check in ddl
        .checks
        .list_mut()
        .iter_mut()
        .filter(|check| check.schema.as_ref() == schema && check.table.as_ref() == from)
    {
        check.table = to.to_string().into();
    }

    for index in ddl
        .indexes
        .list_mut()
        .iter_mut()
        .filter(|index| index.schema.as_ref() == schema && index.table.as_ref() == from)
    {
        index.table = to.to_string().into();
    }

    for policy in ddl
        .policies
        .list_mut()
        .iter_mut()
        .filter(|policy| policy.schema.as_ref() == schema && policy.table.as_ref() == from)
    {
        policy.table = to.to_string().into();
    }

    for fk in ddl.fks.list_mut() {
        if fk.schema.as_ref() == schema && fk.table.as_ref() == from {
            fk.table = to.to_string().into();
        }
        if fk.schema_to.as_ref() == schema && fk.table_to.as_ref() == from {
            fk.table_to = to.to_string().into();
        }
    }
}

fn apply_postgres_column_rename(
    ddl: &mut PostgresDDL,
    schema: &str,
    table: &str,
    from: &str,
    to: &str,
) {
    let to = to.to_string();
    // Columns
    for c in ddl.columns.list_mut().iter_mut() {
        if c.schema.as_ref() == schema && c.table.as_ref() == table && c.name.as_ref() == from {
            c.name = to.clone().into();
        }
    }

    // PKs
    for pk in ddl
        .pks
        .list_mut()
        .iter_mut()
        .filter(|p| p.schema.as_ref() == schema && p.table.as_ref() == table)
    {
        for col in pk.columns.to_mut().iter_mut() {
            if col.as_ref() == from {
                *col = to.clone().into();
            }
        }
    }

    // Uniques
    for u in ddl
        .uniques
        .list_mut()
        .iter_mut()
        .filter(|u| u.schema.as_ref() == schema && u.table.as_ref() == table)
    {
        for col in u.columns.to_mut().iter_mut() {
            if col.as_ref() == from {
                *col = to.clone().into();
            }
        }
    }

    // FKs (both table side and referenced side)
    for fk in ddl.fks.list_mut().iter_mut() {
        if fk.schema.as_ref() == schema && fk.table.as_ref() == table {
            for col in fk.columns.to_mut().iter_mut() {
                if col.as_ref() == from {
                    *col = to.clone().into();
                }
            }
        }
        if fk.schema_to.as_ref() == schema && fk.table_to.as_ref() == table {
            for col in fk.columns_to.to_mut().iter_mut() {
                if col.as_ref() == from {
                    *col = to.clone().into();
                }
            }
        }
    }

    // Indexes
    for idx in ddl
        .indexes
        .list_mut()
        .iter_mut()
        .filter(|i| i.schema.as_ref() == schema && i.table.as_ref() == table)
    {
        for col in &mut idx.columns {
            if !col.is_expression && col.value.as_ref() == from {
                col.value = to.clone().into();
            }
        }
    }
}

/// Compute a migration from snapshots
#[must_use]
pub fn compute_migration_from_snapshots(
    prev: &PostgresSnapshot,
    cur: &PostgresSnapshot,
) -> MigrationDiff {
    let prev_ddl = PostgresDDL::from_entities(prev.ddl.clone());
    let cur_ddl = PostgresDDL::from_entities(cur.ddl.clone());
    compute_migration(&prev_ddl, &cur_ddl)
}

/// Prepare rename tracking strings for snapshot storage
#[must_use]
pub fn prepare_migration_renames(
    schema_renames: &[SchemaRename],
    table_renames: &[TableRename],
    column_renames: &[ColumnRename],
) -> Vec<String> {
    let mut renames = Vec::new();

    for sr in schema_renames {
        renames.push(format!("schema:{}:{}", sr.from, sr.to));
    }

    for tr in table_renames {
        renames.push(format!(
            "table:{}.{}:{}.{}",
            tr.schema, tr.from, tr.schema, tr.to
        ));
    }

    for cr in column_renames {
        renames.push(format!(
            "column:{}.{}.{}:{}.{}.{}",
            cr.schema, cr.table, cr.from, cr.schema, cr.table, cr.to
        ));
    }

    renames
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::postgres::collection::PostgresDDL;
    use crate::postgres::ddl::{
        Column, Enum, ForeignKey, Generated, GeneratedType, Index, IndexColumn, Policy, Schema,
        Table,
    };

    fn postgres_table_with_id(schema: &str, table: &str) -> PostgresDDL {
        let mut ddl = PostgresDDL::new();
        ddl.schemas.push(Schema::new(schema.to_string()));
        ddl.tables
            .push(Table::new(schema.to_string(), table.to_string()));
        ddl.columns
            .push(Column::new(schema.to_string(), table.to_string(), "id", "integer").not_null());
        ddl
    }

    #[test]
    fn test_empty_diff() {
        let prev = Vec::new();
        let cur = Vec::new();

        let diff = diff_snapshots(&prev, &cur);
        assert!(!diff.has_changes());
    }

    #[test]
    fn test_schema_creation() {
        let prev = Vec::new();
        let cur = vec![PostgresEntity::Schema(Schema::new("myschema"))];

        let diff = diff_snapshots(&prev, &cur);
        assert!(diff.has_changes());
        assert_eq!(diff.created_schemas().len(), 1);
    }

    #[test]
    fn test_table_creation() {
        let prev = Vec::new();
        let cur = vec![
            PostgresEntity::Schema(Schema::new("public")),
            PostgresEntity::Table(Table {
                schema: "public".into(),
                name: "users".into(),
                is_unlogged: None,
                is_temporary: None,
                inherits: None,
                tablespace: None,
                is_rls_enabled: None,
                comment: None,
            }),
            PostgresEntity::Column(Column {
                schema: "public".into(),
                table: "users".into(),
                name: "id".into(),
                sql_type: "integer".into(),
                type_schema: None,
                not_null: true,
                default: None,
                generated: None,
                identity: None,
                dimensions: None,
                comment: None,
                collate: None,
                ordinal_position: None,
            }),
        ];

        let diff = diff_snapshots(&prev, &cur);
        assert!(diff.has_changes());
        assert_eq!(diff.created_tables().len(), 1);
    }

    #[test]
    fn pure_table_rename_emits_single_rename_statement() {
        let prev = postgres_table_with_id("public", "users");
        let cur = postgres_table_with_id("public", "accounts");

        let migration = compute_migration(&prev, &cur);

        assert_eq!(
            migration.sql_statements,
            vec!["ALTER TABLE \"users\" RENAME TO \"accounts\";"]
        );
        assert!(
            !migration
                .sql_statements
                .iter()
                .any(|statement| statement.starts_with("DROP TABLE"))
        );
    }

    #[test]
    fn table_rename_rewrites_indexes_foreign_keys_and_policies() {
        let mut prev = postgres_table_with_id("public", "users");
        prev.tables.push(Table::new("public", "posts"));
        prev.columns
            .push(Column::new("public", "posts", "id", "integer").not_null());
        prev.columns
            .push(Column::new("public", "posts", "user_id", "integer"));
        prev.indexes.push(Index::new(
            "public",
            "users",
            "idx_users_id",
            vec![IndexColumn::new("id")],
        ));
        prev.fks.push(ForeignKey::from_strings(
            "public".to_string(),
            "posts".to_string(),
            "fk_posts_user".to_string(),
            vec!["user_id".to_string()],
            "public".to_string(),
            "users".to_string(),
            vec!["id".to_string()],
        ));
        prev.policies
            .push(Policy::new("public", "users", "users_policy"));

        let mut cur = postgres_table_with_id("public", "accounts");
        cur.tables.push(Table::new("public", "posts"));
        cur.columns
            .push(Column::new("public", "posts", "id", "integer").not_null());
        cur.columns
            .push(Column::new("public", "posts", "user_id", "integer"));
        cur.indexes.push(Index::new(
            "public",
            "accounts",
            "idx_users_id",
            vec![IndexColumn::new("id")],
        ));
        cur.fks.push(ForeignKey::from_strings(
            "public".to_string(),
            "posts".to_string(),
            "fk_posts_user".to_string(),
            vec!["user_id".to_string()],
            "public".to_string(),
            "accounts".to_string(),
            vec!["id".to_string()],
        ));
        cur.policies
            .push(Policy::new("public", "accounts", "users_policy"));

        let migration = compute_migration(&prev, &cur);

        assert_eq!(
            migration.sql_statements,
            vec!["ALTER TABLE \"users\" RENAME TO \"accounts\";"]
        );
    }

    #[test]
    fn ambiguous_table_rename_does_not_guess_and_warns() {
        let mut prev = postgres_table_with_id("public", "users");
        let mut admins = postgres_table_with_id("public", "admins");
        admins.schemas.list_mut().clear();
        prev.tables.list_mut().append(admins.tables.list_mut());
        prev.columns.list_mut().append(admins.columns.list_mut());

        let cur = postgres_table_with_id("public", "accounts");

        let migration = compute_migration(&prev, &cur);

        assert!(
            migration.warnings.iter().any(|warning| warning
                .contains("Ambiguous PostgreSQL table rename candidates")
                && warning.contains("rename_table_in")),
            "expected ambiguous rename warning, got {:?}",
            migration.warnings
        );
        assert!(
            !migration
                .sql_statements
                .iter()
                .any(|statement| statement.contains("RENAME TO"))
        );
    }

    #[test]
    fn schema_rename_rekeys_tables_under_schema() {
        let prev = postgres_table_with_id("old_schema", "users");
        let cur = postgres_table_with_id("new_schema", "users");

        let migration = compute_migration(&prev, &cur);

        assert_eq!(
            migration.sql_statements,
            vec!["ALTER SCHEMA \"old_schema\" RENAME TO \"new_schema\";"]
        );
        assert!(
            !migration
                .sql_statements
                .iter()
                .any(|statement| statement.starts_with("DROP TABLE")
                    || statement.starts_with("CREATE TABLE"))
        );
    }

    #[test]
    fn test_column_not_null_change_generates_sql() {
        // Test that changing nullable to not null generates ALTER COLUMN SQL
        let mut prev_ddl = PostgresDDL::new();
        prev_ddl.tables.push(Table {
            schema: "public".into(),
            name: "users".into(),
            is_unlogged: None,
            is_temporary: None,
            inherits: None,
            tablespace: None,
            is_rls_enabled: None,
            comment: None,
        });
        prev_ddl.columns.push(Column {
            schema: "public".into(),
            table: "users".into(),
            name: "email".into(),
            sql_type: "text".into(),
            type_schema: None,
            not_null: false, // nullable
            default: None,
            generated: None,
            identity: None,
            dimensions: None,
            comment: None,
            collate: None,
            ordinal_position: None,
        });

        let mut cur_ddl = PostgresDDL::new();
        cur_ddl.tables.push(Table {
            schema: "public".into(),
            name: "users".into(),
            is_unlogged: None,
            is_temporary: None,
            inherits: None,
            tablespace: None,
            is_rls_enabled: None,
            comment: None,
        });
        cur_ddl.columns.push(Column {
            schema: "public".into(),
            table: "users".into(),
            name: "email".into(),
            sql_type: "text".into(),
            type_schema: None,
            not_null: true, // not null
            default: None,
            generated: None,
            identity: None,
            dimensions: None,
            comment: None,
            collate: None,
            ordinal_position: None,
        });

        let migration = compute_migration(&prev_ddl, &cur_ddl);

        // Should have generated SQL statements
        assert!(
            !migration.sql_statements.is_empty(),
            "Should generate SQL statements"
        );

        // Check the SQL contains ALTER COLUMN SET NOT NULL
        assert_eq!(migration.sql_statements.len(), 1);
        assert_eq!(
            migration.sql_statements[0],
            "ALTER TABLE \"users\" ALTER COLUMN \"email\" SET NOT NULL;"
        );
    }

    #[test]
    fn test_column_type_change_generates_sql() {
        // Test that changing column type generates ALTER COLUMN SQL
        let mut prev_ddl = PostgresDDL::new();
        prev_ddl.tables.push(Table {
            schema: "public".into(),
            name: "users".into(),
            is_unlogged: None,
            is_temporary: None,
            inherits: None,
            tablespace: None,
            is_rls_enabled: None,
            comment: None,
        });
        prev_ddl.columns.push(Column {
            schema: "public".into(),
            table: "users".into(),
            name: "age".into(),
            sql_type: "text".into(), // text
            type_schema: None,
            not_null: false,
            default: None,
            generated: None,
            identity: None,
            dimensions: None,
            comment: None,
            collate: None,
            ordinal_position: None,
        });

        let mut cur_ddl = PostgresDDL::new();
        cur_ddl.tables.push(Table {
            schema: "public".into(),
            name: "users".into(),
            is_unlogged: None,
            is_temporary: None,
            inherits: None,
            tablespace: None,
            is_rls_enabled: None,
            comment: None,
        });
        cur_ddl.columns.push(Column {
            schema: "public".into(),
            table: "users".into(),
            name: "age".into(),
            sql_type: "integer".into(), // integer
            type_schema: None,
            not_null: false,
            default: None,
            generated: None,
            identity: None,
            dimensions: None,
            comment: None,
            collate: None,
            ordinal_position: None,
        });

        let migration = compute_migration(&prev_ddl, &cur_ddl);

        // Should have generated SQL statements
        assert!(
            !migration.sql_statements.is_empty(),
            "Should generate SQL statements"
        );

        // Check the SQL contains ALTER COLUMN SET DATA TYPE with USING cast
        assert_eq!(migration.sql_statements.len(), 1);
        assert_eq!(
            migration.sql_statements[0],
            "ALTER TABLE \"users\" ALTER COLUMN \"age\" SET DATA TYPE integer USING \"age\"::integer;"
        );
    }

    #[test]
    fn test_column_default_change_generates_sql() {
        // Test that changing column default generates ALTER COLUMN SQL
        let mut prev_ddl = PostgresDDL::new();
        prev_ddl.tables.push(Table {
            schema: "public".into(),
            name: "users".into(),
            is_unlogged: None,
            is_temporary: None,
            inherits: None,
            tablespace: None,
            is_rls_enabled: None,
            comment: None,
        });
        prev_ddl.columns.push(Column {
            schema: "public".into(),
            table: "users".into(),
            name: "status".into(),
            sql_type: "text".into(),
            type_schema: None,
            not_null: false,
            default: None, // no default
            generated: None,
            identity: None,
            dimensions: None,
            comment: None,
            collate: None,
            ordinal_position: None,
        });

        let mut cur_ddl = PostgresDDL::new();
        cur_ddl.tables.push(Table {
            schema: "public".into(),
            name: "users".into(),
            is_unlogged: None,
            is_temporary: None,
            inherits: None,
            tablespace: None,
            is_rls_enabled: None,
            comment: None,
        });
        cur_ddl.columns.push(Column {
            schema: "public".into(),
            table: "users".into(),
            name: "status".into(),
            sql_type: "text".into(),
            type_schema: None,
            not_null: false,
            default: Some("'active'".into()), // has default
            generated: None,
            identity: None,
            dimensions: None,
            comment: None,
            collate: None,
            ordinal_position: None,
        });

        let migration = compute_migration(&prev_ddl, &cur_ddl);

        // Should have generated SQL statements
        assert!(
            !migration.sql_statements.is_empty(),
            "Should generate SQL statements"
        );

        // Check the SQL contains ALTER COLUMN SET DEFAULT
        assert_eq!(migration.sql_statements.len(), 1);
        assert_eq!(
            migration.sql_statements[0],
            "ALTER TABLE \"users\" ALTER COLUMN \"status\" SET DEFAULT 'active';"
        );
    }

    #[test]
    fn enum_value_removal_emits_warning() {
        let mut prev_ddl = PostgresDDL::new();
        prev_ddl.enums.push(Enum::from_strings(
            "public".to_string(),
            "status".to_string(),
            vec!["active".to_string(), "archived".to_string()],
        ));

        let mut cur_ddl = PostgresDDL::new();
        cur_ddl.enums.push(Enum::from_strings(
            "public".to_string(),
            "status".to_string(),
            vec!["active".to_string()],
        ));

        let migration = compute_migration(&prev_ddl, &cur_ddl);
        assert!(
            migration
                .warnings
                .iter()
                .any(|warning| warning.contains("cannot drop enum value")),
            "expected enum removal warning, got {:?}",
            migration.warnings
        );
    }

    #[test]
    fn enum_mid_list_addition_uses_before_clause() {
        let mut prev_ddl = PostgresDDL::new();
        prev_ddl.enums.push(Enum::from_strings(
            "public".to_string(),
            "status".to_string(),
            vec!["active".to_string(), "archived".to_string()],
        ));

        let mut cur_ddl = PostgresDDL::new();
        cur_ddl.enums.push(Enum::from_strings(
            "public".to_string(),
            "status".to_string(),
            vec![
                "active".to_string(),
                "pending".to_string(),
                "archived".to_string(),
            ],
        ));

        let migration = compute_migration(&prev_ddl, &cur_ddl);
        assert_eq!(
            migration.sql_statements,
            vec!["ALTER TYPE \"status\" ADD VALUE 'pending' BEFORE 'archived';"]
        );
    }

    #[test]
    fn adding_generated_expression_emits_data_loss_warning() {
        let mut prev_ddl = PostgresDDL::new();
        prev_ddl.tables.push(Table::new("public", "users"));
        prev_ddl
            .columns
            .push(Column::new("public", "users", "name_len", "integer"));

        let mut cur_ddl = prev_ddl.clone();
        cur_ddl.columns.entities.clear();
        let mut generated = Column::new("public", "users", "name_len", "integer");
        generated.generated = Some(Generated {
            expression: "length(name)".into(),
            gen_type: GeneratedType::Stored,
        });
        cur_ddl.columns.push(generated);

        let migration = compute_migration(&prev_ddl, &cur_ddl);
        assert!(
            migration
                .warnings
                .iter()
                .any(|warning| warning.contains("drops and recreates the column")),
            "expected generated column recreation warning, got {:?}",
            migration.warnings
        );
    }
}

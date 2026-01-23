//! Schema diff types and logic for PostgreSQL
//!
//! This module provides diffing between PostgreSQL DDL collections and
//! generates migration statements from schema changes.

use super::collection::{DiffType, EntityDiff, PostgresDDL, diff_ddl};
use super::statements::PostgresGenerator;
use crate::postgres::ddl::PostgresEntity;
use crate::postgres::snapshot::PostgresSnapshot;
use crate::traits::EntityKind;
use std::collections::HashSet;

/// Complete schema diff between two PostgreSQL snapshots
#[derive(Debug, Clone, Default)]
pub struct SchemaDiff {
    pub diffs: Vec<EntityDiff>,
}

impl SchemaDiff {
    pub fn has_changes(&self) -> bool {
        !self.diffs.is_empty()
    }

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

    /// Get created schemas
    pub fn created_schemas(&self) -> Vec<&EntityDiff> {
        self.diffs
            .iter()
            .filter(|d| d.diff_type == DiffType::Create && d.kind == EntityKind::Schema)
            .collect()
    }

    /// Get created enums
    pub fn created_enums(&self) -> Vec<&EntityDiff> {
        self.diffs
            .iter()
            .filter(|d| d.diff_type == DiffType::Create && d.kind == EntityKind::Enum)
            .collect()
    }
}

/// Compare two PostgreSQL snapshots
pub fn diff_snapshots(prev_ddl: &[PostgresEntity], cur_ddl: &[PostgresEntity]) -> SchemaDiff {
    let left = PostgresDDL::from_entities(prev_ddl.to_vec());
    let right = PostgresDDL::from_entities(cur_ddl.to_vec());
    let diffs = diff_ddl(&left, &right);

    SchemaDiff { diffs }
}

/// Compare two PostgreSQL DDL collections directly
pub fn diff_collections(prev: &PostgresDDL, cur: &PostgresDDL) -> SchemaDiff {
    SchemaDiff {
        diffs: diff_ddl(prev, cur),
    }
}

/// Compare two full PostgreSQL snapshots
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

/// Compute a full migration diff between two PostgreSQL DDL states
pub fn compute_migration(prev: &PostgresDDL, cur: &PostgresDDL) -> MigrationDiff {
    // Heuristic rename detection (non-interactive):
    // Detect simple column renames: one dropped + one created column in the same table
    // with identical column properties (type/nullability/default/etc).
    let mut prev_normalized = prev.clone();
    let mut column_renames: Vec<ColumnRename> = Vec::new();
    let mut rename_sql: Vec<String> = Vec::new();

    detect_and_apply_postgres_column_renames(&mut prev_normalized, cur, &mut column_renames, &mut rename_sql);

    let schema_diff = diff_collections(&prev_normalized, cur);
    let generator = PostgresGenerator::new();
    let mut sql_statements = rename_sql;
    sql_statements.extend(generator.generate(&schema_diff.diffs));

    MigrationDiff {
        sql_statements,
        renames: prepare_migration_renames(&[], &[], &column_renames),
        warnings: Vec::new(),
    }
}

fn detect_and_apply_postgres_column_renames(
    prev: &mut PostgresDDL,
    cur: &PostgresDDL,
    out: &mut Vec<ColumnRename>,
    rename_sql: &mut Vec<String>,
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
            prev_cmp.name = cur_col.name.clone();
            if prev_cmp == *cur_col {
                out.push(ColumnRename {
                    schema: schema.clone(),
                    table: table.clone(),
                    from: from.clone(),
                    to: to.clone(),
                });
                rename_sql.push(format!(
                    "ALTER TABLE \"{}\".\"{}\" RENAME COLUMN \"{}\" TO \"{}\";",
                    schema, table, from, to
                ));
                apply_postgres_column_rename(prev, &schema, &table, from, to);
            }
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
    for pk in ddl.pks.list_mut().iter_mut().filter(|p| p.schema.as_ref() == schema && p.table.as_ref() == table) {
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
        for col in idx.columns.iter_mut() {
            if !col.is_expression && col.value.as_ref() == from {
                col.value = to.clone().into();
            }
        }
    }
}

/// Compute a migration from snapshots
pub fn compute_migration_from_snapshots(
    prev: &PostgresSnapshot,
    cur: &PostgresSnapshot,
) -> MigrationDiff {
    let prev_ddl = PostgresDDL::from_entities(prev.ddl.clone());
    let cur_ddl = PostgresDDL::from_entities(cur.ddl.clone());
    compute_migration(&prev_ddl, &cur_ddl)
}

/// Prepare rename tracking strings for snapshot storage
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
    use crate::postgres::ddl::{Column, Schema, Table};

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
                is_rls_enabled: None,
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
                ordinal_position: None,
            }),
        ];

        let diff = diff_snapshots(&prev, &cur);
        assert!(diff.has_changes());
        assert_eq!(diff.created_tables().len(), 1);
    }

    #[test]
    fn test_column_not_null_change_generates_sql() {
        // Test that changing nullable to not null generates ALTER COLUMN SQL
        let mut prev_ddl = PostgresDDL::new();
        prev_ddl.tables.push(Table {
            schema: "public".into(),
            name: "users".into(),
            is_rls_enabled: None,
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
            ordinal_position: None,
        });

        let mut cur_ddl = PostgresDDL::new();
        cur_ddl.tables.push(Table {
            schema: "public".into(),
            name: "users".into(),
            is_rls_enabled: None,
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
            ordinal_position: None,
        });

        let migration = compute_migration(&prev_ddl, &cur_ddl);

        // Should have generated SQL statements
        assert!(
            !migration.sql_statements.is_empty(),
            "Should generate SQL statements"
        );

        // Check the SQL contains SET NOT NULL
        let sql = migration.sql_statements.join("\n");
        assert!(
            sql.contains("SET NOT NULL"),
            "Should contain SET NOT NULL: {}",
            sql
        );
    }

    #[test]
    fn test_column_type_change_generates_sql() {
        // Test that changing column type generates ALTER COLUMN SQL
        let mut prev_ddl = PostgresDDL::new();
        prev_ddl.tables.push(Table {
            schema: "public".into(),
            name: "users".into(),
            is_rls_enabled: None,
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
            ordinal_position: None,
        });

        let mut cur_ddl = PostgresDDL::new();
        cur_ddl.tables.push(Table {
            schema: "public".into(),
            name: "users".into(),
            is_rls_enabled: None,
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
            ordinal_position: None,
        });

        let migration = compute_migration(&prev_ddl, &cur_ddl);

        // Should have generated SQL statements
        assert!(
            !migration.sql_statements.is_empty(),
            "Should generate SQL statements"
        );

        // Check the SQL contains SET DATA TYPE
        let sql = migration.sql_statements.join("\n");
        assert!(
            sql.contains("SET DATA TYPE"),
            "Should contain SET DATA TYPE: {}",
            sql
        );
    }

    #[test]
    fn test_column_default_change_generates_sql() {
        // Test that changing column default generates ALTER COLUMN SQL
        let mut prev_ddl = PostgresDDL::new();
        prev_ddl.tables.push(Table {
            schema: "public".into(),
            name: "users".into(),
            is_rls_enabled: None,
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
            ordinal_position: None,
        });

        let mut cur_ddl = PostgresDDL::new();
        cur_ddl.tables.push(Table {
            schema: "public".into(),
            name: "users".into(),
            is_rls_enabled: None,
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
            ordinal_position: None,
        });

        let migration = compute_migration(&prev_ddl, &cur_ddl);

        // Should have generated SQL statements
        assert!(
            !migration.sql_statements.is_empty(),
            "Should generate SQL statements"
        );

        // Check the SQL contains SET DEFAULT
        let sql = migration.sql_statements.join("\n");
        assert!(
            sql.contains("SET DEFAULT"),
            "Should contain SET DEFAULT: {}",
            sql
        );
    }
}

//! Schema diff types and logic for PostgreSQL
//!
//! This module provides diffing between PostgreSQL DDL collections and
//! generates migration statements from schema changes.

use super::collection::{DiffType, EntityDiff, PostgresDDL, diff_ddl};
use super::statements::PostgresGenerator;
use crate::postgres::ddl::PostgresEntity;
use crate::postgres::snapshot::PostgresSnapshot;
use crate::traits::EntityKind;

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
    let schema_diff = diff_collections(prev, cur);
    let generator = PostgresGenerator::new();
    let sql_statements = generator.generate(&schema_diff.diffs);

    MigrationDiff {
        sql_statements,
        renames: Vec::new(),
        warnings: Vec::new(),
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
        let cur = vec![PostgresEntity::Schema(Schema {
            name: "myschema".to_string(),
        })];

        let diff = diff_snapshots(&prev, &cur);
        assert!(diff.has_changes());
        assert_eq!(diff.created_schemas().len(), 1);
    }

    #[test]
    fn test_table_creation() {
        let prev = Vec::new();
        let cur = vec![
            PostgresEntity::Schema(Schema {
                name: "public".to_string(),
            }),
            PostgresEntity::Table(Table {
                schema: "public".to_string(),
                name: "users".to_string(),
                is_rls_enabled: None,
            }),
            PostgresEntity::Column(Column {
                schema: "public".to_string(),
                table: "users".to_string(),
                name: "id".to_string(),
                sql_type: "integer".to_string(),
                type_schema: None,
                not_null: true,
                default: None,
                generated: None,
                identity: None,
                dimensions: None,
            }),
        ];

        let diff = diff_snapshots(&prev, &cur);
        assert!(diff.has_changes());
        assert_eq!(diff.created_tables().len(), 1);
    }
}

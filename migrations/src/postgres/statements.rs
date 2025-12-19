//! PostgreSQL SQL generation from schema metadata

use super::collection::{DiffType, EntityDiff};
use super::ddl::{
    CheckConstraint, Column, Enum, ForeignKey, Index, Policy, PostgresEntity, PrimaryKey, Role,
    Schema, Sequence, Table, UniqueConstraint, View,
};
use crate::traits::EntityKind;
use serde::Serialize;
use std::collections::HashMap;

pub const BREAKPOINT: &str = "--> statement-breakpoint";

// =============================================================================
// JSON Statements
// =============================================================================

#[derive(Serialize, Debug, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum JsonStatement {
    CreateTable {
        table: RichTable,
    },
    DropTable {
        table: Table,
        #[serde(rename = "key")]
        table_key: String,
    },
    RenameTable {
        schema: String,
        from: String,
        to: String,
    },
    AddColumn {
        column: Column,
        #[serde(rename = "isPK")]
        is_pk: bool,
        #[serde(rename = "isCompositePK")]
        is_composite_pk: bool,
    },
    DropColumn {
        column: Column,
    },
    AlterColumn {
        to: Column,
        #[serde(rename = "wasEnum")]
        was_enum: bool,
        #[serde(rename = "isEnum")]
        is_enum: bool,
        diff: HashMap<String, serde_json::Value>, // simplified diff structure
    },
    RenameColumn {
        from: Box<Column>,
        to: Box<Column>,
    },
    CreateIndex {
        index: Index,
    },
    DropIndex {
        index: Index,
    },
    CreateFk {
        fk: ForeignKey,
    },
    DropFk {
        fk: ForeignKey,
    },
    AddPk {
        pk: PrimaryKey,
    },
    DropPk {
        pk: PrimaryKey,
    },
    AddUnique {
        unique: UniqueConstraint,
    },
    DropUnique {
        unique: UniqueConstraint,
    },
    AddCheck {
        check: CheckConstraint,
    },
    DropCheck {
        check: CheckConstraint,
    },
    CreateSchema {
        name: String,
    },
    DropSchema {
        name: String,
    },
    RenameSchema {
        from: Schema,
        to: Schema,
    },
    CreateEnum {
        #[serde(rename = "enum")]
        enum_: Enum,
    },
    DropEnum {
        #[serde(rename = "enum")]
        enum_: Enum,
    },
    AlterEnum {
        from: Enum,
        to: Enum,
        diff: Vec<EnumDiff>,
    },
    CreateSequence {
        sequence: Sequence,
    },
    DropSequence {
        sequence: Sequence,
    },
    CreateView {
        view: View,
    },
    DropView {
        view: View,
    },
    CreateRole {
        role: Role,
    },
    DropRole {
        role: Role,
    },
    CreatePolicy {
        policy: Policy,
    },
    DropPolicy {
        policy: Policy,
    },
    // Missing some less common ones for brevity but covering core DDL
}

#[derive(Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct EnumDiff {
    pub r#type: String, // "added"
    pub value: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub before_value: Option<String>,
}

/// A "Rich" table structure that includes sub-entities (columns, constraints)
/// needed for CREATE TABLE statement generation.
#[derive(Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RichTable {
    pub name: String,
    pub schema: String,
    pub columns: Vec<Column>,
    pub indexes: Vec<Index>,
    pub foreign_keys: Vec<ForeignKey>,
    pub pk: Option<PrimaryKey>,
    pub uniques: Vec<UniqueConstraint>,
    pub checks: Vec<CheckConstraint>,
    pub policies: Vec<Policy>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_rls_enabled: Option<bool>,
}

// =============================================================================
// Generator
// =============================================================================

pub struct PostgresGenerator {
    pub breakpoints: bool,
}

impl Default for PostgresGenerator {
    fn default() -> Self {
        Self::new()
    }
}

impl PostgresGenerator {
    pub fn new() -> Self {
        Self { breakpoints: true }
    }

    pub fn with_breakpoints(mut self, breakpoints: bool) -> Self {
        self.breakpoints = breakpoints;
        self
    }

    pub fn generate(&self, diff: &[EntityDiff]) -> Vec<String> {
        let mut sqls = Vec::new();

        // 1. Identify created tables to group their components
        let created_tables: Vec<String> = diff
            .iter()
            .filter(|d| d.diff_type == DiffType::Create && d.kind == EntityKind::Table)
            .map(|d| d.name.clone())
            .collect();

        // 2. Identify dropped tables to filter their components
        let dropped_tables: Vec<String> = diff
            .iter()
            .filter(|d| d.diff_type == DiffType::Drop && d.kind == EntityKind::Table)
            .map(|d| d.name.clone())
            .collect();

        // 3. Process Schema creations first
        for d in diff.iter().filter(|d| d.kind == EntityKind::Schema) {
            if let Some(stmt) = self.diff_to_statement_with_context(d, diff) {
                sqls.push(self.statement_to_sql(stmt));
            }
        }

        // 4. Process Enum creations
        for d in diff.iter().filter(|d| d.kind == EntityKind::Enum) {
            if let Some(stmt) = self.diff_to_statement_with_context(d, diff) {
                sqls.push(self.statement_to_sql(stmt));
            }
        }

        // 5. Process Sequence creations
        for d in diff.iter().filter(|d| d.kind == EntityKind::Sequence) {
            if let Some(stmt) = self.diff_to_statement_with_context(d, diff) {
                sqls.push(self.statement_to_sql(stmt));
            }
        }

        // 6. Process Table creations (Rich tables)
        for table_key in &created_tables {
            let table_diff = diff
                .iter()
                .find(|d| d.name == *table_key && d.kind == EntityKind::Table)
                .unwrap();
            if let Some(PostgresEntity::Table(table)) = &table_diff.right {
                let rich_table = self.build_rich_table(table, diff);
                let stmt = JsonStatement::CreateTable { table: rich_table };
                sqls.push(self.statement_to_sql(stmt));
            }
        }

        // 7. Process other entities
        for d in diff {
            // Skip if handled above
            if d.kind == EntityKind::Schema
                || d.kind == EntityKind::Enum
                || d.kind == EntityKind::Sequence
            {
                continue;
            }

            // Skip table create (handled)
            if d.kind == EntityKind::Table && d.diff_type == DiffType::Create {
                continue;
            }

            // For creates of sub-entities, check if table was created/dropped
            if d.diff_type == DiffType::Create
                && let Some(parent_table) = self.get_parent_table_key(d)
                && created_tables.contains(&parent_table)
            {
                continue; // Handled in CreateTable
            }
            if d.diff_type == DiffType::Drop
                && let Some(parent_table) = self.get_parent_table_key(d)
                && dropped_tables.contains(&parent_table)
            {
                continue; // Handled in DropTable (implied CASCADE usually or handled by dropping table)
            }

            // Process individual statements with full diff context for PK lookups
            if let Some(stmt) = self.diff_to_statement_with_context(d, diff) {
                sqls.push(self.statement_to_sql(stmt));
            }
        }

        sqls
    }

    fn get_parent_table_key(&self, d: &EntityDiff) -> Option<String> {
        // Extract schema.name for table from entity
        // Uses the conventions from collection.rs keys
        match d.kind {
            EntityKind::Column | EntityKind::Policy => {
                // key: schema.table.name
                let parts: Vec<&str> = d.name.split('.').collect();
                if parts.len() >= 3 {
                    Some(format!("{}.{}", parts[0], parts[1]))
                } else {
                    None
                }
            }
            EntityKind::Index
            | EntityKind::ForeignKey
            | EntityKind::PrimaryKey
            | EntityKind::UniqueConstraint
            | EntityKind::CheckConstraint => {
                // key: schema.name (constraint/index name).
                // Need the entity itself to know the table.
                let entity = d.right.as_ref().or(d.left.as_ref())?;
                match entity {
                    PostgresEntity::Index(i) => Some(format!("{}.{}", i.schema, i.table)),
                    PostgresEntity::ForeignKey(f) => Some(format!("{}.{}", f.schema, f.table)),
                    PostgresEntity::PrimaryKey(p) => Some(format!("{}.{}", p.schema, p.table)),
                    PostgresEntity::UniqueConstraint(u) => {
                        Some(format!("{}.{}", u.schema, u.table))
                    }
                    PostgresEntity::CheckConstraint(c) => Some(format!("{}.{}", c.schema, c.table)),
                    _ => None,
                }
            }
            _ => None,
        }
    }

    fn build_rich_table(&self, table: &Table, diff: &[EntityDiff]) -> RichTable {
        let table_key = format!("{}.{}", table.schema, table.name);

        let columns = self.extract_created_entities(diff, EntityKind::Column, &table_key, |e| {
            if let PostgresEntity::Column(c) = e {
                Some(c.clone())
            } else {
                None
            }
        });

        let indexes = self.extract_created_entities(diff, EntityKind::Index, &table_key, |e| {
            if let PostgresEntity::Index(i) = e {
                Some(i.clone())
            } else {
                None
            }
        });

        let foreign_keys =
            self.extract_created_entities(diff, EntityKind::ForeignKey, &table_key, |e| {
                if let PostgresEntity::ForeignKey(f) = e {
                    Some(f.clone())
                } else {
                    None
                }
            });

        let uniques =
            self.extract_created_entities(diff, EntityKind::UniqueConstraint, &table_key, |e| {
                if let PostgresEntity::UniqueConstraint(u) = e {
                    Some(u.clone())
                } else {
                    None
                }
            });

        let checks =
            self.extract_created_entities(diff, EntityKind::CheckConstraint, &table_key, |e| {
                if let PostgresEntity::CheckConstraint(c) = e {
                    Some(c.clone())
                } else {
                    None
                }
            });

        let policies = self.extract_created_entities(diff, EntityKind::Policy, &table_key, |e| {
            if let PostgresEntity::Policy(p) = e {
                Some(p.clone())
            } else {
                None
            }
        });

        let pk_list =
            self.extract_created_entities(diff, EntityKind::PrimaryKey, &table_key, |e| {
                if let PostgresEntity::PrimaryKey(p) = e {
                    Some(p.clone())
                } else {
                    None
                }
            });
        let pk = pk_list.into_iter().next();

        RichTable {
            name: table.name.to_string(),
            schema: table.schema.to_string(),
            is_rls_enabled: table.is_rls_enabled,
            columns,
            indexes,
            foreign_keys,
            pk,
            uniques,
            checks,
            policies,
        }
    }

    fn extract_created_entities<T, F>(
        &self,
        diff: &[EntityDiff],
        kind: EntityKind,
        table_key: &str,
        extractor: F,
    ) -> Vec<T>
    where
        F: Fn(&PostgresEntity) -> Option<T>,
    {
        diff.iter()
            .filter(|d| d.diff_type == DiffType::Create && d.kind == kind)
            .filter_map(|d| {
                if let Some(parent) = self.get_parent_table_key(d)
                    && parent == table_key
                {
                    return d.right.as_ref().and_then(&extractor);
                }
                None
            })
            .collect()
    }

    /// Convert a single diff entry to a JSON statement, with access to the full diff
    /// for cross-entity lookups (e.g., determining if a column is part of a PK).
    fn diff_to_statement_with_context(
        &self,
        d: &EntityDiff,
        all_diffs: &[EntityDiff],
    ) -> Option<JsonStatement> {
        match d.diff_type {
            DiffType::Create => match d.right.as_ref()? {
                PostgresEntity::Schema(s) => Some(JsonStatement::CreateSchema {
                    name: s.name.to_string(),
                }),
                PostgresEntity::Enum(e) => Some(JsonStatement::CreateEnum { enum_: e.clone() }),
                PostgresEntity::Sequence(s) => Some(JsonStatement::CreateSequence {
                    sequence: s.clone(),
                }),
                PostgresEntity::Role(r) => Some(JsonStatement::CreateRole { role: r.clone() }),
                PostgresEntity::View(v) => Some(JsonStatement::CreateView { view: v.clone() }),
                PostgresEntity::Column(c) => {
                    // Check if this column is part of a newly created PK
                    let (is_pk, is_composite_pk) = self.check_column_pk_status(c, all_diffs);
                    Some(JsonStatement::AddColumn {
                        column: c.clone(),
                        is_pk,
                        is_composite_pk,
                    })
                }
                PostgresEntity::Index(i) => Some(JsonStatement::CreateIndex { index: i.clone() }),
                PostgresEntity::ForeignKey(f) => Some(JsonStatement::CreateFk { fk: f.clone() }),
                PostgresEntity::PrimaryKey(p) => Some(JsonStatement::AddPk { pk: p.clone() }),
                PostgresEntity::UniqueConstraint(u) => {
                    Some(JsonStatement::AddUnique { unique: u.clone() })
                }
                PostgresEntity::CheckConstraint(c) => {
                    Some(JsonStatement::AddCheck { check: c.clone() })
                }
                PostgresEntity::Policy(p) => {
                    Some(JsonStatement::CreatePolicy { policy: p.clone() })
                }
                PostgresEntity::Table(_) => None, // Handled separately in CreateTable
                PostgresEntity::Privilege(_) => None, // Privileges not yet tracked
            },
            DiffType::Drop => match d.left.as_ref()? {
                PostgresEntity::Schema(s) => Some(JsonStatement::DropSchema {
                    name: s.name.to_string(),
                }),
                PostgresEntity::Enum(e) => Some(JsonStatement::DropEnum { enum_: e.clone() }),
                PostgresEntity::Sequence(s) => Some(JsonStatement::DropSequence {
                    sequence: s.clone(),
                }),
                PostgresEntity::Role(r) => Some(JsonStatement::DropRole { role: r.clone() }),
                PostgresEntity::View(v) => Some(JsonStatement::DropView { view: v.clone() }),
                PostgresEntity::Table(t) => Some(JsonStatement::DropTable {
                    table: t.clone(),
                    table_key: format!("{}.{}", t.schema, t.name),
                }),
                PostgresEntity::Column(c) => Some(JsonStatement::DropColumn { column: c.clone() }),
                PostgresEntity::Index(i) => Some(JsonStatement::DropIndex { index: i.clone() }),
                PostgresEntity::ForeignKey(f) => Some(JsonStatement::DropFk { fk: f.clone() }),
                PostgresEntity::PrimaryKey(p) => Some(JsonStatement::DropPk { pk: p.clone() }),
                PostgresEntity::UniqueConstraint(u) => {
                    Some(JsonStatement::DropUnique { unique: u.clone() })
                }
                PostgresEntity::CheckConstraint(c) => {
                    Some(JsonStatement::DropCheck { check: c.clone() })
                }
                PostgresEntity::Policy(p) => Some(JsonStatement::DropPolicy { policy: p.clone() }),
                PostgresEntity::Privilege(_) => None, // Privileges not yet tracked
            },
            DiffType::Alter => {
                match (d.left.as_ref(), d.right.as_ref()) {
                    (Some(PostgresEntity::Enum(old)), Some(PostgresEntity::Enum(new))) => {
                        // Find new values added to the enum
                        let mut diffs = Vec::new();
                        for val in new.values.iter() {
                            if !old.values.iter().any(|v| v == val) {
                                diffs.push(EnumDiff {
                                    r#type: "added".to_string(),
                                    value: val.to_string(),
                                    before_value: None,
                                });
                            }
                        }
                        if !diffs.is_empty() {
                            Some(JsonStatement::AlterEnum {
                                from: old.clone(),
                                to: new.clone(),
                                diff: diffs,
                            })
                        } else {
                            None
                        }
                    }
                    (Some(PostgresEntity::Column(old)), Some(PostgresEntity::Column(new))) => {
                        // Build a granular diff structure for column alterations
                        let diff = self.build_column_diff(old, new);
                        let was_enum = old.type_schema.is_some();
                        let is_enum = new.type_schema.is_some();

                        Some(JsonStatement::AlterColumn {
                            to: new.clone(),
                            was_enum,
                            is_enum,
                            diff,
                        })
                    }
                    _ => None,
                }
            }
        }
    }

    /// Check if a column is part of a newly created primary key.
    /// Returns (is_pk, is_composite_pk):
    /// - is_pk: true if this is a single-column PK with default naming
    /// - is_composite_pk: true if this column is part of a multi-column PK
    fn check_column_pk_status(&self, col: &Column, all_diffs: &[EntityDiff]) -> (bool, bool) {
        // Look for created PKs for the same table
        let table_key = format!("{}.{}", col.schema, col.table);

        for d in all_diffs {
            if d.diff_type == DiffType::Create
                && d.kind == EntityKind::PrimaryKey
                && let Some(PostgresEntity::PrimaryKey(pk)) = &d.right
            {
                let pk_table_key = format!("{}.{}", pk.schema, pk.table);
                if pk_table_key == table_key && pk.columns.contains(&col.name) {
                    let is_composite = pk.columns.len() > 1;
                    // is_pk is true only for single-column PKs with default naming
                    let default_pk_name = format!("{}_pkey", col.table);
                    let is_single_pk = pk.columns.len() == 1 && pk.name == default_pk_name;
                    return (is_single_pk, is_composite);
                }
            }
        }

        (false, false)
    }

    /// Build a granular diff structure for column alterations.
    /// Tracks changes to type, default, notNull, generated, and identity.
    fn build_column_diff(&self, old: &Column, new: &Column) -> HashMap<String, serde_json::Value> {
        let mut diff = HashMap::new();

        // Type change
        if old.sql_type != new.sql_type || old.type_schema != new.type_schema {
            let mut type_diff = serde_json::Map::new();
            type_diff.insert("from".to_string(), serde_json::json!(old.sql_type));
            type_diff.insert("to".to_string(), serde_json::json!(new.sql_type));
            diff.insert("type".to_string(), serde_json::Value::Object(type_diff));

            if old.type_schema != new.type_schema {
                let mut ts_diff = serde_json::Map::new();
                ts_diff.insert("from".to_string(), serde_json::json!(old.type_schema));
                ts_diff.insert("to".to_string(), serde_json::json!(new.type_schema));
                diff.insert("typeSchema".to_string(), serde_json::Value::Object(ts_diff));
            }
        }

        // Default change
        if old.default != new.default {
            let mut default_diff = serde_json::Map::new();
            default_diff.insert("from".to_string(), serde_json::json!(old.default));
            default_diff.insert("to".to_string(), serde_json::json!(new.default));
            diff.insert(
                "default".to_string(),
                serde_json::Value::Object(default_diff),
            );
        }

        // NOT NULL change
        if old.not_null != new.not_null {
            let mut nn_diff = serde_json::Map::new();
            nn_diff.insert("from".to_string(), serde_json::json!(old.not_null));
            nn_diff.insert("to".to_string(), serde_json::json!(new.not_null));
            diff.insert("notNull".to_string(), serde_json::Value::Object(nn_diff));
        }

        // Generated column change
        if old.generated != new.generated {
            let mut gen_diff = serde_json::Map::new();
            gen_diff.insert("from".to_string(), serde_json::json!(old.generated));
            gen_diff.insert("to".to_string(), serde_json::json!(new.generated));
            diff.insert("generated".to_string(), serde_json::Value::Object(gen_diff));
        }

        // Identity change
        if old.identity != new.identity {
            let mut id_diff = serde_json::Map::new();
            id_diff.insert("from".to_string(), serde_json::json!(old.identity));
            id_diff.insert("to".to_string(), serde_json::json!(new.identity));
            diff.insert("identity".to_string(), serde_json::Value::Object(id_diff));
        }

        diff
    }

    fn statement_to_sql(&self, stmt: JsonStatement) -> String {
        match stmt {
            JsonStatement::CreateSchema { name } => format!("CREATE SCHEMA \"{}\";", name),
            JsonStatement::DropSchema { name } => format!("DROP SCHEMA \"{}\";", name),
            JsonStatement::CreateEnum { enum_: e } => {
                let schema_prefix = if e.schema != "public" {
                    format!("\"{}\".", e.schema)
                } else {
                    String::new()
                };
                let values = e
                    .values
                    .iter()
                    .map(|v| format!("'{}'", v))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!(
                    "CREATE TYPE {}\"{}\" AS ENUM ({});",
                    schema_prefix, e.name, values
                )
            }
            JsonStatement::DropEnum { enum_: e } => {
                let schema_prefix = if e.schema != "public" {
                    format!("\"{}\".", e.schema)
                } else {
                    String::new()
                };
                format!("DROP TYPE {}\"{}\";", schema_prefix, e.name)
            }
            JsonStatement::AlterEnum { from: _, to, diff } => {
                let schema_prefix = if to.schema != "public" {
                    format!("\"{}\".", to.schema)
                } else {
                    String::new()
                };
                let mut sqls = Vec::new();
                for d in diff {
                    sqls.push(format!(
                        "ALTER TYPE {}\"{}\" ADD VALUE '{}';",
                        schema_prefix, to.name, d.value
                    ));
                }
                sqls.join("\n")
            }
            JsonStatement::CreateSequence { sequence: s } => {
                let schema_prefix = if s.schema != "public" {
                    format!("\"{}\".", s.schema)
                } else {
                    String::new()
                };
                let mut sql = format!("CREATE SEQUENCE {}\"{}\"", schema_prefix, s.name);
                if let Some(ref inc) = s.increment_by {
                    sql.push_str(&format!(" INCREMENT BY {}", inc));
                }
                if let Some(ref min) = s.min_value {
                    sql.push_str(&format!(" MINVALUE {}", min));
                }
                if let Some(ref max) = s.max_value {
                    sql.push_str(&format!(" MAXVALUE {}", max));
                }
                if let Some(ref start) = s.start_with {
                    sql.push_str(&format!(" START WITH {}", start));
                }
                if let Some(cache) = s.cache_size {
                    sql.push_str(&format!(" CACHE {}", cache));
                }
                if s.cycle.unwrap_or(false) {
                    sql.push_str(" CYCLE");
                }
                sql.push(';');
                sql
            }
            JsonStatement::DropSequence { sequence: s } => {
                let schema_prefix = if s.schema != "public" {
                    format!("\"{}\".", s.schema)
                } else {
                    String::new()
                };
                format!("DROP SEQUENCE {}\"{}\";", schema_prefix, s.name)
            }
            JsonStatement::CreateTable { table } => {
                let schema_prefix = if table.schema != "public" {
                    format!("\"{}\".", table.schema)
                } else {
                    String::new()
                };
                let mut sql = format!("CREATE TABLE {}\"{}\" (\n", schema_prefix, table.name);
                let mut lines = Vec::new();

                for col in &table.columns {
                    lines.push(format!("\t{}", self.column_def(col)));
                }

                if let Some(pk) = &table.pk {
                    let cols = pk
                        .columns
                        .iter()
                        .map(|c| format!("\"{}\"", c))
                        .collect::<Vec<_>>()
                        .join(", ");
                    if pk.name_explicit {
                        lines.push(format!(
                            "\tCONSTRAINT \"{}\" PRIMARY KEY({})",
                            pk.name, cols
                        ));
                    } else {
                        lines.push(format!("\tPRIMARY KEY({})", cols));
                    }
                }

                for fk in &table.foreign_keys {
                    lines.push(format!("\t{}", self.fk_def(fk)));
                }

                for u in &table.uniques {
                    let cols = u
                        .columns
                        .iter()
                        .map(|c| format!("\"{}\"", c))
                        .collect::<Vec<_>>()
                        .join(", ");
                    lines.push(format!("\tCONSTRAINT \"{}\" UNIQUE({})", u.name, cols));
                }

                for c in &table.checks {
                    lines.push(format!("\tCONSTRAINT \"{}\" CHECK ({})", c.name, c.value));
                }

                sql.push_str(&lines.join(",\n"));
                sql.push_str("\n);");

                // Add Indexes separate? No, standard is create table then indexes usually in Drizzle,
                // but JsonStatement::CreateTable doesn't strictly imply inline indexes.
                // But Drizzle-Kit usually separates indexes.

                sql
            }
            JsonStatement::DropTable { table, .. } => {
                let schema_prefix = if table.schema != "public" {
                    format!("\"{}\".", table.schema)
                } else {
                    String::new()
                };
                format!("DROP TABLE {}\"{}\";", schema_prefix, table.name)
            }
            JsonStatement::AddColumn { column, is_pk, .. } => {
                let schema_prefix = if column.schema != "public" {
                    format!("\"{}\".", column.schema)
                } else {
                    String::new()
                };
                let pk_clause = if is_pk { " PRIMARY KEY" } else { "" };
                format!(
                    "ALTER TABLE {}\"{}\" ADD COLUMN {}{};",
                    schema_prefix,
                    column.table,
                    self.column_def(&column),
                    pk_clause
                )
            }
            JsonStatement::DropColumn { column } => {
                let schema_prefix = if column.schema != "public" {
                    format!("\"{}\".", column.schema)
                } else {
                    String::new()
                };
                format!(
                    "ALTER TABLE {}\"{}\" DROP COLUMN \"{}\";",
                    schema_prefix, column.table, column.name
                )
            }
            JsonStatement::CreateIndex { index } => {
                let unique = if index.is_unique { "UNIQUE " } else { "" };
                let concurrently = if index.concurrently {
                    "CONCURRENTLY "
                } else {
                    ""
                };
                let schema_prefix = if index.schema != "public" {
                    format!("\"{}\".", index.schema)
                } else {
                    String::new()
                };

                let cols = index
                    .columns
                    .iter()
                    .map(|c| {
                        let val = if c.is_expression {
                            c.value.to_string()
                        } else {
                            format!("\"{}\"", c.value)
                        };
                        let order = if c.asc { "" } else { " DESC" };
                        let nulls = if c.nulls_first {
                            " NULLS FIRST"
                        } else {
                            " NULLS LAST"
                        }; // Default varies but being explicit safe?
                        // Actually Drizzle defaults: ASC NULLS LAST.
                        format!("{}{}{}", val, order, nulls)
                    })
                    .collect::<Vec<_>>()
                    .join(", ");

                format!(
                    "CREATE {}{}INDEX \"{}\" ON {}\"{}\" USING {} ({});",
                    unique,
                    concurrently,
                    index.name,
                    schema_prefix,
                    index.table,
                    index.method.as_deref().unwrap_or("btree"),
                    cols
                )
            }
            JsonStatement::DropIndex { index } => {
                let schema_prefix = if index.schema != "public" {
                    format!("\"{}\".", index.schema)
                } else {
                    String::new()
                };
                format!("DROP INDEX {}\"{}\";", schema_prefix, index.name)
            }
            JsonStatement::CreateFk { fk } => {
                let schema_prefix = if fk.schema != "public" {
                    format!("\"{}\".", fk.schema)
                } else {
                    String::new()
                };
                format!(
                    "ALTER TABLE {}\"{}\" ADD {};",
                    schema_prefix,
                    fk.table,
                    self.fk_def(&fk)
                )
            }
            JsonStatement::DropFk { fk } => {
                let schema_prefix = if fk.schema != "public" {
                    format!("\"{}\".", fk.schema)
                } else {
                    String::new()
                };
                format!(
                    "ALTER TABLE {}\"{}\" DROP CONSTRAINT \"{}\";",
                    schema_prefix, fk.table, fk.name
                )
            }
            JsonStatement::CreateView { view } => {
                let schema_prefix = if view.schema != "public" {
                    format!("\"{}\".", view.schema)
                } else {
                    String::new()
                };
                let mat = if view.materialized {
                    "MATERIALIZED "
                } else {
                    ""
                };
                let def = view.definition.as_deref().unwrap_or("");
                format!(
                    "CREATE {}VIEW {}\"{}\" AS {};",
                    mat, schema_prefix, view.name, def
                )
            }
            JsonStatement::DropView { view } => {
                let schema_prefix = if view.schema != "public" {
                    format!("\"{}\".", view.schema)
                } else {
                    String::new()
                };
                let mat = if view.materialized {
                    "MATERIALIZED "
                } else {
                    ""
                };
                format!("DROP {}VIEW {}\"{}\";", mat, schema_prefix, view.name)
            }
            // Implement others as needed
            _ => format!("-- Unimplemented statement: {:?}", stmt),
        }
    }

    fn column_def(&self, col: &Column) -> String {
        let mut def = format!("\"{}\" {}", col.name, col.sql_type);
        if col.not_null {
            def.push_str(" NOT NULL");
        }
        if let Some(default) = &col.default {
            def.push_str(&format!(" DEFAULT {}", default));
        }

        if let Some(generated_col) = &col.generated {
            def.push_str(&format!(
                " GENERATED ALWAYS AS ({}) STORED",
                generated_col.expression
            ));
        }

        if let Some(id) = &col.identity {
            use super::ddl::IdentityType;
            let type_str = match id.type_ {
                IdentityType::Always => "ALWAYS",
                IdentityType::ByDefault => "BY DEFAULT",
            };
            def.push_str(&format!(" GENERATED {} AS IDENTITY", type_str));
        }

        def
    }

    fn fk_def(&self, fk: &ForeignKey) -> String {
        let cols_from = fk
            .columns
            .iter()
            .map(|c| format!("\"{}\"", c))
            .collect::<Vec<_>>()
            .join(", ");
        let cols_to = fk
            .columns_to
            .iter()
            .map(|c| format!("\"{}\"", c))
            .collect::<Vec<_>>()
            .join(", ");
        let schema_to_prefix = if fk.schema_to != "public" {
            format!("\"{}\".", fk.schema_to)
        } else {
            String::new()
        };

        let mut def = format!(
            "CONSTRAINT \"{}\" FOREIGN KEY ({}) REFERENCES {}\"{}\"({})",
            fk.name, cols_from, schema_to_prefix, fk.table_to, cols_to
        );

        if let Some(on_delete) = &fk.on_delete {
            def.push_str(&format!(" ON DELETE {}", on_delete));
        }
        if let Some(on_update) = &fk.on_update {
            def.push_str(&format!(" ON UPDATE {}", on_update));
        }
        def
    }
}

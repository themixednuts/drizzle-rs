//! `PostgreSQL` SQL generation from schema metadata

use super::collection::{DiffType, EntityDiff, PostgresDDL};
use super::ddl::{
    CheckConstraint, Column, Enum, ForeignKey, Index, Policy, PostgresEntity, PrimaryKey, Role,
    Schema, Sequence, Table, TableSql, UniqueConstraint, View,
};
use crate::traits::EntityKind;
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::fmt::Write;

pub const BREAKPOINT: &str = "--> statement-breakpoint";

#[derive(Debug, Clone)]
struct CreateTableOrder {
    ordered: Vec<String>,
    cycle_tables: HashSet<String>,
}

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
        column: Box<Column>,
        #[serde(rename = "isPK")]
        is_pk: bool,
        #[serde(rename = "isCompositePK")]
        is_composite_pk: bool,
    },
    DropColumn {
        column: Box<Column>,
    },
    AlterColumn {
        to: Box<Column>,
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
    /// Alter a view by dropping and recreating (`PostgreSQL` doesn't support ALTER VIEW for definition changes)
    AlterView {
        old_view: Box<View>,
        new_view: Box<View>,
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
    AlterTable {
        old_table: Table,
        new_table: Table,
    },
    RecreateFk {
        old_fk: ForeignKey,
        new_fk: ForeignKey,
    },
    RecreateUnique {
        old_unique: UniqueConstraint,
        new_unique: UniqueConstraint,
    },
    /// Recreate column by dropping and re-adding (for generated columns, type changes, etc.)
    RecreateColumn {
        old_column: Box<Column>,
        new_column: Box<Column>,
    },
    /// Recreate an index by dropping and re-creating (`PostgreSQL` has no
    /// general ALTER INDEX for definition changes).
    RecreateIndex {
        old_index: Box<Index>,
        new_index: Box<Index>,
    },
    /// Recreate a primary key: DROP CONSTRAINT + ADD CONSTRAINT.
    RecreatePk {
        old_pk: PrimaryKey,
        new_pk: PrimaryKey,
    },
    /// Recreate a check constraint: DROP CONSTRAINT + ADD CONSTRAINT.
    RecreateCheck {
        old_check: CheckConstraint,
        new_check: CheckConstraint,
    },
    /// Recreate a policy: DROP POLICY + CREATE POLICY (drizzle-kit style).
    RecreatePolicy {
        old_policy: Box<Policy>,
        new_policy: Box<Policy>,
    },
    /// ALTER SEQUENCE with the changed options.
    AlterSequence {
        old_sequence: Sequence,
        new_sequence: Sequence,
    },
    /// ALTER ROLE with the changed flags.
    AlterRole {
        old_role: Role,
        new_role: Role,
    },
    /// Recreate an enum type whose values were removed or reordered
    /// (drizzle-kit flow: alter dependent columns to text, drop + recreate
    /// the type, alter the columns back with `USING ::text::type`, restore
    /// defaults).
    RecreateEnum {
        old_enum: Enum,
        new_enum: Enum,
        /// Columns (from the current schema) whose type is this enum.
        columns: Vec<Column>,
    },
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
    pub comment: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_rls_enabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_unlogged: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_temporary: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inherits: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tablespace: Option<String>,
}

#[derive(Default)]
struct CreatedTableEntities<'a> {
    columns: Vec<&'a Column>,
    indexes: Vec<&'a Index>,
    foreign_keys: Vec<&'a ForeignKey>,
    primary_keys: Vec<&'a PrimaryKey>,
    unique_constraints: Vec<&'a UniqueConstraint>,
    check_constraints: Vec<&'a CheckConstraint>,
    policies: Vec<&'a Policy>,
}

struct DiffIndex<'a> {
    table_diffs: HashMap<&'a str, &'a EntityDiff>,
    created_by_table: HashMap<String, CreatedTableEntities<'a>>,
}

impl<'a> DiffIndex<'a> {
    fn new(diffs: &'a [EntityDiff]) -> Self {
        let mut table_diffs = HashMap::new();
        let mut created_by_table = HashMap::<String, CreatedTableEntities<'a>>::new();

        for diff in diffs {
            if diff.kind == EntityKind::Table {
                table_diffs.insert(diff.name.as_str(), diff);
            }
            if diff.diff_type != DiffType::Create {
                continue;
            }

            let Some(entity) = diff.right.as_ref() else {
                continue;
            };
            let Some(table_key) = Generator::get_parent_table_key(diff) else {
                continue;
            };
            let entries = created_by_table.entry(table_key).or_default();
            match entity {
                PostgresEntity::Column(value) => entries.columns.push(value),
                PostgresEntity::Index(value) => entries.indexes.push(value),
                PostgresEntity::ForeignKey(value) => entries.foreign_keys.push(value),
                PostgresEntity::PrimaryKey(value) => entries.primary_keys.push(value),
                PostgresEntity::UniqueConstraint(value) => {
                    entries.unique_constraints.push(value);
                }
                PostgresEntity::CheckConstraint(value) => entries.check_constraints.push(value),
                PostgresEntity::Policy(value) => entries.policies.push(value),
                _ => {}
            }
        }

        Self {
            table_diffs,
            created_by_table,
        }
    }

    fn table_diff(&self, table_key: &str) -> Option<&'a EntityDiff> {
        self.table_diffs.get(table_key).copied()
    }
}

fn rich_table_to_table(table: &RichTable) -> Table {
    Table {
        schema: table.schema.clone().into(),
        name: table.name.clone().into(),
        is_unlogged: table.is_unlogged,
        is_temporary: table.is_temporary,
        inherits: table.inherits.clone().map(Into::into),
        tablespace: table.tablespace.clone().map(Into::into),
        is_rls_enabled: table.is_rls_enabled,
        comment: table.comment.clone().map(Into::into),
    }
}

// =============================================================================
// Generator
// =============================================================================

pub struct Generator {
    pub breakpoints: bool,
}

impl Default for Generator {
    fn default() -> Self {
        Self::new()
    }
}

impl Generator {
    #[must_use]
    pub const fn new() -> Self {
        Self { breakpoints: true }
    }

    #[must_use]
    pub const fn with_breakpoints(mut self, breakpoints: bool) -> Self {
        self.breakpoints = breakpoints;
        self
    }

    /// Generate SQL statements from a set of entity diffs.
    ///
    /// # Panics
    ///
    /// Panics if a table listed in `created_tables` is not found in `diff`
    /// — this cannot happen in practice because `created_tables` is built
    /// from `diff` itself.
    #[must_use]
    pub fn generate(&self, diff: &[EntityDiff]) -> Vec<String> {
        self.generate_with_ddl(diff, None)
    }

    /// Generate SQL statements from a set of entity diffs, with access to the
    /// full *current* DDL for cross-entity lookups (e.g. finding the columns
    /// that depend on an enum being recreated).
    ///
    /// Statement ordering is dependency-phased:
    ///
    /// 1. creates of schemas → enums → sequences → roles,
    /// 2. drops of views,
    /// 3. drops of table sub-entities on surviving tables
    ///    (FKs/indexes/constraints/policies before columns),
    /// 4. table drops in reverse dependency order,
    /// 5. table creates in dependency order (with inlined sub-entities),
    /// 6. sub-entity creates on pre-existing tables (columns first),
    /// 7. alters (including `ALTER COLUMN ... USING` enum conversions),
    /// 8. enum and sequence drops (after alters so `USING` casts run before
    ///    `DROP TYPE`),
    /// 9. view creates,
    /// 10. role and schema drops.
    ///
    /// # Panics
    ///
    /// Panics if a table listed in `created_tables` is not found in `diff`
    /// — this cannot happen in practice because `created_tables` is built
    /// from `diff` itself.
    #[must_use]
    pub fn generate_with_ddl(
        &self,
        diff: &[EntityDiff],
        cur_ddl: Option<&PostgresDDL>,
    ) -> Vec<String> {
        let mut sqls = Vec::new();
        let diff_index = DiffIndex::new(diff);

        // Identify created/dropped tables to group and filter their components.
        let created_tables: Vec<String> = diff
            .iter()
            .filter(|d| d.diff_type == DiffType::Create && d.kind == EntityKind::Table)
            .map(|d| d.name.clone())
            .collect();
        let dropped_tables: Vec<String> = diff
            .iter()
            .filter(|d| d.diff_type == DiffType::Drop && d.kind == EntityKind::Table)
            .map(|d| d.name.clone())
            .collect();

        let push_diff = |sqls: &mut Vec<String>, d: &EntityDiff| {
            if let Some(stmt) = Self::diff_to_statement_with_context(d, &diff_index, cur_ddl) {
                sqls.extend(Self::statement_to_sqls(stmt));
            }
        };

        // Phase 1: creates of top-level entities in dependency order.
        for kind in [
            EntityKind::Schema,
            EntityKind::Enum,
            EntityKind::Sequence,
            EntityKind::Role,
        ] {
            for d in diff
                .iter()
                .filter(|d| d.kind == kind && d.diff_type == DiffType::Create)
            {
                push_diff(&mut sqls, d);
            }
        }

        // Phase 2: view drops (views depend on tables/columns/enums, so they
        // must go before any of those are dropped or altered).
        for d in diff
            .iter()
            .filter(|d| d.kind == EntityKind::View && d.diff_type == DiffType::Drop)
        {
            push_diff(&mut sqls, d);
        }

        // Phase 3: sub-entity drops on surviving tables. FK, index and
        // constraint drops must precede column drops — dropping a column
        // first would implicitly remove its dependent objects and make the
        // later DROP INDEX / DROP CONSTRAINT fail with "does not exist".
        for kind in [
            EntityKind::ForeignKey,
            EntityKind::Index,
            EntityKind::PrimaryKey,
            EntityKind::UniqueConstraint,
            EntityKind::CheckConstraint,
            EntityKind::Policy,
            EntityKind::Column,
        ] {
            for d in diff
                .iter()
                .filter(|d| d.kind == kind && d.diff_type == DiffType::Drop)
            {
                if let Some(parent_table) = Self::get_parent_table_key(d)
                    && dropped_tables.contains(&parent_table)
                {
                    continue; // handled by DROP TABLE
                }
                push_diff(&mut sqls, d);
            }
        }

        // Phase 4: table drops in reverse dependency order (referencing
        // tables drop before the tables they point at).
        let sorted_drops = topological_sort_tables_for_drop(&dropped_tables, diff);
        for table_key in &sorted_drops {
            if let Some(table_diff) = diff_index.table_diff(table_key)
                && table_diff.diff_type == DiffType::Drop
            {
                push_diff(&mut sqls, table_diff);
            }
        }

        // Phase 5: table creates (Rich tables) in dependency order.
        let sorted_creates = topological_sort_tables_for_create(&created_tables, diff);
        if sorted_creates.cycle_tables.is_empty() {
            for table_key in &sorted_creates.ordered {
                let table_diff = diff_index
                    .table_diff(table_key)
                    .expect("created table must have an indexed table diff");
                if let Some(PostgresEntity::Table(table)) = &table_diff.right {
                    let rich_table = Self::build_rich_table(table, &diff_index);
                    sqls.push(Self::create_table_sql(&rich_table));
                    Self::push_created_table_extras(&mut sqls, &rich_table);
                }
            }
        } else {
            let mut deferred_fks = Vec::new();
            let mut rich_tables = Vec::new();

            for table_key in &sorted_creates.ordered {
                let table_diff = diff_index
                    .table_diff(table_key)
                    .expect("created table must have an indexed table diff");
                if let Some(PostgresEntity::Table(table)) = &table_diff.right {
                    let mut rich_table = Self::build_rich_table(table, &diff_index);
                    let (inline_fks, cycle_fks): (Vec<_>, Vec<_>) = rich_table
                        .foreign_keys
                        .into_iter()
                        .partition(|fk| !Self::is_cycle_fk(fk, &sorted_creates.cycle_tables));
                    rich_table.foreign_keys = inline_fks;
                    deferred_fks.extend(cycle_fks);
                    sqls.push(Self::create_table_sql(&rich_table));
                    rich_tables.push(rich_table);
                }
            }

            for fk in &deferred_fks {
                sqls.push(Self::add_fk_sql(fk));
            }

            for rich_table in &rich_tables {
                Self::push_created_table_extras(&mut sqls, rich_table);
            }
        }

        // Phase 6: sub-entity creates on pre-existing tables, columns first so
        // constraints and indexes can reference them.
        for kind in [
            EntityKind::Column,
            EntityKind::PrimaryKey,
            EntityKind::UniqueConstraint,
            EntityKind::CheckConstraint,
            EntityKind::Index,
            EntityKind::ForeignKey,
            EntityKind::Policy,
        ] {
            for d in diff
                .iter()
                .filter(|d| d.kind == kind && d.diff_type == DiffType::Create)
            {
                if let Some(parent_table) = Self::get_parent_table_key(d)
                    && created_tables.contains(&parent_table)
                {
                    continue; // inlined in CREATE TABLE
                }
                push_diff(&mut sqls, d);
            }
        }

        // Phase 7: alters, in diff order (enum ADD VALUE alters precede
        // column alters, which precede table/view alters).
        for d in diff.iter().filter(|d| d.diff_type == DiffType::Alter) {
            push_diff(&mut sqls, d);
        }

        // Phase 8: enum and sequence drops. These must come after column
        // alters: converting a column away from an enum (`ALTER COLUMN ...
        // USING`) has to run before `DROP TYPE`, and a dropped sequence may
        // be referenced by a column default until the alter removes it.
        for kind in [EntityKind::Enum, EntityKind::Sequence] {
            for d in diff
                .iter()
                .filter(|d| d.kind == kind && d.diff_type == DiffType::Drop)
            {
                push_diff(&mut sqls, d);
            }
        }

        // Phase 9: view creates (after every table/column they may select from).
        for d in diff
            .iter()
            .filter(|d| d.kind == EntityKind::View && d.diff_type == DiffType::Create)
        {
            push_diff(&mut sqls, d);
        }

        // Phase 10: role drops (after the policies that referenced them) and
        // schema drops (after everything inside the schema is gone).
        for kind in [EntityKind::Role, EntityKind::Schema] {
            for d in diff
                .iter()
                .filter(|d| d.kind == kind && d.diff_type == DiffType::Drop)
            {
                push_diff(&mut sqls, d);
            }
        }

        sqls
    }

    fn get_parent_table_key(d: &EntityDiff) -> Option<String> {
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

    fn table_key(schema: &str, table: &str) -> String {
        format!("{schema}.{table}")
    }

    fn is_cycle_fk(fk: &ForeignKey, cycle_tables: &HashSet<String>) -> bool {
        cycle_tables.contains(&Self::table_key(&fk.schema, &fk.table))
            && cycle_tables.contains(&Self::table_key(&fk.schema_to, &fk.table_to))
    }

    fn push_created_table_extras(sqls: &mut Vec<String>, table: &RichTable) {
        sqls.extend(Self::created_table_comments_sql(table));

        for index in &table.indexes {
            sqls.push(Self::create_index_sql(index));
        }

        if table.is_rls_enabled.unwrap_or(false) {
            sqls.push(format!(
                "ALTER TABLE {} ENABLE ROW LEVEL SECURITY;",
                Self::qualified_name(&table.schema, &table.name)
            ));
        }

        for policy in &table.policies {
            sqls.push(Self::create_policy_sql(policy));
        }
    }

    fn build_rich_table(table: &Table, diff_index: &DiffIndex<'_>) -> RichTable {
        let table_key = format!("{}.{}", table.schema, table.name);
        let entities = diff_index.created_by_table.get(&table_key);
        let columns = entities
            .map(|entries| {
                entries
                    .columns
                    .iter()
                    .map(|value| (*value).clone())
                    .collect()
            })
            .unwrap_or_default();
        let indexes = entities
            .map(|entries| {
                entries
                    .indexes
                    .iter()
                    .map(|value| (*value).clone())
                    .collect()
            })
            .unwrap_or_default();
        let foreign_keys = entities
            .map(|entries| {
                entries
                    .foreign_keys
                    .iter()
                    .map(|value| (*value).clone())
                    .collect()
            })
            .unwrap_or_default();
        let uniques = entities
            .map(|entries| {
                entries
                    .unique_constraints
                    .iter()
                    .map(|value| (*value).clone())
                    .collect()
            })
            .unwrap_or_default();
        let checks = entities
            .map(|entries| {
                entries
                    .check_constraints
                    .iter()
                    .map(|value| (*value).clone())
                    .collect()
            })
            .unwrap_or_default();
        let policies = entities
            .map(|entries| {
                entries
                    .policies
                    .iter()
                    .map(|value| (*value).clone())
                    .collect()
            })
            .unwrap_or_default();
        let pk = entities
            .and_then(|entries| entries.primary_keys.first())
            .map(|value| (*value).clone());

        RichTable {
            name: table.name.to_string(),
            schema: table.schema.to_string(),
            is_rls_enabled: table.is_rls_enabled,
            is_unlogged: table.is_unlogged,
            is_temporary: table.is_temporary,
            inherits: table.inherits.as_ref().map(ToString::to_string),
            tablespace: table.tablespace.as_ref().map(ToString::to_string),
            columns,
            indexes,
            foreign_keys,
            pk,
            uniques,
            checks,
            policies,
            comment: table.comment.as_ref().map(ToString::to_string),
        }
    }

    /// Convert a single diff entry to a JSON statement, with access to the full diff
    /// for cross-entity lookups (e.g., determining if a column is part of a PK).
    fn diff_to_statement_with_context(
        d: &EntityDiff,
        diff_index: &DiffIndex<'_>,
        cur_ddl: Option<&PostgresDDL>,
    ) -> Option<JsonStatement> {
        match d.diff_type {
            DiffType::Create => Self::create_diff_to_statement(d.right.as_ref()?, diff_index),
            DiffType::Drop => Self::drop_diff_to_statement(d.left.as_ref()?),
            DiffType::Alter => {
                Self::alter_diff_to_statement(d.left.as_ref(), d.right.as_ref(), cur_ddl)
            }
        }
    }

    fn create_diff_to_statement(
        right: &PostgresEntity,
        diff_index: &DiffIndex<'_>,
    ) -> Option<JsonStatement> {
        match right {
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
                let (is_pk, is_composite_pk) = Self::check_column_pk_status(c, diff_index);
                Some(JsonStatement::AddColumn {
                    column: Box::new(c.clone()),
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
            PostgresEntity::Policy(p) => Some(JsonStatement::CreatePolicy { policy: p.clone() }),
            // Handled separately in CreateTable; privileges not yet tracked
            PostgresEntity::Table(_) | PostgresEntity::Privilege(_) => None,
        }
    }

    fn drop_diff_to_statement(left: &PostgresEntity) -> Option<JsonStatement> {
        match left {
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
            PostgresEntity::Column(c) => Some(JsonStatement::DropColumn {
                column: Box::new(c.clone()),
            }),
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
        }
    }

    /// Check whether `old` is a subsequence of `new` — i.e. every old value
    /// still exists and their relative order is preserved, so the change is
    /// expressible with `ALTER TYPE ... ADD VALUE`.
    fn enum_values_are_pure_additions(
        old: &[std::borrow::Cow<'static, str>],
        new: &[std::borrow::Cow<'static, str>],
    ) -> bool {
        let mut new_iter = new.iter();
        old.iter()
            .all(|old_value| new_iter.any(|new_value| new_value == old_value))
    }

    /// Collect the columns of the current schema whose type is the given enum.
    fn enum_dependent_columns(ddl: &PostgresDDL, enum_: &Enum) -> Vec<Column> {
        ddl.columns
            .list()
            .iter()
            .filter(|column| {
                column.sql_type.as_ref() == enum_.name.as_ref()
                    && column.type_schema.as_deref().unwrap_or("public") == enum_.schema.as_ref()
            })
            .map(|column| (*column).clone())
            .collect()
    }

    fn alter_diff_to_statement(
        left: Option<&PostgresEntity>,
        right: Option<&PostgresEntity>,
        cur_ddl: Option<&PostgresDDL>,
    ) -> Option<JsonStatement> {
        match (left, right) {
            (Some(PostgresEntity::Enum(old)), Some(PostgresEntity::Enum(new))) => {
                if old.values == new.values {
                    return None;
                }
                if !Self::enum_values_are_pure_additions(&old.values, &new.values) {
                    // Removed or reordered values: PostgreSQL cannot express
                    // this with ALTER TYPE, so recreate the type
                    // (drizzle-kit parity).
                    let columns = cur_ddl
                        .map(|ddl| Self::enum_dependent_columns(ddl, new))
                        .unwrap_or_default();
                    return Some(JsonStatement::RecreateEnum {
                        old_enum: old.clone(),
                        new_enum: new.clone(),
                        columns,
                    });
                }
                let mut diffs = Vec::new();
                for (idx, val) in new.values.iter().enumerate() {
                    if !old.values.iter().any(|v| v == val) {
                        let before_value = new
                            .values
                            .iter()
                            .skip(idx + 1)
                            .find(|candidate| {
                                old.values.iter().any(|old_value| old_value == *candidate)
                            })
                            .map(ToString::to_string);
                        diffs.push(EnumDiff {
                            r#type: "added".to_string(),
                            value: val.to_string(),
                            before_value,
                        });
                    }
                }
                if diffs.is_empty() {
                    None
                } else {
                    Some(JsonStatement::AlterEnum {
                        from: old.clone(),
                        to: new.clone(),
                        diff: diffs,
                    })
                }
            }
            (Some(PostgresEntity::Column(old)), Some(PostgresEntity::Column(new))) => {
                // PostgreSQL doesn't support ALTER COLUMN ... ADD GENERATED AS
                let needs_recreate = old.generated.is_none() && new.generated.is_some();
                if needs_recreate {
                    Some(JsonStatement::RecreateColumn {
                        old_column: Box::new(old.clone()),
                        new_column: Box::new(new.clone()),
                    })
                } else {
                    let diff = Self::build_column_diff(old, new);
                    let was_enum = old.type_schema.is_some();
                    let is_enum = new.type_schema.is_some();
                    Some(JsonStatement::AlterColumn {
                        to: Box::new(new.clone()),
                        was_enum,
                        is_enum,
                        diff,
                    })
                }
            }
            (Some(PostgresEntity::Table(old)), Some(PostgresEntity::Table(new))) => {
                if Self::alter_table_sql(old, new).is_some() {
                    Some(JsonStatement::AlterTable {
                        old_table: old.clone(),
                        new_table: new.clone(),
                    })
                } else {
                    None
                }
            }
            (Some(PostgresEntity::ForeignKey(old)), Some(PostgresEntity::ForeignKey(new))) => {
                Some(JsonStatement::RecreateFk {
                    old_fk: old.clone(),
                    new_fk: new.clone(),
                })
            }
            (
                Some(PostgresEntity::UniqueConstraint(old)),
                Some(PostgresEntity::UniqueConstraint(new)),
            ) => Some(JsonStatement::RecreateUnique {
                old_unique: old.clone(),
                new_unique: new.clone(),
            }),
            // PostgreSQL doesn't support ALTER VIEW for definition changes,
            // so we drop and recreate the view.
            (Some(PostgresEntity::View(old)), Some(PostgresEntity::View(new))) => {
                if old.is_existing || new.is_existing {
                    None
                } else {
                    Some(JsonStatement::AlterView {
                        old_view: Box::new(old.clone()),
                        new_view: Box::new(new.clone()),
                    })
                }
            }
            // Index definition changes require drop + recreate.
            (Some(PostgresEntity::Index(old)), Some(PostgresEntity::Index(new))) => {
                Some(JsonStatement::RecreateIndex {
                    old_index: Box::new(old.clone()),
                    new_index: Box::new(new.clone()),
                })
            }
            (Some(PostgresEntity::PrimaryKey(old)), Some(PostgresEntity::PrimaryKey(new))) => {
                Some(JsonStatement::RecreatePk {
                    old_pk: old.clone(),
                    new_pk: new.clone(),
                })
            }
            (
                Some(PostgresEntity::CheckConstraint(old)),
                Some(PostgresEntity::CheckConstraint(new)),
            ) => Some(JsonStatement::RecreateCheck {
                old_check: old.clone(),
                new_check: new.clone(),
            }),
            (Some(PostgresEntity::Policy(old)), Some(PostgresEntity::Policy(new))) => {
                Some(JsonStatement::RecreatePolicy {
                    old_policy: Box::new(old.clone()),
                    new_policy: Box::new(new.clone()),
                })
            }
            (Some(PostgresEntity::Sequence(old)), Some(PostgresEntity::Sequence(new))) => {
                if Self::alter_sequence_sql(old, new).is_some() {
                    Some(JsonStatement::AlterSequence {
                        old_sequence: old.clone(),
                        new_sequence: new.clone(),
                    })
                } else {
                    None
                }
            }
            (Some(PostgresEntity::Role(old)), Some(PostgresEntity::Role(new))) => {
                if Self::alter_role_sql(old, new).is_some() {
                    Some(JsonStatement::AlterRole {
                        old_role: old.clone(),
                        new_role: new.clone(),
                    })
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    /// Check if a column is part of a newly created primary key.
    /// Returns (`is_pk`, `is_composite_pk)`:
    /// - `is_pk`: true if this is a single-column PK with default naming
    ///   **and the parent table itself is created in this diff**. Columns
    ///   added to a pre-existing table never render an inline ` PRIMARY KEY`
    ///   — the PK arrives via a separate `ADD CONSTRAINT ..._pkey`
    ///   statement, and emitting both would create two constraints.
    /// - `is_composite_pk`: true if this column is part of a multi-column PK
    fn check_column_pk_status(col: &Column, diff_index: &DiffIndex<'_>) -> (bool, bool) {
        let table_key = format!("{}.{}", col.schema, col.table);
        let table_created_this_diff = diff_index
            .table_diff(&table_key)
            .is_some_and(|table_diff| table_diff.diff_type == DiffType::Create);

        if let Some(entries) = diff_index.created_by_table.get(&table_key) {
            for pk in &entries.primary_keys {
                if pk.columns.contains(&col.name) {
                    let is_composite = pk.columns.len() > 1;
                    let default_pk_name = format!("{}_pkey", col.table);
                    let is_single_pk = table_created_this_diff
                        && pk.columns.len() == 1
                        && pk.name == default_pk_name;
                    return (is_single_pk, is_composite);
                }
            }
        }

        (false, false)
    }

    /// Build a granular diff structure for column alterations.
    /// Tracks changes to type, default, notNull, generated, and identity.
    fn build_column_diff(old: &Column, new: &Column) -> HashMap<String, serde_json::Value> {
        let mut diff = HashMap::new();

        // Type change
        if old.sql_type != new.sql_type
            || old.type_schema != new.type_schema
            || old.dimensions != new.dimensions
        {
            let mut type_diff = serde_json::Map::new();
            type_diff.insert("from".to_string(), serde_json::json!(old.sql_type));
            type_diff.insert("to".to_string(), serde_json::json!(new.sql_type));
            type_diff.insert(
                "fromDimensions".to_string(),
                serde_json::json!(old.dimensions),
            );
            type_diff.insert(
                "toDimensions".to_string(),
                serde_json::json!(new.dimensions),
            );
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

        if old.comment != new.comment {
            let mut comment_diff = serde_json::Map::new();
            comment_diff.insert("from".to_string(), serde_json::json!(old.comment));
            comment_diff.insert("to".to_string(), serde_json::json!(new.comment));
            diff.insert(
                "comment".to_string(),
                serde_json::Value::Object(comment_diff),
            );
        }

        diff
    }

    fn schema_prefix(schema: &str) -> String {
        if schema == "public" {
            String::new()
        } else {
            format!("{}.", Self::quote_ident(schema))
        }
    }

    fn quote_ident(ident: &str) -> String {
        format!("\"{}\"", ident.replace('"', "\"\""))
    }

    fn quote_literal(value: &str) -> String {
        format!("'{}'", value.replace('\'', "''"))
    }

    fn qualified_name(schema: &str, name: &str) -> String {
        format!("{}{}", Self::schema_prefix(schema), Self::quote_ident(name))
    }

    fn comment_value_sql(comment: Option<&str>) -> String {
        comment.map_or_else(|| "NULL".to_string(), Self::quote_literal)
    }

    fn comment_on_table_sql(schema: &str, table: &str, comment: Option<&str>) -> String {
        format!(
            "COMMENT ON TABLE {} IS {};",
            Self::qualified_name(schema, table),
            Self::comment_value_sql(comment)
        )
    }

    fn comment_on_column_sql(
        schema: &str,
        table: &str,
        column: &str,
        comment: Option<&str>,
    ) -> String {
        format!(
            "COMMENT ON COLUMN {}.{} IS {};",
            Self::qualified_name(schema, table),
            Self::quote_ident(column),
            Self::comment_value_sql(comment)
        )
    }

    fn created_table_comments_sql(table: &RichTable) -> Vec<String> {
        let ddl_table = rich_table_to_table(table);
        TableSql::new(&ddl_table)
            .columns(&table.columns)
            .create_comments_sql()
    }

    /// Render a column's full type, schema-qualifying and quoting custom
    /// (enum) type names and appending array dimensions.
    fn column_type_sql(col: &Column) -> String {
        col.type_sql()
    }

    fn identity_sql(col: &Column, id: &super::ddl::Identity) -> Option<String> {
        use super::ddl::IdentityType;
        use super::grammar::PgTypeCategory;

        if PgTypeCategory::from_sql_type(&col.sql_type).is_serial() {
            return None;
        }

        let type_str = match id.type_ {
            IdentityType::Always => "ALWAYS",
            IdentityType::ByDefault => "BY DEFAULT",
        };

        let mut sql = format!(" GENERATED {type_str} AS IDENTITY");
        let mut options = Vec::new();
        if let Some(increment) = id.increment.as_ref() {
            options.push(format!("INCREMENT BY {increment}"));
        }
        if let Some(min) = id.min_value.as_ref() {
            options.push(format!("MINVALUE {min}"));
        }
        if let Some(max) = id.max_value.as_ref() {
            options.push(format!("MAXVALUE {max}"));
        }
        if let Some(start) = id.start_with.as_ref() {
            options.push(format!("START WITH {start}"));
        }
        if let Some(cache) = id.cache {
            options.push(format!("CACHE {cache}"));
        }
        if id.cycle.unwrap_or(false) {
            options.push("CYCLE".to_string());
        }
        if !options.is_empty() {
            let _ = write!(sql, " ({})", options.join(" "));
        }
        Some(sql)
    }

    /// Build the `SET GENERATED ... / SET <sequence option>` chain for an
    /// identity column whose configuration changed. `old` is the serialized
    /// [`super::ddl::Identity`] taken from the column diff (camelCase keys).
    /// Returns `None` when nothing tracked actually changed.
    fn identity_set_options_sql(
        new: &super::ddl::Identity,
        old: &serde_json::Value,
    ) -> Option<String> {
        use super::ddl::IdentityType;

        let old_str = |key: &str| old.get(key).and_then(|v| v.as_str());
        let mut pieces = Vec::new();

        let new_type = match new.type_ {
            IdentityType::Always => "always",
            IdentityType::ByDefault => "byDefault",
        };
        if old_str("type").is_some_and(|t| t != new_type) {
            pieces.push(match new.type_ {
                IdentityType::Always => "SET GENERATED ALWAYS".to_string(),
                IdentityType::ByDefault => "SET GENERATED BY DEFAULT".to_string(),
            });
        }

        if old_str("increment") != new.increment.as_deref() {
            let increment = new.increment.as_deref().unwrap_or("1");
            pieces.push(format!("SET INCREMENT BY {increment}"));
        }
        if old_str("minValue") != new.min_value.as_deref() {
            match new.min_value.as_deref() {
                Some(min) => pieces.push(format!("SET MINVALUE {min}")),
                None => pieces.push("SET NO MINVALUE".to_string()),
            }
        }
        if old_str("maxValue") != new.max_value.as_deref() {
            match new.max_value.as_deref() {
                Some(max) => pieces.push(format!("SET MAXVALUE {max}")),
                None => pieces.push("SET NO MAXVALUE".to_string()),
            }
        }
        if old_str("startWith") != new.start_with.as_deref()
            && let Some(start) = new.start_with.as_deref()
        {
            pieces.push(format!("SET START WITH {start}"));
        }
        let old_cache = old.get("cache").and_then(serde_json::Value::as_i64);
        if old_cache != new.cache.map(i64::from) {
            let cache = new.cache.unwrap_or(1);
            pieces.push(format!("SET CACHE {cache}"));
        }
        let old_cycle = old
            .get("cycle")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false);
        let new_cycle = new.cycle.unwrap_or(false);
        if old_cycle != new_cycle {
            pieces.push(if new_cycle {
                "SET CYCLE".to_string()
            } else {
                "SET NO CYCLE".to_string()
            });
        }

        if pieces.is_empty() {
            None
        } else {
            Some(pieces.join(" "))
        }
    }

    fn create_sequence_sql(s: &super::ddl::Sequence) -> String {
        s.create_sequence_sql()
    }

    fn create_table_sql(table: &RichTable) -> String {
        let ddl_table = rich_table_to_table(table);
        TableSql::new(&ddl_table)
            .columns(&table.columns)
            .primary_key(table.pk.as_ref())
            .foreign_keys(&table.foreign_keys)
            .unique_constraints(&table.uniques)
            .check_constraints(&table.checks)
            .create_table_sql()
    }

    fn drop_constraint_sql(schema: &str, table: &str, name: &str) -> String {
        format!(
            "ALTER TABLE {} DROP CONSTRAINT {};",
            Self::qualified_name(schema, table),
            Self::quote_ident(name)
        )
    }

    fn create_enum_sql(e: &super::ddl::Enum) -> String {
        e.create_enum_sql()
    }

    fn alter_enum_sql(to: &super::ddl::Enum, diff: &[EnumDiff]) -> String {
        diff.iter()
            .map(|d| to.add_value_sql(&d.value, d.before_value.as_deref()))
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn add_pk_sql(pk: &super::ddl::PrimaryKey) -> String {
        pk.add_pk_sql()
    }

    fn add_unique_sql(unique: &super::ddl::UniqueConstraint) -> String {
        unique.add_unique_sql()
    }

    fn recreate_column_sql(old_column: &Column, new_column: &Column) -> String {
        // Recreate column by dropping and adding.
        // Used for adding generated expressions, which PostgreSQL doesn't support via ALTER.
        let table_key = Self::qualified_name(&new_column.schema, &new_column.table);
        let drop_sql = format!(
            "ALTER TABLE {} DROP COLUMN {};",
            table_key,
            Self::quote_ident(&old_column.name)
        );
        let add_sql = format!(
            "ALTER TABLE {} ADD COLUMN {};",
            table_key,
            new_column.to_column_sql()
        );
        if new_column.comment.is_some() {
            format!(
                "{drop_sql}\n{add_sql}\n{}",
                Self::comment_on_column_sql(
                    &new_column.schema,
                    &new_column.table,
                    &new_column.name,
                    new_column.comment.as_deref(),
                )
            )
        } else {
            format!("{drop_sql}\n{add_sql}")
        }
    }

    fn add_column_sql(column: &Column, is_pk: bool) -> String {
        let pk_clause = if is_pk { " PRIMARY KEY" } else { "" };
        let add_sql = format!(
            "ALTER TABLE {} ADD COLUMN {}{};",
            Self::qualified_name(&column.schema, &column.table),
            column.to_column_sql(),
            pk_clause
        );
        if column.comment.is_some() {
            format!(
                "{add_sql}\n{}",
                Self::comment_on_column_sql(
                    &column.schema,
                    &column.table,
                    &column.name,
                    column.comment.as_deref(),
                )
            )
        } else {
            add_sql
        }
    }

    fn add_check_sql(check: &super::ddl::CheckConstraint) -> String {
        check.add_check_sql()
    }

    fn drop_policy_sql(policy: &super::ddl::Policy) -> String {
        format!(
            "DROP POLICY {} ON {};",
            Self::quote_ident(&policy.name),
            Self::qualified_name(&policy.schema, &policy.table)
        )
    }

    fn alter_view_sql(old_view: &View, new_view: &View) -> String {
        // PostgreSQL doesn't support ALTER VIEW for definition changes,
        // so we drop and recreate the view.
        let drop_sql = Self::drop_view_sql(old_view);
        let create_sql = Self::create_view_sql(new_view);
        format!("{drop_sql}\n{create_sql}")
    }

    /// DROP INDEX + CREATE INDEX from the new definition. The old index's
    /// CONCURRENTLY flag drives the drop, the new definition's flag drives
    /// the create.
    fn recreate_index_sql(old_index: &Index, new_index: &Index) -> String {
        let concurrently = if old_index.concurrently {
            "CONCURRENTLY "
        } else {
            ""
        };
        let drop_sql = format!(
            "DROP INDEX {}{};",
            concurrently,
            Self::qualified_name(&old_index.schema, &old_index.name)
        );
        format!("{drop_sql}\n{}", Self::create_index_sql(new_index))
    }

    /// ALTER SEQUENCE with only the changed options. Returns `None` when no
    /// tracked option differs.
    fn alter_sequence_sql(old: &Sequence, new: &Sequence) -> Option<String> {
        let mut options = Vec::new();

        if old.increment_by != new.increment_by {
            let increment = new.increment_by.as_deref().unwrap_or("1");
            options.push(format!("INCREMENT BY {increment}"));
        }
        if old.min_value != new.min_value {
            match new.min_value.as_deref() {
                Some(min) => options.push(format!("MINVALUE {min}")),
                None => options.push("NO MINVALUE".to_string()),
            }
        }
        if old.max_value != new.max_value {
            match new.max_value.as_deref() {
                Some(max) => options.push(format!("MAXVALUE {max}")),
                None => options.push("NO MAXVALUE".to_string()),
            }
        }
        if old.start_with != new.start_with
            && let Some(start) = new.start_with.as_deref()
        {
            options.push(format!("START WITH {start}"));
        }
        if old.cache_size != new.cache_size {
            let cache = new.cache_size.unwrap_or(1);
            options.push(format!("CACHE {cache}"));
        }
        if old.cycle.unwrap_or(false) != new.cycle.unwrap_or(false) {
            options.push(if new.cycle.unwrap_or(false) {
                "CYCLE".to_string()
            } else {
                "NO CYCLE".to_string()
            });
        }

        if options.is_empty() {
            return None;
        }
        Some(format!(
            "ALTER SEQUENCE {} {};",
            Self::qualified_name(&new.schema, &new.name),
            options.join(" ")
        ))
    }

    /// ALTER ROLE with only the changed flags. Returns `None` when no
    /// tracked flag differs.
    fn alter_role_sql(old: &Role, new: &Role) -> Option<String> {
        let mut options = Vec::new();

        if old.create_db.unwrap_or(false) != new.create_db.unwrap_or(false) {
            options.push(if new.create_db.unwrap_or(false) {
                "CREATEDB"
            } else {
                "NOCREATEDB"
            });
        }
        if old.create_role.unwrap_or(false) != new.create_role.unwrap_or(false) {
            options.push(if new.create_role.unwrap_or(false) {
                "CREATEROLE"
            } else {
                "NOCREATEROLE"
            });
        }
        if old.inherit.unwrap_or(true) != new.inherit.unwrap_or(true) {
            options.push(if new.inherit.unwrap_or(true) {
                "INHERIT"
            } else {
                "NOINHERIT"
            });
        }
        if old.can_login.unwrap_or(false) != new.can_login.unwrap_or(false) {
            options.push(if new.can_login.unwrap_or(false) {
                "LOGIN"
            } else {
                "NOLOGIN"
            });
        }
        if old.bypass_rls.unwrap_or(false) != new.bypass_rls.unwrap_or(false) {
            options.push(if new.bypass_rls.unwrap_or(false) {
                "BYPASSRLS"
            } else {
                "NOBYPASSRLS"
            });
        }

        let mut sql_options: Vec<String> = options.iter().map(ToString::to_string).collect();
        if old.conn_limit != new.conn_limit {
            sql_options.push(format!("CONNECTION LIMIT {}", new.conn_limit.unwrap_or(-1)));
        }

        if sql_options.is_empty() {
            return None;
        }
        Some(format!(
            "ALTER ROLE {} WITH {};",
            Self::quote_ident(&new.name),
            sql_options.join(" ")
        ))
    }

    /// Recreate an enum whose values were removed or reordered, drizzle-kit
    /// style: convert dependent columns to text, drop and recreate the type,
    /// convert the columns back with `USING ::text::type`, restore defaults.
    fn recreate_enum_sql(new_enum: &Enum, columns: &[Column]) -> String {
        let type_name = Self::qualified_name(&new_enum.schema, &new_enum.name);
        let mut stmts = Vec::new();

        for column in columns {
            if column.default.is_some() {
                stmts.push(format!(
                    "ALTER TABLE {} ALTER COLUMN {} DROP DEFAULT;",
                    Self::qualified_name(&column.schema, &column.table),
                    Self::quote_ident(&column.name)
                ));
            }
        }
        for column in columns {
            stmts.push(format!(
                "ALTER TABLE {} ALTER COLUMN {} SET DATA TYPE text USING {}::text;",
                Self::qualified_name(&column.schema, &column.table),
                Self::quote_ident(&column.name),
                Self::quote_ident(&column.name)
            ));
        }
        stmts.push(format!("DROP TYPE {type_name};"));
        stmts.push(Self::create_enum_sql(new_enum));
        for column in columns {
            stmts.push(format!(
                "ALTER TABLE {} ALTER COLUMN {} SET DATA TYPE {} USING {}::text::{};",
                Self::qualified_name(&column.schema, &column.table),
                Self::quote_ident(&column.name),
                type_name,
                Self::quote_ident(&column.name),
                type_name
            ));
        }
        for column in columns {
            if let Some(default) = column.default.as_deref() {
                stmts.push(format!(
                    "ALTER TABLE {} ALTER COLUMN {} SET DEFAULT {};",
                    Self::qualified_name(&column.schema, &column.table),
                    Self::quote_ident(&column.name),
                    default
                ));
            }
        }

        stmts.join("\n")
    }

    /// Render a statement as one or more single-command SQL strings.
    ///
    /// Several renderers pack multiple commands into one string separated by
    /// newlines (constraint/index/policy/enum recreates, chained alters,
    /// COMMENT ON riders). Drivers execute each returned entry through a
    /// prepared statement, which rejects multi-command text — so split them
    /// here. Joined output for display stays available via
    /// [`Self::statement_to_sql`].
    pub(crate) fn statement_to_sqls(stmt: JsonStatement) -> Vec<String> {
        Self::split_joined_commands(Self::statement_to_sql(stmt))
    }

    /// Split renderer output holding several `;`-terminated commands
    /// separated by newlines into individual statements. A boundary exists
    /// only after a line ending in `;` — interior lines of a single
    /// multi-line command (e.g. a CREATE TABLE body) end with `,` or `(`,
    /// never `;`, so such commands stay whole.
    fn split_joined_commands(sql: String) -> Vec<String> {
        if !sql.contains('\n') {
            return vec![sql];
        }
        let mut out = Vec::new();
        let mut current = String::new();
        for line in sql.lines() {
            if !current.is_empty() {
                current.push('\n');
            }
            current.push_str(line);
            if line.trim_end().ends_with(';') {
                out.push(std::mem::take(&mut current));
            }
        }
        if !current.trim().is_empty() {
            out.push(current);
        }
        out
    }

    pub(crate) fn statement_to_sql(stmt: JsonStatement) -> String {
        match stmt {
            JsonStatement::CreateSchema { name } => {
                format!("CREATE SCHEMA {};", Self::quote_ident(&name))
            }
            JsonStatement::DropSchema { name } => {
                format!("DROP SCHEMA {};", Self::quote_ident(&name))
            }
            JsonStatement::RenameSchema { from, to } => {
                format!(
                    "ALTER SCHEMA {} RENAME TO {};",
                    Self::quote_ident(&from.name),
                    Self::quote_ident(&to.name)
                )
            }
            JsonStatement::CreateEnum { enum_: e } => Self::create_enum_sql(&e),
            JsonStatement::DropEnum { enum_: e } => {
                format!("DROP TYPE {};", Self::qualified_name(&e.schema, &e.name))
            }
            JsonStatement::AlterEnum { from: _, to, diff } => Self::alter_enum_sql(&to, &diff),
            JsonStatement::CreateSequence { sequence: s } => Self::create_sequence_sql(&s),
            JsonStatement::DropSequence { sequence: s } => format!(
                "DROP SEQUENCE {};",
                Self::qualified_name(&s.schema, &s.name)
            ),
            JsonStatement::CreateTable { table } => Self::create_table_sql(&table),
            JsonStatement::DropTable { table, .. } => format!(
                "DROP TABLE {};",
                Self::qualified_name(&table.schema, &table.name)
            ),
            JsonStatement::RenameTable { schema, from, to } => format!(
                "ALTER TABLE {} RENAME TO {};",
                Self::qualified_name(&schema, &from),
                Self::quote_ident(&to)
            ),
            JsonStatement::AddColumn { column, is_pk, .. } => Self::add_column_sql(&column, is_pk),
            JsonStatement::DropColumn { column } => format!(
                "ALTER TABLE {} DROP COLUMN {};",
                Self::qualified_name(&column.schema, &column.table),
                Self::quote_ident(&column.name)
            ),
            JsonStatement::RenameColumn { from, to } => format!(
                "ALTER TABLE {} RENAME COLUMN {} TO {};",
                Self::qualified_name(&from.schema, &from.table),
                Self::quote_ident(&from.name),
                Self::quote_ident(&to.name)
            ),
            JsonStatement::AlterColumn {
                to,
                was_enum,
                is_enum,
                diff,
            } => Self::alter_column_sql(&to, was_enum, is_enum, &diff),
            JsonStatement::RecreateColumn {
                old_column,
                new_column,
            } => Self::recreate_column_sql(&old_column, &new_column),
            JsonStatement::CreateIndex { index } => Self::create_index_sql(&index),
            JsonStatement::DropIndex { index } => format!(
                "DROP INDEX {};",
                Self::qualified_name(&index.schema, &index.name)
            ),
            JsonStatement::CreateFk { fk } => Self::add_fk_sql(&fk),
            JsonStatement::DropFk { fk } => {
                Self::drop_constraint_sql(&fk.schema, &fk.table, &fk.name)
            }
            JsonStatement::CreateView { view } => Self::create_view_sql(&view),
            JsonStatement::DropView { view } => Self::drop_view_sql(&view),
            JsonStatement::AlterView { old_view, new_view } => {
                Self::alter_view_sql(&old_view, &new_view)
            }
            JsonStatement::AddPk { pk } => Self::add_pk_sql(&pk),
            JsonStatement::DropPk { pk } => {
                Self::drop_constraint_sql(&pk.schema, &pk.table, &pk.name)
            }
            JsonStatement::AddUnique { unique } => Self::add_unique_sql(&unique),
            JsonStatement::DropUnique { unique } => {
                Self::drop_constraint_sql(&unique.schema, &unique.table, &unique.name)
            }
            JsonStatement::AddCheck { check } => Self::add_check_sql(&check),
            JsonStatement::DropCheck { check } => {
                Self::drop_constraint_sql(&check.schema, &check.table, &check.name)
            }
            JsonStatement::CreateRole { role } => Self::create_role_sql(&role),
            JsonStatement::DropRole { role } => {
                format!("DROP ROLE {};", Self::quote_ident(&role.name))
            }
            JsonStatement::CreatePolicy { policy } => Self::create_policy_sql(&policy),
            JsonStatement::DropPolicy { policy } => Self::drop_policy_sql(&policy),
            JsonStatement::AlterTable {
                old_table,
                new_table,
            } => Self::alter_table_sql(&old_table, &new_table)
                .expect("alter table statement was prechecked"),
            JsonStatement::RecreateFk { old_fk, new_fk } => format!(
                "{}\n{}",
                Self::drop_constraint_sql(&old_fk.schema, &old_fk.table, &old_fk.name),
                Self::add_fk_sql(&new_fk)
            ),
            JsonStatement::RecreateUnique {
                old_unique,
                new_unique,
            } => format!(
                "{}\n{}",
                Self::drop_constraint_sql(&old_unique.schema, &old_unique.table, &old_unique.name),
                Self::add_unique_sql(&new_unique)
            ),
            JsonStatement::RecreateIndex {
                old_index,
                new_index,
            } => Self::recreate_index_sql(&old_index, &new_index),
            JsonStatement::RecreatePk { old_pk, new_pk } => format!(
                "{}\n{}",
                Self::drop_constraint_sql(&old_pk.schema, &old_pk.table, &old_pk.name),
                Self::add_pk_sql(&new_pk)
            ),
            JsonStatement::RecreateCheck {
                old_check,
                new_check,
            } => format!(
                "{}\n{}",
                Self::drop_constraint_sql(&old_check.schema, &old_check.table, &old_check.name),
                Self::add_check_sql(&new_check)
            ),
            JsonStatement::RecreatePolicy {
                old_policy,
                new_policy,
            } => format!(
                "{}\n{}",
                Self::drop_policy_sql(&old_policy),
                Self::create_policy_sql(&new_policy)
            ),
            JsonStatement::AlterSequence {
                old_sequence,
                new_sequence,
            } => Self::alter_sequence_sql(&old_sequence, &new_sequence)
                .expect("alter sequence statement was prechecked"),
            JsonStatement::AlterRole { old_role, new_role } => {
                Self::alter_role_sql(&old_role, &new_role)
                    .expect("alter role statement was prechecked")
            }
            JsonStatement::RecreateEnum {
                new_enum, columns, ..
            } => Self::recreate_enum_sql(&new_enum, &columns),
        }
    }

    fn create_index_sql(index: &Index) -> String {
        index.create_index_sql()
    }

    fn create_view_sql(view: &View) -> String {
        view.create_view_sql()
    }

    fn drop_view_sql(view: &View) -> String {
        let mat = if view.materialized {
            "MATERIALIZED "
        } else {
            ""
        };
        format!(
            "DROP {}VIEW {};",
            mat,
            Self::qualified_name(&view.schema, &view.name)
        )
    }

    fn alter_column_sql(
        to: &Column,
        was_enum: bool,
        is_enum: bool,
        diff: &HashMap<String, serde_json::Value>,
    ) -> String {
        let table_key = Self::qualified_name(&to.schema, &to.table);
        let mut stmts = Vec::new();

        if diff.contains_key("type") {
            let type_sql = Self::column_type_sql(to);
            // Enum-to-enum conversions must round-trip through text
            // (drizzle-kit parity): PostgreSQL has no direct cast between
            // distinct enum types.
            let using_cast = if was_enum && is_enum {
                format!("{}::text::{type_sql}", Self::quote_ident(&to.name))
            } else {
                format!("{}::{type_sql}", Self::quote_ident(&to.name))
            };
            stmts.push(format!(
                "ALTER TABLE {} ALTER COLUMN {} SET DATA TYPE {} USING {};",
                table_key,
                Self::quote_ident(&to.name),
                type_sql,
                using_cast
            ));
        }

        if diff.contains_key("notNull") {
            if to.not_null {
                stmts.push(format!(
                    "ALTER TABLE {} ALTER COLUMN {} SET NOT NULL;",
                    table_key,
                    Self::quote_ident(&to.name)
                ));
            } else {
                stmts.push(format!(
                    "ALTER TABLE {} ALTER COLUMN {} DROP NOT NULL;",
                    table_key,
                    Self::quote_ident(&to.name)
                ));
            }
        }

        if diff.contains_key("default") {
            if let Some(default) = &to.default {
                stmts.push(format!(
                    "ALTER TABLE {} ALTER COLUMN {} SET DEFAULT {};",
                    table_key,
                    Self::quote_ident(&to.name),
                    default
                ));
            } else {
                stmts.push(format!(
                    "ALTER TABLE {} ALTER COLUMN {} DROP DEFAULT;",
                    table_key,
                    Self::quote_ident(&to.name)
                ));
            }
        }

        if diff.contains_key("generated") && to.generated.is_none() {
            stmts.push(format!(
                "ALTER TABLE {} ALTER COLUMN {} DROP EXPRESSION;",
                table_key,
                Self::quote_ident(&to.name)
            ));
        }

        if diff.contains_key("identity")
            && !super::grammar::PgTypeCategory::from_sql_type(&to.sql_type).is_serial()
        {
            let old_identity = diff
                .get("identity")
                .and_then(|change| change.get("from"))
                .filter(|from| !from.is_null());
            match (&to.identity, old_identity) {
                // None -> Some: ADD GENERATED ... AS IDENTITY.
                (Some(id), None) => {
                    if let Some(identity_sql) = Self::identity_sql(to, id) {
                        stmts.push(format!(
                            "ALTER TABLE {} ALTER COLUMN {} ADD{};",
                            table_key,
                            Self::quote_ident(&to.name),
                            identity_sql
                        ));
                    }
                }
                // Some -> Some: ALTER COLUMN ... SET GENERATED / SET <option>.
                // `ADD GENERATED` on an existing identity column is invalid.
                (Some(id), Some(old)) => {
                    if let Some(set_sql) = Self::identity_set_options_sql(id, old) {
                        stmts.push(format!(
                            "ALTER TABLE {} ALTER COLUMN {} {};",
                            table_key,
                            Self::quote_ident(&to.name),
                            set_sql
                        ));
                    }
                }
                // Some -> None: DROP IDENTITY.
                (None, _) => {
                    stmts.push(format!(
                        "ALTER TABLE {} ALTER COLUMN {} DROP IDENTITY;",
                        table_key,
                        Self::quote_ident(&to.name)
                    ));
                }
            }
        }

        if diff.contains_key("comment") {
            stmts.push(Self::comment_on_column_sql(
                &to.schema,
                &to.table,
                &to.name,
                to.comment.as_deref(),
            ));
        }

        if stmts.is_empty() {
            format!("-- No column changes for {}.{}", to.table, to.name)
        } else {
            stmts.join("\n")
        }
    }

    fn create_role_sql(role: &super::ddl::Role) -> String {
        let mut sql = format!("CREATE ROLE {}", Self::quote_ident(&role.name));
        if role.create_db.unwrap_or(false) {
            sql.push_str(" CREATEDB");
        }
        if role.create_role.unwrap_or(false) {
            sql.push_str(" CREATEROLE");
        }
        if role.inherit.unwrap_or(true) {
            sql.push_str(" INHERIT");
        } else {
            sql.push_str(" NOINHERIT");
        }
        if role.can_login.unwrap_or(false) {
            sql.push_str(" LOGIN");
        }
        if role.bypass_rls.unwrap_or(false) {
            sql.push_str(" BYPASSRLS");
        }
        if let Some(conn_limit) = role.conn_limit {
            let _ = write!(sql, " CONNECTION LIMIT {conn_limit}");
        }
        sql.push(';');
        sql
    }

    fn create_policy_sql(policy: &super::ddl::Policy) -> String {
        policy.create_policy_sql()
    }

    fn add_fk_sql(fk: &ForeignKey) -> String {
        fk.add_fk_sql()
    }

    fn alter_table_sql(old: &Table, new: &Table) -> Option<String> {
        let mut stmts = Vec::new();
        let table_name = Self::qualified_name(&new.schema, &new.name);
        let old_unlogged = old.is_unlogged.unwrap_or(false);
        let new_unlogged = new.is_unlogged.unwrap_or(false);
        if old_unlogged != new_unlogged {
            let logged = if new_unlogged { "UNLOGGED" } else { "LOGGED" };
            stmts.push(format!("ALTER TABLE {table_name} SET {logged};"));
        }

        if old.tablespace.as_deref() != new.tablespace.as_deref() {
            let tablespace = new.tablespace.as_deref().unwrap_or("pg_default");
            stmts.push(format!(
                "ALTER TABLE {table_name} SET TABLESPACE {};",
                Self::quote_ident(tablespace)
            ));
        }

        if old.comment.as_deref() != new.comment.as_deref() {
            stmts.push(Self::comment_on_table_sql(
                &new.schema,
                &new.name,
                new.comment.as_deref(),
            ));
        }

        if stmts.is_empty() {
            None
        } else {
            Some(stmts.join("\n"))
        }
    }
}

// =============================================================================
// Topological Sort for Table Dependencies
// =============================================================================

/// Topological sort tables for CREATE: referenced tables come first
fn topological_sort_tables_for_create(
    table_keys: &[String],
    diff: &[EntityDiff],
) -> CreateTableOrder {
    if table_keys.len() <= 1 {
        return CreateTableOrder {
            ordered: table_keys.to_vec(),
            cycle_tables: HashSet::new(),
        };
    }

    // Build a set of table keys for quick lookup
    let table_set: HashSet<&String> = table_keys.iter().collect();

    // Build dependency graph: table -> tables it depends on (via FKs)
    let mut dependencies: HashMap<String, HashSet<String>> = HashMap::new();
    for table_key in table_keys {
        dependencies.insert(table_key.clone(), HashSet::new());
    }

    // Find FK dependencies from created FKs
    for d in diff
        .iter()
        .filter(|d| d.kind == EntityKind::ForeignKey && d.diff_type == DiffType::Create)
    {
        if let Some(PostgresEntity::ForeignKey(fk)) = &d.right {
            let from_table = format!("{}.{}", fk.schema, fk.table);
            let to_table = format!("{}.{}", fk.schema_to, fk.table_to);

            // from_table depends on to_table (to_table must be created first)
            if table_set.contains(&from_table)
                && table_set.contains(&to_table)
                && let Some(deps) = dependencies.get_mut(&from_table)
            {
                deps.insert(to_table);
            }
        }
    }

    // Tables with no dependencies come first, then tables that depend on them, etc.
    let mut result = Vec::new();
    let mut remaining: HashSet<String> = table_keys.iter().cloned().collect();
    let mut satisfied: HashSet<String> = HashSet::new();
    let mut cycle_tables = HashSet::new();

    while !remaining.is_empty() {
        // Find tables whose dependencies are all satisfied
        let ready: Vec<String> = remaining
            .iter()
            .filter(|t| {
                dependencies
                    .get(*t)
                    .is_none_or(|deps| deps.iter().all(|d| satisfied.contains(d)))
            })
            .cloned()
            .collect();

        if ready.is_empty() {
            // Circular dependency: create remaining tables without their cycle FKs,
            // then add those constraints after all tables exist.
            cycle_tables = remaining.clone();
            result.extend(remaining);
            break;
        }

        for t in ready {
            remaining.remove(&t);
            satisfied.insert(t.clone());
            result.push(t);
        }
    }

    CreateTableOrder {
        ordered: result,
        cycle_tables,
    }
}

/// Topological sort tables for DROP: tables with FKs come first (reverse of create)
fn topological_sort_tables_for_drop(table_keys: &[String], diff: &[EntityDiff]) -> Vec<String> {
    // For drops, reverse the create order: tables that reference others drop first
    let create_order = topological_sort_tables_for_create(table_keys, diff);
    create_order.ordered.into_iter().rev().collect()
}

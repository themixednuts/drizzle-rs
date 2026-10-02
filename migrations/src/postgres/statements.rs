//! PostgreSQL migration statements and their SQL rendering.

use super::collection::{DiffType, EntityDiff, PostgresDDL, normalize_expression};
use super::ddl::{
    CheckConstraint, Column, Enum, ForeignKey, Index, Policy, PostgresEntity, PrimaryKey, Role,
    Schema, Sequence, Table, TableSql, UniqueConstraint, View,
};
use crate::traits::EntityKind;
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::fmt::Write;

/// The line that separates statements in `migration.sql`.
pub const BREAKPOINT: &str = "--> statement-breakpoint";

#[derive(Debug, Clone)]
struct CreateTableOrder {
    ordered: Vec<String>,
    cycle_tables: HashSet<String>,
}

// =============================================================================
// JSON Statements
// =============================================================================

/// One PostgreSQL migration operation, serialized with a snake_case `type`
/// tag (drizzle-kit's statement format). [`Generator`] renders these to SQL.
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
        from: Box<Column>,
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
    /// `ALTER TABLE ... RENAME CONSTRAINT` (keys and foreign keys whose
    /// derived name follows a table or column rename).
    RenameConstraint {
        schema: String,
        table: String,
        from: String,
        to: String,
    },
    /// `ALTER INDEX ... RENAME TO`.
    RenameIndex {
        schema: String,
        from: String,
        to: String,
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
        /// Columns typed with the enum before the migration; converted to
        /// text while the type is recreated.
        columns: Vec<Column>,
        /// Columns typed with the enum after the migration; converted back
        /// and given their default.
        restore: Vec<Column>,
    },
}

/// One enum label change.
#[derive(Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct EnumDiff {
    /// Change kind, e.g. `"added"`.
    pub r#type: String,
    /// The label.
    pub value: String,
    /// The existing label to insert before (`ADD VALUE ... BEFORE`); `None`
    /// appends.
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
// Generation context
// =============================================================================

/// Constraints and indexes that disappear with a column that is dropped and
/// re-added (a column becoming generated, or changing its expression), and
/// must be put back afterwards (drizzle-kit `recreate_column` parity).
#[derive(Default)]
struct RecreateExtras {
    /// FKs that reference a recreated column: dropped before the column.
    drop_fks: Vec<ForeignKey>,
    create_indexes: Vec<Index>,
    add_uniques: Vec<UniqueConstraint>,
    add_pks: Vec<PrimaryKey>,
    create_fks: Vec<ForeignKey>,
    /// Keys (`schema.name`) of indexes/constraints the recreation removes
    /// that also have an alter diff: their recreate renders the create half
    /// only.
    implicitly_dropped: HashSet<(EntityKind, String)>,
}

struct GenContext<'a> {
    diff: &'a [EntityDiff],
    index: DiffIndex<'a>,
    prev: Option<&'a PostgresDDL>,
    cur: Option<&'a PostgresDDL>,
    created_tables: Vec<String>,
    dropped_tables: Vec<String>,
    recreate: RecreateExtras,
    /// Columns (`schema.table.name`) an enum recreation converts back to
    /// the enum and gives their default; their own alter skips the default.
    enum_restored_columns: HashSet<String>,
}

fn column_key(column: &Column) -> String {
    format!("{}.{}.{}", column.schema, column.table, column.name)
}

impl<'a> GenContext<'a> {
    fn new(
        diff: &'a [EntityDiff],
        prev: Option<&'a PostgresDDL>,
        cur: Option<&'a PostgresDDL>,
    ) -> Self {
        let created_tables = diff
            .iter()
            .filter(|d| d.diff_type == DiffType::Create && d.kind == EntityKind::Table)
            .map(|d| d.name.clone())
            .collect();
        let dropped_tables = diff
            .iter()
            .filter(|d| d.diff_type == DiffType::Drop && d.kind == EntityKind::Table)
            .map(|d| d.name.clone())
            .collect();
        let mut ctx = Self {
            diff,
            index: DiffIndex::new(diff),
            prev,
            cur,
            created_tables,
            dropped_tables,
            recreate: RecreateExtras::default(),
            enum_restored_columns: HashSet::new(),
        };
        ctx.recreate = ctx.collect_recreate_extras();
        ctx.enum_restored_columns = ctx.collect_enum_restored_columns();
        ctx
    }

    fn diff_for(&self, kind: EntityKind, key: &str) -> Option<DiffType> {
        self.diff
            .iter()
            .find(|d| d.kind == kind && d.name == key)
            .map(|d| d.diff_type)
    }

    fn on_dropped_table(&self, d: &EntityDiff) -> bool {
        Generator::get_parent_table_key(d).is_some_and(|key| self.dropped_tables.contains(&key))
    }

    fn on_created_table(&self, d: &EntityDiff) -> bool {
        Generator::get_parent_table_key(d).is_some_and(|key| self.created_tables.contains(&key))
    }

    /// drizzle-kit keeps an FK drop unless its table is dropped while the
    /// referenced table survives (`DROP TABLE` removes it then). FKs
    /// between two dropped tables are dropped first, which makes the order
    /// of the table drops irrelevant.
    fn keep_fk_drop(&self, fk: &ForeignKey) -> bool {
        let from_dropped = self
            .dropped_tables
            .contains(&Generator::table_key(&fk.schema, &fk.table));
        let to_dropped = (fk.schema != fk.schema_to || fk.table != fk.table_to)
            && self
                .dropped_tables
                .contains(&Generator::table_key(&fk.schema_to, &fk.table_to));
        !(from_dropped && !to_dropped)
    }

    fn sorted_drops(&self) -> Vec<String> {
        topological_sort_tables_for_drop(&self.dropped_tables, self.diff)
    }

    fn is_enum_recreate(&self, d: &EntityDiff) -> bool {
        matches!(
            (&d.left, &d.right),
            (Some(PostgresEntity::Enum(old)), Some(PostgresEntity::Enum(new)))
                if old.values != new.values
                    && !Generator::enum_values_are_pure_additions(&old.values, &new.values)
        )
    }

    fn is_column_recreate(&self, d: &EntityDiff) -> bool {
        matches!(
            (&d.left, &d.right),
            (Some(PostgresEntity::Column(old)), Some(PostgresEntity::Column(new)))
                if Generator::column_needs_recreate(old, new)
        )
    }

    fn is_enum_type(ddl: &PostgresDDL, column: &Column) -> bool {
        let schema = column.type_schema.as_deref().unwrap_or("public");
        ddl.enums.one(schema, &column.sql_type).is_some()
    }

    fn column_is_enum(column: &Column, enum_: &Enum) -> bool {
        column.sql_type.as_ref() == enum_.name.as_ref()
            && column.type_schema.as_deref().unwrap_or("public") == enum_.schema.as_ref()
    }

    /// Columns the recreation of an enum converts back to it: those still
    /// typed with it after the migration.
    fn collect_enum_restored_columns(&self) -> HashSet<String> {
        let mut out = HashSet::new();
        for d in self.diff.iter().filter(|d| self.is_enum_recreate(d)) {
            let (Some(PostgresEntity::Enum(old)), Some(PostgresEntity::Enum(new))) =
                (&d.left, &d.right)
            else {
                continue;
            };
            let (_, restore) = self.enum_recreate_columns(old, new);
            out.extend(restore.iter().map(column_key));
        }
        out
    }

    /// The statement for one diff entry, with the cross-entity context the
    /// plain conversion lacks.
    fn statement_for(&self, d: &EntityDiff) -> Option<JsonStatement> {
        let stmt = Generator::diff_to_statement_with_context(d, &self.index, self.cur)?;
        Some(match stmt {
            JsonStatement::RecreateEnum {
                old_enum, new_enum, ..
            } => {
                let (columns, restore) = self.enum_recreate_columns(&old_enum, &new_enum);
                JsonStatement::RecreateEnum {
                    old_enum,
                    new_enum,
                    columns,
                    restore,
                }
            }
            JsonStatement::AlterColumn {
                from, to, mut diff, ..
            } => {
                let was_enum = self.prev.map_or(from.type_schema.is_some(), |prev| {
                    Self::is_enum_type(prev, &from)
                });
                let is_enum = self
                    .cur
                    .map_or(to.type_schema.is_some(), |cur| Self::is_enum_type(cur, &to));
                if self.enum_restored_columns.contains(&column_key(&to)) {
                    // The enum recreation already restored this default.
                    diff.remove("default");
                }
                if diff.is_empty() {
                    return None;
                }
                JsonStatement::AlterColumn {
                    from,
                    to,
                    was_enum,
                    is_enum,
                    diff,
                }
            }
            JsonStatement::RecreateIndex { new_index, .. }
                if self.implicitly_dropped(
                    EntityKind::Index,
                    &new_index.schema,
                    &new_index.name,
                ) =>
            {
                JsonStatement::CreateIndex { index: *new_index }
            }
            JsonStatement::RecreateUnique { new_unique, .. }
                if self.implicitly_dropped(
                    EntityKind::UniqueConstraint,
                    &new_unique.schema,
                    &new_unique.name,
                ) =>
            {
                JsonStatement::AddUnique { unique: new_unique }
            }
            JsonStatement::RecreatePk { new_pk, .. }
                if self.implicitly_dropped(
                    EntityKind::PrimaryKey,
                    &new_pk.schema,
                    &new_pk.name,
                ) =>
            {
                JsonStatement::AddPk { pk: new_pk }
            }
            JsonStatement::RecreateFk { new_fk, .. }
                if self.implicitly_dropped(
                    EntityKind::ForeignKey,
                    &new_fk.schema,
                    &new_fk.name,
                ) =>
            {
                JsonStatement::CreateFk { fk: new_fk }
            }
            other => other,
        })
    }

    fn implicitly_dropped(&self, kind: EntityKind, schema: &str, name: &str) -> bool {
        self.recreate
            .implicitly_dropped
            .contains(&(kind, format!("{schema}.{name}")))
    }

    /// The columns an enum recreation converts to text (every column typed
    /// with the enum before the migration, including ones dropped later in
    /// it) and the ones it converts back (still typed with the enum after).
    fn enum_recreate_columns(
        &self,
        old_enum: &Enum,
        new_enum: &Enum,
    ) -> (Vec<Column>, Vec<Column>) {
        let typed_in = |ddl: &PostgresDDL, enum_: &Enum| -> Vec<Column> {
            ddl.columns
                .list()
                .iter()
                .filter(|column| Self::column_is_enum(column, enum_))
                .cloned()
                .collect()
        };
        let Some(cur) = self.cur else {
            return (Vec::new(), Vec::new());
        };
        let Some(prev) = self.prev else {
            let columns = typed_in(cur, new_enum);
            return (columns.clone(), columns);
        };
        // Every column using the type now, including ones a later step
        // drops (with their table or alone): DROP TYPE needs them gone.
        let columns = typed_in(prev, old_enum);
        let converted: HashSet<String> = columns.iter().map(column_key).collect();
        let restore = typed_in(cur, new_enum)
            .into_iter()
            .filter(|column| converted.contains(&column_key(column)))
            .collect();
        (columns, restore)
    }

    /// Created tables in dependency order, each with its inlined columns and
    /// constraints. Returns the foreign keys left out of `CREATE TABLE`: FKs
    /// inside a reference cycle, and FKs to a surviving table whose columns
    /// or keys change later in this migration (the referenced unique key may
    /// not exist yet).
    fn push_created_tables(&self, sqls: &mut Vec<String>) -> Vec<ForeignKey> {
        let sorted = topological_sort_tables_for_create(&self.created_tables, self.diff);
        let mut deferred_fks = Vec::new();
        let mut rich_tables = Vec::new();
        for table_key in &sorted.ordered {
            let Some(table_diff) = self.index.table_diff(table_key) else {
                continue;
            };
            let Some(PostgresEntity::Table(table)) = &table_diff.right else {
                continue;
            };
            let mut rich_table = Generator::build_rich_table(table, &self.index);
            let (inline_fks, later_fks): (Vec<_>, Vec<_>) =
                rich_table.foreign_keys.into_iter().partition(|fk| {
                    !Generator::is_cycle_fk(fk, &sorted.cycle_tables) && !self.target_changes(fk)
                });
            rich_table.foreign_keys = inline_fks;
            deferred_fks.extend(later_fks);
            sqls.push(Generator::create_table_sql(&rich_table));
            if sorted.cycle_tables.is_empty() {
                Generator::push_created_table_extras(sqls, &rich_table);
            } else {
                rich_tables.push(rich_table);
            }
        }
        for rich_table in &rich_tables {
            Generator::push_created_table_extras(sqls, rich_table);
        }
        deferred_fks
    }

    /// Whether an FK's referenced table survives the migration but gains or
    /// changes columns, keys or indexes in it.
    fn target_changes(&self, fk: &ForeignKey) -> bool {
        let target = Generator::table_key(&fk.schema_to, &fk.table_to);
        if self.created_tables.contains(&target) {
            return false;
        }
        self.diff.iter().any(|d| {
            d.diff_type != DiffType::Drop
                && matches!(
                    d.kind,
                    EntityKind::Column
                        | EntityKind::PrimaryKey
                        | EntityKind::UniqueConstraint
                        | EntityKind::Index
                )
                && Generator::get_parent_table_key(d).as_deref() == Some(target.as_str())
        })
    }

    /// Row-level security toggles for tables that exist before and after.
    ///
    /// A table has RLS when it enables it explicitly or has any policy
    /// (drizzle-kit enables RLS for tables with policies), so adding the
    /// first policy enables it and removing the last one disables it unless
    /// the table enables it explicitly.
    fn rls_toggles(&self) -> Vec<(String, String, bool)> {
        let mut out = Vec::new();
        if let (Some(prev), Some(cur)) = (self.prev, self.cur) {
            for table in cur.tables.list() {
                let Some(prev_table) = prev.tables.one(&table.schema, &table.name) else {
                    continue;
                };
                let was = prev_table.is_rls_enabled.unwrap_or(false)
                    || !prev
                        .policies
                        .for_table(&table.schema, &table.name)
                        .is_empty();
                let is = table.is_rls_enabled.unwrap_or(false)
                    || !cur
                        .policies
                        .for_table(&table.schema, &table.name)
                        .is_empty();
                if was != is {
                    out.push((table.schema.to_string(), table.name.to_string(), is));
                }
            }
        } else {
            for d in self
                .diff
                .iter()
                .filter(|d| d.kind == EntityKind::Table && d.diff_type == DiffType::Alter)
            {
                if let (Some(PostgresEntity::Table(old)), Some(PostgresEntity::Table(new))) =
                    (&d.left, &d.right)
                {
                    let was = old.is_rls_enabled.unwrap_or(false);
                    let is = new.is_rls_enabled.unwrap_or(false);
                    if was != is {
                        out.push((new.schema.to_string(), new.name.to_string(), is));
                    }
                }
            }
        }
        out
    }

    /// Indexes, keys and FKs that dropping and re-adding a column removes,
    /// to be created again afterwards.
    fn collect_recreate_extras(&self) -> RecreateExtras {
        let mut extras = RecreateExtras::default();
        let Some(cur) = self.cur else {
            return extras;
        };
        let recreated: Vec<&Column> = self
            .diff
            .iter()
            .filter(|d| self.is_column_recreate(d))
            .filter_map(|d| match &d.right {
                Some(PostgresEntity::Column(column)) => Some(column),
                _ => None,
            })
            .collect();

        let mut seen: HashSet<(EntityKind, String)> = HashSet::new();
        for column in recreated {
            let (schema, table, name) = (&column.schema, &column.table, &column.name);
            let has_column =
                |columns: &[std::borrow::Cow<'static, str>]| columns.iter().any(|c| c == name);

            for index in cur.indexes.for_table(schema, table) {
                if !index
                    .columns
                    .iter()
                    .any(|c| !c.is_expression && c.value == *name)
                {
                    continue;
                }
                let key = format!("{}.{}", index.schema, index.name);
                if !seen.insert((EntityKind::Index, key.clone())) {
                    continue;
                }
                match self.diff_for(EntityKind::Index, &key) {
                    Some(DiffType::Create) => {}
                    Some(DiffType::Alter) => {
                        extras.implicitly_dropped.insert((EntityKind::Index, key));
                    }
                    _ => extras.create_indexes.push(index.clone()),
                }
            }
            for unique in cur.uniques.for_table(schema, table) {
                if !has_column(&unique.columns) {
                    continue;
                }
                let key = format!("{}.{}", unique.schema, unique.name);
                if !seen.insert((EntityKind::UniqueConstraint, key.clone())) {
                    continue;
                }
                match self.diff_for(EntityKind::UniqueConstraint, &key) {
                    Some(DiffType::Create) => {}
                    Some(DiffType::Alter) => {
                        extras
                            .implicitly_dropped
                            .insert((EntityKind::UniqueConstraint, key));
                    }
                    _ => extras.add_uniques.push(unique.clone()),
                }
            }
            if let Some(pk) = cur.pks.for_table(schema, table)
                && has_column(&pk.columns)
            {
                let key = format!("{}.{}", pk.schema, pk.name);
                if seen.insert((EntityKind::PrimaryKey, key.clone())) {
                    match self.diff_for(EntityKind::PrimaryKey, &key) {
                        Some(DiffType::Create) => {}
                        Some(DiffType::Alter) => {
                            extras
                                .implicitly_dropped
                                .insert((EntityKind::PrimaryKey, key));
                        }
                        _ => extras.add_pks.push(pk.clone()),
                    }
                }
            }
            for fk in cur.fks.list() {
                let from = fk.schema == *schema && fk.table == *table && has_column(&fk.columns);
                let to =
                    fk.schema_to == *schema && fk.table_to == *table && has_column(&fk.columns_to);
                if !from && !to {
                    continue;
                }
                let key = format!("{}.{}", fk.schema, fk.name);
                if !seen.insert((EntityKind::ForeignKey, key.clone())) {
                    continue;
                }
                match self.diff_for(EntityKind::ForeignKey, &key) {
                    Some(DiffType::Create) => {}
                    Some(DiffType::Alter) => {
                        if to && !from {
                            extras.drop_fks.push(fk.clone());
                        }
                        extras
                            .implicitly_dropped
                            .insert((EntityKind::ForeignKey, key));
                    }
                    _ => {
                        if to && !from {
                            // Referencing FKs block the column drop.
                            extras.drop_fks.push(fk.clone());
                        }
                        extras.create_fks.push(fk.clone());
                    }
                }
            }
        }
        extras
    }
}

// =============================================================================
// Generator
// =============================================================================

/// Renders PostgreSQL entity diffs as SQL statements in dependency order.
pub struct Generator {
    /// Stored for API symmetry; this generator returns separate statements
    /// and never joins them, so the flag has no effect.
    pub breakpoints: bool,
}

impl Default for Generator {
    fn default() -> Self {
        Self::new()
    }
}

impl Generator {
    /// Creates a generator.
    #[must_use]
    pub const fn new() -> Self {
        Self { breakpoints: true }
    }

    /// Sets [`breakpoints`](Self::breakpoints).
    #[must_use]
    pub const fn with_breakpoints(mut self, breakpoints: bool) -> Self {
        self.breakpoints = breakpoints;
        self
    }

    /// Generates SQL statements from entity diffs; same as
    /// [`generate_with_ddl`](Self::generate_with_ddl) without the current DDL.
    #[must_use]
    pub fn generate(&self, diff: &[EntityDiff]) -> Vec<String> {
        self.generate_with_context(diff, None, None)
    }

    /// Generate SQL statements from a set of entity diffs, with access to the
    /// full *current* DDL for cross-entity lookups (e.g. finding the columns
    /// that depend on an enum being recreated).
    ///
    /// Statement ordering follows drizzle-kit:
    ///
    /// 1. creates of schemas, enums (and `ADD VALUE`), sequences, roles,
    /// 2. view drops (including views whose definition changes),
    /// 3. recreation of enums whose values were removed or reordered,
    /// 4. table creates (with inlined columns and constraints; foreign keys
    ///    whose target is only completed later in the migration are
    ///    deferred to step 10),
    /// 5. policy and foreign-key drops,
    /// 6. table drops,
    /// 7. table alters (row-level security, logging, tablespace, comment),
    /// 8. unique/check/index/primary-key drops on surviving tables,
    /// 9. column adds, primary-key adds, generated-column recreation, column
    ///    drops, column alters,
    /// 10. unique, index, foreign-key and check creates on surviving tables,
    /// 11. view creates, policy creates,
    /// 12. enum, sequence, role and schema drops.
    #[must_use]
    pub fn generate_with_ddl(
        &self,
        diff: &[EntityDiff],
        cur_ddl: Option<&PostgresDDL>,
    ) -> Vec<String> {
        self.generate_with_context(diff, None, cur_ddl)
    }

    /// [`generate_with_ddl`](Self::generate_with_ddl) with the *previous*
    /// DDL as well, which lets the generator see what a dependent object
    /// looked like before the migration (enum-typed columns, row-level
    /// security implied by policies, serial columns).
    #[must_use]
    pub fn generate_with_context(
        &self,
        diff: &[EntityDiff],
        prev_ddl: Option<&PostgresDDL>,
        cur_ddl: Option<&PostgresDDL>,
    ) -> Vec<String> {
        let ctx = GenContext::new(diff, prev_ddl, cur_ddl);
        let mut sqls = Vec::new();

        let push_diff = |sqls: &mut Vec<String>, d: &EntityDiff| {
            if let Some(stmt) = ctx.statement_for(d) {
                sqls.extend(Self::statement_to_sqls(stmt));
            }
        };
        let of = |kind: EntityKind, diff_type: DiffType| {
            diff.iter()
                .filter(move |d| d.kind == kind && d.diff_type == diff_type)
        };

        // 1. Top-level creates. Enum `ADD VALUE` alters come right after the
        //    enum creates (recreated enums are handled with the columns).
        for d in of(EntityKind::Schema, DiffType::Create) {
            push_diff(&mut sqls, d);
        }
        for d in of(EntityKind::Enum, DiffType::Create) {
            push_diff(&mut sqls, d);
        }
        for d in of(EntityKind::Enum, DiffType::Alter) {
            if !ctx.is_enum_recreate(d) {
                push_diff(&mut sqls, d);
            }
        }
        for kind in [EntityKind::Sequence, EntityKind::Role] {
            for d in of(kind, DiffType::Create) {
                push_diff(&mut sqls, d);
            }
            for d in of(kind, DiffType::Alter) {
                push_diff(&mut sqls, d);
            }
        }

        // 2. View drops: dropped views and views whose definition changes
        //    (recreated in step 11). Views depend on columns that later
        //    steps drop or retype.
        for d in of(EntityKind::View, DiffType::Drop) {
            push_diff(&mut sqls, d);
        }
        for d in of(EntityKind::View, DiffType::Alter) {
            if let (Some(PostgresEntity::View(old)), Some(PostgresEntity::View(new))) =
                (&d.left, &d.right)
                && Self::view_needs_recreate(old, new)
            {
                sqls.push(Self::drop_view_sql(old));
            }
        }

        // 3. Enums whose values were removed or reordered are recreated
        //    before any new table or column can use them (DROP TYPE fails
        //    while a column of the old type exists).
        for d in of(EntityKind::Enum, DiffType::Alter) {
            if ctx.is_enum_recreate(d) {
                push_diff(&mut sqls, d);
            }
        }

        // 4. Table creates in dependency order.
        let deferred_fks = ctx.push_created_tables(&mut sqls);

        // 5. Policy drops, then foreign-key drops: FKs on surviving tables,
        //    FKs between two dropped tables (so the drop order of the tables
        //    no longer matters), and FKs that reference a column about to
        //    be recreated.
        for d in of(EntityKind::Policy, DiffType::Drop) {
            if !ctx.on_dropped_table(d) {
                push_diff(&mut sqls, d);
            }
        }
        for d in of(EntityKind::ForeignKey, DiffType::Drop) {
            if let Some(PostgresEntity::ForeignKey(fk)) = &d.left
                && ctx.keep_fk_drop(fk)
            {
                push_diff(&mut sqls, d);
            }
        }
        for fk in &ctx.recreate.drop_fks {
            sqls.push(Self::drop_constraint_sql(&fk.schema, &fk.table, &fk.name));
        }

        // 6. Table drops.
        for table_key in &ctx.sorted_drops() {
            if let Some(table_diff) = ctx.index.table_diff(table_key)
                && table_diff.diff_type == DiffType::Drop
            {
                push_diff(&mut sqls, table_diff);
            }
        }

        // 7. Table alters: row-level security toggles, then the rest.
        for (schema, name, enable) in ctx.rls_toggles() {
            sqls.push(Self::alter_rls_sql(&schema, &name, enable));
        }
        for d in of(EntityKind::Table, DiffType::Alter) {
            push_diff(&mut sqls, d);
        }

        // 8. Constraint and index drops on surviving tables (dropped tables
        //    took theirs with them).
        for kind in [
            EntityKind::UniqueConstraint,
            EntityKind::CheckConstraint,
            EntityKind::Index,
            EntityKind::PrimaryKey,
        ] {
            for d in of(kind, DiffType::Drop) {
                if !ctx.on_dropped_table(d) {
                    push_diff(&mut sqls, d);
                }
            }
        }

        // 9. Columns.
        for kind in [EntityKind::Column, EntityKind::PrimaryKey] {
            for d in of(kind, DiffType::Create) {
                if !ctx.on_created_table(d) {
                    push_diff(&mut sqls, d);
                }
            }
        }
        for d in of(EntityKind::Column, DiffType::Alter) {
            if ctx.is_column_recreate(d) {
                push_diff(&mut sqls, d);
            }
        }
        for d in of(EntityKind::Column, DiffType::Drop) {
            if !ctx.on_dropped_table(d) {
                push_diff(&mut sqls, d);
            }
        }
        for d in of(EntityKind::PrimaryKey, DiffType::Alter) {
            push_diff(&mut sqls, d);
        }
        for pk in &ctx.recreate.add_pks {
            sqls.push(Self::add_pk_sql(pk));
        }
        for d in of(EntityKind::Column, DiffType::Alter) {
            if !ctx.is_column_recreate(d) {
                push_diff(&mut sqls, d);
            }
        }

        // 10. Constraint and index creates on surviving tables, after every
        //    column they reference has its final type. Uniques and indexes
        //    precede foreign keys so an FK can target a new unique key.
        for d in of(EntityKind::Index, DiffType::Alter) {
            push_diff(&mut sqls, d);
        }
        for d in of(EntityKind::UniqueConstraint, DiffType::Create) {
            if !ctx.on_created_table(d) {
                push_diff(&mut sqls, d);
            }
        }
        for unique in &ctx.recreate.add_uniques {
            sqls.push(Self::add_unique_sql(unique));
        }
        for d in of(EntityKind::UniqueConstraint, DiffType::Alter) {
            push_diff(&mut sqls, d);
        }
        for d in of(EntityKind::Index, DiffType::Create) {
            if !ctx.on_created_table(d) {
                push_diff(&mut sqls, d);
            }
        }
        for index in &ctx.recreate.create_indexes {
            sqls.push(Self::create_index_sql(index));
        }
        for d in of(EntityKind::ForeignKey, DiffType::Create) {
            if !ctx.on_created_table(d) {
                push_diff(&mut sqls, d);
            }
        }
        for fk in deferred_fks.iter().chain(&ctx.recreate.create_fks) {
            sqls.push(Self::add_fk_sql(fk));
        }
        for d in of(EntityKind::ForeignKey, DiffType::Alter) {
            push_diff(&mut sqls, d);
        }
        for d in of(EntityKind::CheckConstraint, DiffType::Create) {
            if !ctx.on_created_table(d) {
                push_diff(&mut sqls, d);
            }
        }
        for d in of(EntityKind::CheckConstraint, DiffType::Alter) {
            push_diff(&mut sqls, d);
        }

        // 11. View creates (after every table/column they may select from),
        //     then policies.
        for d in of(EntityKind::View, DiffType::Create) {
            push_diff(&mut sqls, d);
        }
        for d in of(EntityKind::View, DiffType::Alter) {
            if let (Some(PostgresEntity::View(old)), Some(PostgresEntity::View(new))) =
                (&d.left, &d.right)
                && Self::view_needs_recreate(old, new)
            {
                sqls.push(Self::create_view_sql(new));
            }
        }
        for d in of(EntityKind::Policy, DiffType::Create) {
            if !ctx.on_created_table(d) {
                push_diff(&mut sqls, d);
            }
        }
        for d in of(EntityKind::Policy, DiffType::Alter) {
            push_diff(&mut sqls, d);
        }

        // 12. Drops of top-level entities, after the column alters that
        //     move columns off a dropped enum or sequence, the policies that
        //     referenced a dropped role, and everything inside a dropped
        //     schema.
        for kind in [
            EntityKind::Enum,
            EntityKind::Sequence,
            EntityKind::Role,
            EntityKind::Schema,
        ] {
            for d in of(kind, DiffType::Drop) {
                push_diff(&mut sqls, d);
            }
        }

        sqls
    }

    /// A view whose definition (or another property `PostgreSQL` cannot
    /// alter in place) changed is dropped and created again.
    fn view_needs_recreate(old: &View, new: &View) -> bool {
        !(old.is_existing || new.is_existing)
    }

    fn alter_rls_sql(schema: &str, name: &str, enable: bool) -> String {
        format!(
            "ALTER TABLE {} {} ROW LEVEL SECURITY;",
            Self::qualified_name(schema, name),
            if enable { "ENABLE" } else { "DISABLE" }
        )
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

        // Policies only take effect with row-level security on; drizzle-kit
        // enables it for every table that has one.
        if table.is_rls_enabled.unwrap_or(false) || !table.policies.is_empty() {
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
            // `public` always exists. drizzle-rs snapshots list it and
            // drizzle-kit snapshots leave it implicit, so it is never created.
            PostgresEntity::Schema(s) if s.name == "public" => None,
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
            // Never dropped; see `create_diff_to_statement`.
            PostgresEntity::Schema(s) if s.name == "public" => None,
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
                        restore: columns.clone(),
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
                if Self::column_needs_recreate(old, new) {
                    Some(JsonStatement::RecreateColumn {
                        old_column: Box::new(old.clone()),
                        new_column: Box::new(new.clone()),
                    })
                } else {
                    let diff = Self::build_column_diff(old, new);
                    if diff.is_empty() {
                        return None;
                    }
                    let is_custom = |column: &Column| {
                        column
                            .type_schema
                            .as_deref()
                            .is_some_and(|schema| !schema.eq_ignore_ascii_case("pg_catalog"))
                    };
                    Some(JsonStatement::AlterColumn {
                        from: Box::new(old.clone()),
                        to: Box::new(new.clone()),
                        was_enum: is_custom(old),
                        is_enum: is_custom(new),
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

    /// Whether a column change needs `DROP COLUMN` + `ADD COLUMN`:
    /// `PostgreSQL` cannot add a generation expression to an existing column,
    /// and changing one needs `SET EXPRESSION` (PostgreSQL 17+), so like
    /// drizzle-kit the column is recreated whenever its generated expression
    /// is set or changed.
    pub(crate) fn column_needs_recreate(old: &Column, new: &Column) -> bool {
        let Some(new_generated) = new.generated.as_ref() else {
            return false;
        };
        old.generated.as_ref().is_none_or(|old_generated| {
            old_generated.gen_type != new_generated.gen_type
                || normalize_expression(&old_generated.expression)
                    != normalize_expression(&new_generated.expression)
        })
    }

    /// Build a granular diff structure for column alterations: one entry per
    /// changed property (`type`, `default`, `notNull`, `generated`,
    /// `identity`, `collate`, `comment`), compared after normalizing
    /// spellings that `PostgreSQL` treats as equal (`int4`/`INTEGER`,
    /// `'-1'::integer`/`-1`, unset identity options/their defaults).
    fn build_column_diff(old: &Column, new: &Column) -> HashMap<String, serde_json::Value> {
        use super::collection::{
            normalize_column_type_for_compare, normalize_default_for_compare,
            normalize_identity_options,
        };

        let mut diff = HashMap::new();
        let change = |from: serde_json::Value, to: serde_json::Value| {
            let mut map = serde_json::Map::new();
            map.insert("from".to_string(), from);
            map.insert("to".to_string(), to);
            serde_json::Value::Object(map)
        };
        let type_schema = |column: &Column| {
            column
                .type_schema
                .as_deref()
                .filter(|schema| !schema.eq_ignore_ascii_case("pg_catalog"))
                .map(ToString::to_string)
        };

        if normalize_column_type_for_compare(old) != normalize_column_type_for_compare(new)
            || type_schema(old) != type_schema(new)
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

            if type_schema(old) != type_schema(new) {
                diff.insert(
                    "typeSchema".to_string(),
                    change(
                        serde_json::json!(old.type_schema),
                        serde_json::json!(new.type_schema),
                    ),
                );
            }
        }

        if old.default.as_deref().map(normalize_default_for_compare)
            != new.default.as_deref().map(normalize_default_for_compare)
        {
            diff.insert(
                "default".to_string(),
                change(
                    serde_json::json!(old.default),
                    serde_json::json!(new.default),
                ),
            );
        }

        if old.not_null != new.not_null {
            diff.insert(
                "notNull".to_string(),
                change(
                    serde_json::json!(old.not_null),
                    serde_json::json!(new.not_null),
                ),
            );
        }

        let generated_key = |column: &Column| {
            column
                .generated
                .as_ref()
                .map(|g| (normalize_expression(&g.expression), g.gen_type))
        };
        if generated_key(old) != generated_key(new) {
            diff.insert(
                "generated".to_string(),
                change(
                    serde_json::json!(old.generated),
                    serde_json::json!(new.generated),
                ),
            );
        }

        if normalize_identity_options(old) != normalize_identity_options(new) {
            diff.insert(
                "identity".to_string(),
                change(
                    serde_json::json!(old.identity),
                    serde_json::json!(new.identity),
                ),
            );
        }

        if old.collate != new.collate {
            diff.insert(
                "collate".to_string(),
                change(
                    serde_json::json!(old.collate),
                    serde_json::json!(new.collate),
                ),
            );
        }

        if old.comment != new.comment {
            diff.insert(
                "comment".to_string(),
                change(
                    serde_json::json!(old.comment),
                    serde_json::json!(new.comment),
                ),
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
    /// identity column whose configuration changed. Returns `None` when
    /// nothing tracked actually changed.
    fn identity_set_options_sql(
        new: &super::ddl::Identity,
        old: &super::ddl::Identity,
    ) -> Option<String> {
        use super::ddl::IdentityType;

        let mut pieces = Vec::new();
        if old.type_ != new.type_ {
            pieces.push(match new.type_ {
                IdentityType::Always => "SET GENERATED ALWAYS".to_string(),
                IdentityType::ByDefault => "SET GENERATED BY DEFAULT".to_string(),
            });
        }
        if old.increment != new.increment {
            let increment = new.increment.as_deref().unwrap_or("1");
            pieces.push(format!("SET INCREMENT BY {increment}"));
        }
        if old.min_value != new.min_value {
            match new.min_value.as_deref() {
                Some(min) => pieces.push(format!("SET MINVALUE {min}")),
                None => pieces.push("SET NO MINVALUE".to_string()),
            }
        }
        if old.max_value != new.max_value {
            match new.max_value.as_deref() {
                Some(max) => pieces.push(format!("SET MAXVALUE {max}")),
                None => pieces.push("SET NO MAXVALUE".to_string()),
            }
        }
        if old.start_with != new.start_with
            && let Some(start) = new.start_with.as_deref()
        {
            pieces.push(format!("SET START WITH {start}"));
        }
        if old.cache != new.cache {
            pieces.push(format!("SET CACHE {}", new.cache.unwrap_or(1)));
        }
        let new_cycle = new.cycle.unwrap_or(false);
        if old.cycle.unwrap_or(false) != new_cycle {
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

    /// DROP INDEX + CREATE INDEX from the new definition. Like drizzle-kit,
    /// the drop never uses CONCURRENTLY (it cannot run inside a migration's
    /// transaction); the new definition's flag drives the create.
    fn recreate_index_sql(old_index: &Index, new_index: &Index) -> String {
        let drop_sql = format!(
            "DROP INDEX {};",
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
    /// style: convert the columns typed with it to text (dropping their
    /// defaults), drop and recreate the type, convert the columns that keep
    /// the type back with `USING`, and restore their defaults. Array columns
    /// keep their dimensions throughout. A column that moves to another type
    /// in the same migration stays text here and is converted by its own
    /// column alter.
    fn recreate_enum_sql(new_enum: &Enum, columns: &[Column], restore: &[Column]) -> String {
        let type_name = Self::qualified_name(&new_enum.schema, &new_enum.name);
        let dims = |column: &Column| "[]".repeat(column.dimensions.unwrap_or(0).max(0) as usize);
        let mut stmts = Vec::new();

        for column in columns {
            let table = Self::qualified_name(&column.schema, &column.table);
            let name = Self::quote_ident(&column.name);
            if column.default.is_some() {
                stmts.push(format!(
                    "ALTER TABLE {table} ALTER COLUMN {name} DROP DEFAULT;"
                ));
            }
            let text = format!("text{}", dims(column));
            stmts.push(format!(
                "ALTER TABLE {table} ALTER COLUMN {name} SET DATA TYPE {text} USING {name}::{text};"
            ));
        }
        stmts.push(format!("DROP TYPE {type_name};"));
        stmts.push(Self::create_enum_sql(new_enum));
        for column in restore {
            let table = Self::qualified_name(&column.schema, &column.table);
            let name = Self::quote_ident(&column.name);
            let enum_type = format!("{type_name}{}", dims(column));
            stmts.push(format!(
                "ALTER TABLE {table} ALTER COLUMN {name} SET DATA TYPE {enum_type} USING {name}::{enum_type};"
            ));
            if let Some(default) = column.default.as_deref() {
                stmts.push(format!(
                    "ALTER TABLE {table} ALTER COLUMN {name} SET DEFAULT {default};"
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
            .into_iter()
            .filter(|sql| !sql.trim().is_empty())
            .collect()
    }

    /// Split renderer output holding several `;`-terminated commands
    /// separated by newlines into individual statements. A boundary is a `;`
    /// at the end of a line outside any string literal, quoted identifier or
    /// dollar-quoted body — so a comment or default containing `;` followed
    /// by a newline stays whole, as does a multi-line CREATE TABLE body.
    fn split_joined_commands(sql: String) -> Vec<String> {
        if !sql.contains('\n') {
            return vec![sql];
        }
        let bytes = sql.as_bytes();
        let mut out = Vec::new();
        let mut start = 0;
        let mut i = 0;
        let mut quote: Option<u8> = None;
        let mut dollar_tag: Option<&str> = None;
        while i < bytes.len() {
            let b = bytes[i];
            if let Some(tag) = dollar_tag {
                if sql[i..].starts_with(tag) {
                    i += tag.len();
                    dollar_tag = None;
                } else {
                    i += 1;
                }
                continue;
            }
            if let Some(q) = quote {
                // A doubled quote closes and immediately reopens: still
                // inside the literal either way.
                if b == q {
                    quote = None;
                }
                i += 1;
                continue;
            }
            match b {
                b'\'' | b'"' => quote = Some(b),
                b'$' => {
                    let tag_end = sql[i + 1..]
                        .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
                        .map(|offset| i + 1 + offset);
                    if let Some(tag_end) = tag_end
                        && bytes[tag_end] == b'$'
                        && !sql[i + 1..tag_end].starts_with(|c: char| c.is_ascii_digit())
                    {
                        dollar_tag = Some(&sql[i..=tag_end]);
                        i = tag_end + 1;
                        continue;
                    }
                }
                b';' => {
                    let rest = &sql[i + 1..];
                    if let Some(newline) = rest.find('\n')
                        && rest[..newline].trim().is_empty()
                    {
                        out.push(sql[start..=i].to_string());
                        start = i + 1 + newline + 1;
                        i = start;
                        continue;
                    }
                }
                _ => {}
            }
            i += 1;
        }
        if start < sql.len() && !sql[start..].trim().is_empty() {
            out.push(sql[start..].to_string());
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
            JsonStatement::RenameConstraint {
                schema,
                table,
                from,
                to,
            } => format!(
                "ALTER TABLE {} RENAME CONSTRAINT {} TO {};",
                Self::qualified_name(&schema, &table),
                Self::quote_ident(&from),
                Self::quote_ident(&to)
            ),
            JsonStatement::RenameIndex { schema, from, to } => format!(
                "ALTER INDEX {} RENAME TO {};",
                Self::qualified_name(&schema, &from),
                Self::quote_ident(&to)
            ),
            JsonStatement::AlterColumn {
                from,
                to,
                was_enum,
                is_enum,
                diff,
            } => Self::alter_column_sql(&from, &to, was_enum, is_enum, &diff).join("\n"),
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
                new_enum,
                columns,
                restore,
                ..
            } => Self::recreate_enum_sql(&new_enum, &columns, &restore),
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

    /// The integer type behind a serial pseudo-type (`serial` → `integer`);
    /// `None` for other types.
    fn serial_base_type(sql_type: &str) -> Option<&'static str> {
        use super::grammar::PgTypeCategory;
        match PgTypeCategory::from_sql_type(sql_type) {
            PgTypeCategory::Serial => Some("integer"),
            PgTypeCategory::BigSerial => Some("bigint"),
            PgTypeCategory::SmallSerial => Some("smallint"),
            _ => None,
        }
    }

    /// The sequence `PostgreSQL` creates for a serial column.
    fn serial_sequence_name(column: &Column) -> String {
        Self::qualified_name(
            &column.schema,
            &format!("{}_{}_seq", column.table, column.name),
        )
    }

    /// `ALTER TABLE ... ALTER COLUMN` statements for one changed column
    /// (drizzle-kit `alter_column`): type (enum-to-enum through text, serial
    /// pseudo-types expanded into their sequence, default dropped and put
    /// back around an enum conversion), NOT NULL, default, generated
    /// expression, identity, collation and comment.
    #[allow(clippy::too_many_lines)]
    fn alter_column_sql(
        from: &Column,
        to: &Column,
        was_enum: bool,
        is_enum: bool,
        diff: &HashMap<String, serde_json::Value>,
    ) -> Vec<String> {
        let table_key = Self::qualified_name(&to.schema, &to.table);
        let column = Self::quote_ident(&to.name);
        let alter =
            |clause: &str| format!("ALTER TABLE {table_key} ALTER COLUMN {column} {clause};");
        let mut stmts = Vec::new();

        let type_changed = diff.contains_key("type");
        let collate_changed = diff.contains_key("collate");
        // PostgreSQL cannot cast a default across enum types: drop it,
        // convert, then set the new one.
        let recreate_default = type_changed && (is_enum || was_enum) && from.default.is_some();
        if recreate_default {
            stmts.push(alter("DROP DEFAULT"));
        }

        if type_changed || collate_changed {
            let from_serial = Self::serial_base_type(&from.sql_type);
            let to_serial = Self::serial_base_type(&to.sql_type);
            let mut target = to.clone();
            if let Some(base) = to_serial {
                target.sql_type = base.into();
            }
            let type_sql = Self::column_type_sql(&target);
            let from_base = from_serial.map_or_else(
                || super::collection::normalize_column_type_for_compare(from),
                ToString::to_string,
            );
            let base_changed = type_changed
                && (from_base != super::collection::normalize_column_type_for_compare(&target)
                    || was_enum != is_enum
                    || diff.contains_key("typeSchema"));

            match (from_serial, to_serial) {
                // serial -> plain integer: the column stops owning its
                // sequence.
                (Some(_), None) => {
                    if !recreate_default {
                        stmts.push(alter("DROP DEFAULT"));
                    }
                    stmts.push(format!(
                        "DROP SEQUENCE {};",
                        Self::serial_sequence_name(from)
                    ));
                }
                // plain integer -> serial: create and attach the sequence.
                (None, Some(base)) => {
                    let sequence = Self::serial_sequence_name(to);
                    stmts.push(format!("CREATE SEQUENCE {sequence} AS {base};"));
                    stmts.push(alter(&format!(
                        "SET DEFAULT nextval({})",
                        Self::quote_literal(&sequence)
                    )));
                    stmts.push(format!(
                        "ALTER SEQUENCE {sequence} OWNED BY {table_key}.{column};"
                    ));
                }
                _ => {}
            }

            if base_changed || collate_changed {
                let collate = if collate_changed {
                    format!(
                        " COLLATE {}",
                        Self::quote_ident(to.collate.as_deref().unwrap_or("default"))
                    )
                } else {
                    String::new()
                };
                // A generated column's values come from its expression;
                // PostgreSQL rejects USING for it.
                let using = if !base_changed || to.generated.is_some() {
                    String::new()
                } else if was_enum && is_enum {
                    format!(" USING {column}::text::{type_sql}")
                } else {
                    format!(" USING {column}::{type_sql}")
                };
                stmts.push(alter(&format!("SET DATA TYPE {type_sql}{collate}{using}")));
            }

            // serial <-> bigserial/smallserial: the owned sequence follows
            // the column's integer width.
            if let (Some(old_base), Some(new_base)) = (from_serial, to_serial)
                && old_base != new_base
            {
                stmts.push(format!(
                    "ALTER SEQUENCE {} AS {new_base};",
                    Self::serial_sequence_name(to)
                ));
            }

            if recreate_default && let Some(default) = &to.default {
                stmts.push(alter(&format!("SET DEFAULT {default}")));
            }
        }

        if diff.contains_key("notNull") {
            stmts.push(alter(if to.not_null {
                "SET NOT NULL"
            } else {
                "DROP NOT NULL"
            }));
        }

        if diff.contains_key("default") && !recreate_default {
            if let Some(default) = &to.default {
                stmts.push(alter(&format!("SET DEFAULT {default}")));
            } else if !(type_changed && Self::serial_base_type(&from.sql_type).is_some()) {
                // (A serial column's default went with its sequence above.)
                stmts.push(alter("DROP DEFAULT"));
            }
        }

        if diff.contains_key("generated") && to.generated.is_none() {
            stmts.push(alter("DROP EXPRESSION"));
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
                        stmts.push(alter(&format!("ADD{identity_sql}")));
                    }
                }
                // Some -> Some: ALTER COLUMN ... SET GENERATED / SET <option>.
                // `ADD GENERATED` on an existing identity column is invalid.
                (Some(_), Some(_)) => {
                    // Compare with unset options filled in: an option left
                    // at its default is not a change.
                    let normalized = super::collection::normalize_identity_options;
                    if let (Some(new), Some(old)) = (normalized(to), normalized(from))
                        && let Some(set_sql) = Self::identity_set_options_sql(&new, &old)
                    {
                        stmts.push(alter(&set_sql));
                    }
                }
                // Some -> None: DROP IDENTITY.
                (None, _) => stmts.push(alter("DROP IDENTITY")),
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

        stmts
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

/// Topological sort tables for CREATE: referenced tables come first. Ties
/// keep the order of `table_keys` (the schema's declaration order), so the
/// output is deterministic.
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

    let table_set: HashSet<&String> = table_keys.iter().collect();

    // table -> tables it references (which must be created first)
    let mut dependencies: HashMap<&str, HashSet<String>> = HashMap::new();
    for d in diff
        .iter()
        .filter(|d| d.kind == EntityKind::ForeignKey && d.diff_type == DiffType::Create)
    {
        if let Some(PostgresEntity::ForeignKey(fk)) = &d.right {
            let from_table = format!("{}.{}", fk.schema, fk.table);
            let to_table = format!("{}.{}", fk.schema_to, fk.table_to);
            if from_table != to_table
                && let (Some(from), true) =
                    (table_set.get(&from_table), table_set.contains(&to_table))
            {
                dependencies
                    .entry(from.as_str())
                    .or_default()
                    .insert(to_table);
            }
        }
    }

    let mut result = Vec::new();
    let mut remaining: Vec<&String> = table_keys.iter().collect();
    let mut satisfied: HashSet<String> = HashSet::new();
    let mut cycle_tables = HashSet::new();

    while !remaining.is_empty() {
        let (ready, blocked): (Vec<&String>, Vec<&String>) = remaining.iter().partition(|t| {
            dependencies
                .get(t.as_str())
                .is_none_or(|deps| deps.iter().all(|d| satisfied.contains(d)))
        });

        if ready.is_empty() {
            // Circular dependency: create remaining tables without their cycle FKs,
            // then add those constraints after all tables exist.
            cycle_tables = blocked.iter().map(|t| (*t).clone()).collect();
            result.extend(blocked.into_iter().cloned());
            break;
        }

        for t in ready {
            satisfied.insert(t.clone());
            result.push(t.clone());
        }
        remaining = blocked;
    }

    CreateTableOrder {
        ordered: result,
        cycle_tables,
    }
}

/// Topological sort tables for DROP: tables with FKs come first (reverse of create)
fn topological_sort_tables_for_drop(table_keys: &[String], diff: &[EntityDiff]) -> Vec<String> {
    // Dropped tables' FKs only appear as drop diffs; treat them as the
    // dependencies.
    let as_created: Vec<EntityDiff> = diff
        .iter()
        .filter(|d| d.kind == EntityKind::ForeignKey && d.diff_type == DiffType::Drop)
        .map(|d| EntityDiff {
            diff_type: DiffType::Create,
            kind: d.kind,
            name: d.name.clone(),
            changes: HashMap::new(),
            left: None,
            right: d.left.clone(),
        })
        .collect();
    let create_order = topological_sort_tables_for_create(table_keys, &as_created);
    create_order.ordered.into_iter().rev().collect()
}

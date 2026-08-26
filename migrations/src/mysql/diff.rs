//! Deterministic MySQL schema diffing.
//!
//! The planner emits dependency phases instead of relying on entity insertion
//! order. It rejects database-scope changes and never invents database DDL.

use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet};

use drizzle_types::mysql::ddl as model;
use thiserror::Error;

use super::collection::MySQLDDL;
use super::statements::{
    CheckDefinition, ColumnDefinition, ColumnType, ForeignKeyDefinition, GeneratedDefinition,
    GeneratedKind, IndexAlgorithm, IndexColumnDefinition, IndexDefinition, IndexLock, IndexUsing,
    MySQLStatement, PrimaryKeyDefinition, ReferentialAction, RenderError, SortOrder,
    TableDefinition, UniqueDefinition, ViewAlgorithm, ViewCheckOption, ViewDefinition,
    ViewSecurity, render_statements,
};

/// Explicit table rename within the selected database.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TableRename {
    pub database: Option<String>,
    pub from: String,
    pub to: String,
}

/// Explicit column rename within the selected database.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColumnRename {
    pub database: Option<String>,
    pub table: String,
    pub from: String,
    pub to: String,
}

/// Explicit view rename within the selected database.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ViewRename {
    pub database: Option<String>,
    pub from: String,
    pub to: String,
}

/// MySQL-specific rename inputs. The planner never guesses a rename.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RenameHints {
    pub tables: Vec<TableRename>,
    pub columns: Vec<ColumnRename>,
    pub views: Vec<ViewRename>,
}

impl RenameHints {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn table(mut self, from: impl Into<String>, to: impl Into<String>) -> Self {
        self.tables.push(TableRename {
            database: None,
            from: from.into(),
            to: to.into(),
        });
        self
    }

    #[must_use]
    pub fn column(
        mut self,
        table: impl Into<String>,
        from: impl Into<String>,
        to: impl Into<String>,
    ) -> Self {
        self.columns.push(ColumnRename {
            database: None,
            table: table.into(),
            from: from.into(),
            to: to.into(),
        });
        self
    }

    #[must_use]
    pub fn view(mut self, from: impl Into<String>, to: impl Into<String>) -> Self {
        self.views.push(ViewRename {
            database: None,
            from: from.into(),
            to: to.into(),
        });
        self
    }
}

/// Options for the standalone MySQL planner.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DiffOptions {
    pub renames: RenameHints,
    pub strict_renames: bool,
}

/// A structural migration warning which does not require live row counts.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum MySQLWarning {
    DropTable {
        table: String,
    },
    DropColumn {
        table: String,
        column: String,
    },
    RecreateColumn {
        table: String,
        column: String,
    },
    ChangeGeneratedColumn {
        table: String,
        column: String,
    },
    ChangeColumnType {
        table: String,
        column: String,
    },
    TightenNullability {
        table: String,
        column: String,
    },
    RemoveOrReorderInlineValues {
        table: String,
        column: String,
    },
    ChangeCharsetOrCollation {
        table: String,
        column: Option<String>,
    },
    DropConstraint {
        table: String,
        kind: &'static str,
        name: String,
    },
    DropView {
        view: String,
    },
}

impl std::fmt::Display for MySQLWarning {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DropTable { table } => {
                write!(formatter, "dropping table {table:?} can lose data")
            }
            Self::DropColumn { table, column } => {
                write!(formatter, "dropping column {table}.{column} can lose data")
            }
            Self::RecreateColumn { table, column } => write!(
                formatter,
                "recreating column {table}.{column} uses separate DROP and ADD statements and loses stored values"
            ),
            Self::ChangeGeneratedColumn { table, column } => write!(
                formatter,
                "changing generated column {table}.{column} can recompute stored values"
            ),
            Self::ChangeColumnType { table, column } => write!(
                formatter,
                "changing the type of {table}.{column} can truncate or reject existing values"
            ),
            Self::TightenNullability { table, column } => write!(
                formatter,
                "making {table}.{column} NOT NULL can fail when rows contain NULL"
            ),
            Self::RemoveOrReorderInlineValues { table, column } => write!(
                formatter,
                "removing or reordering inline enum/set values on {table}.{column} can remap or reject stored values"
            ),
            Self::ChangeCharsetOrCollation { table, column } => {
                if let Some(column) = column {
                    write!(
                        formatter,
                        "changing charset or collation on {table}.{column} can recode data and change uniqueness"
                    )
                } else {
                    write!(
                        formatter,
                        "changing charset or collation on table {table:?} can recode data and change uniqueness"
                    )
                }
            }
            Self::DropConstraint { table, kind, name } => {
                write!(
                    formatter,
                    "dropping {kind} {table}.{name} removes data integrity enforcement"
                )
            }
            Self::DropView { view } => {
                write!(formatter, "dropping view {view:?} removes its database API")
            }
        }
    }
}

/// Successful MySQL migration plan.
#[derive(Debug, Clone, Default)]
pub struct MigrationDiff {
    pub statements: Vec<MySQLStatement>,
    pub sql_statements: Vec<String>,
    pub renames: Vec<String>,
    pub typed_warnings: Vec<MySQLWarning>,
    pub warnings: Vec<String>,
}

/// MySQL schema planning failure.
#[derive(Debug, Error)]
pub enum DiffError {
    #[error("MySQL snapshot contains more than one explicit database: {databases:?}")]
    MultipleDatabases { databases: Vec<String> },
    #[error("MySQL migration cannot change database scope from {from:?} to {to:?}")]
    DatabaseScopeChange {
        from: Option<String>,
        to: Option<String>,
    },
    #[error("MySQL migration cannot reference database {foreign:?} from scope {selected:?}")]
    CrossDatabaseReference {
        selected: Option<String>,
        foreign: Option<String>,
    },
    #[error(
        "MySQL foreign key {name:?} references columns without an eligible index on table {table:?}"
    )]
    NonUniqueForeignKeyTarget { name: String, table: String },
    #[error("invalid MySQL rename hint: {0}")]
    Rename(String),
    #[error("cannot remove explicit MySQL table option {option} from table {table:?}")]
    CannotUnsetTableOption { table: String, option: &'static str },
    #[error("MySQL cannot alter TEMPORARY status for existing table {table:?}")]
    TemporaryTableAlter { table: String },
    #[error("MySQL table {table:?} contains unsupported changed options: {options:?}")]
    UnsupportedTableOptions { table: String, options: Vec<String> },
    #[error("MySQL view dependency cycle among {views:?}")]
    ViewDependencyCycle { views: Vec<String> },
    #[error(transparent)]
    Validation(#[from] super::collection::ValidationError),
    #[error(transparent)]
    Render(#[from] RenderError),
}

/// Required SQL strategy for an existing column.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColumnAlterStrategy {
    Modify,
    Recreate,
}

/// Implements MySQL's generated-column ALTER transition matrix.
#[must_use]
pub const fn generated_alter_strategy(
    old: Option<GeneratedKind>,
    new: Option<GeneratedKind>,
) -> ColumnAlterStrategy {
    match (old, new) {
        (Some(GeneratedKind::Virtual), Some(GeneratedKind::Virtual))
        | (Some(GeneratedKind::Stored), Some(GeneratedKind::Stored))
        | (None, None)
        | (None, Some(GeneratedKind::Stored))
        | (Some(GeneratedKind::Stored), None) => ColumnAlterStrategy::Modify,
        (None, Some(GeneratedKind::Virtual))
        | (Some(GeneratedKind::Virtual), None)
        | (Some(GeneratedKind::Virtual), Some(GeneratedKind::Stored))
        | (Some(GeneratedKind::Stored), Some(GeneratedKind::Virtual)) => {
            ColumnAlterStrategy::Recreate
        }
    }
}

fn database(database: &Option<Cow<'static, str>>) -> Option<String> {
    database.as_deref().map(str::to_string)
}

fn declared_database(ddl: &MySQLDDL) -> Result<Option<String>, DiffError> {
    let mut databases = BTreeSet::new();
    macro_rules! collect {
        ($collection:expr) => {
            databases.extend(
                $collection
                    .list()
                    .iter()
                    .filter_map(|entity| entity.database.as_deref().map(str::to_string)),
            );
        };
    }
    collect!(ddl.tables);
    collect!(ddl.columns);
    collect!(ddl.indexes);
    collect!(ddl.fks);
    collect!(ddl.pks);
    collect!(ddl.uniques);
    collect!(ddl.checks);
    collect!(ddl.views);
    match databases.len() {
        0 => Ok(None),
        1 => Ok(databases.into_iter().next()),
        _ => Err(DiffError::MultipleDatabases {
            databases: databases.into_iter().collect(),
        }),
    }
}

fn selected_database(prev: &MySQLDDL, cur: &MySQLDDL) -> Result<Option<String>, DiffError> {
    let prev_is_empty = prev.is_empty();
    let cur_is_empty = cur.is_empty();
    let prev = declared_database(prev)?;
    let cur = declared_database(cur)?;
    if prev_is_empty {
        return Ok(cur);
    }
    if cur_is_empty {
        return Ok(prev);
    }
    if prev != cur {
        return Err(DiffError::DatabaseScopeChange {
            from: prev,
            to: cur,
        });
    }
    Ok(cur)
}

fn effective_database(explicit: Option<&str>, selected: Option<&str>) -> Option<String> {
    explicit.or(selected).map(str::to_string)
}

fn validate_foreign_key_scope(ddl: &MySQLDDL, selected: Option<&str>) -> Result<(), DiffError> {
    for foreign_key in ddl.fks.list() {
        let local = effective_database(foreign_key.database.as_deref(), selected);
        let foreign = effective_database(foreign_key.foreign_database.as_deref(), selected);
        if local != foreign {
            return Err(DiffError::CrossDatabaseReference {
                selected: local,
                foreign,
            });
        }
    }
    Ok(())
}

fn named_columns_have_prefix<'a>(
    available: impl IntoIterator<Item = &'a Cow<'static, str>>,
    required: impl IntoIterator<Item = &'a str>,
) -> bool {
    let mut available = available.into_iter();
    required
        .into_iter()
        .all(|required| available.next().is_some_and(|column| column == required))
}

fn index_supports_columns<'a>(
    index: &model::Index,
    required: impl IntoIterator<Item = &'a str>,
) -> bool {
    let mut columns = index.columns.iter();
    required.into_iter().all(|required| {
        columns.next().is_some_and(|column| {
            !column.is_expression && column.length.is_none() && column.expression == required
        })
    })
}

fn validate_foreign_key_targets(ddl: &MySQLDDL) -> Result<(), DiffError> {
    for foreign_key in ddl.fks.list() {
        let target: Vec<&str> = foreign_key
            .foreign_columns
            .iter()
            .map(AsRef::as_ref)
            .collect();
        let primary_key_matches = ddl.pks.list().iter().any(|primary_key| {
            primary_key.table == foreign_key.foreign_table
                && named_columns_have_prefix(primary_key.columns.iter(), target.iter().copied())
        }) || (target.len() == 1
            && ddl.columns.list().iter().any(|column| {
                column.table == foreign_key.foreign_table
                    && column.name == target[0]
                    && column.primary_key
            }));
        let unique_matches = ddl.uniques.list().iter().any(|unique| {
            unique.table == foreign_key.foreign_table
                && named_columns_have_prefix(unique.columns.iter(), target.iter().copied())
        }) || ddl.indexes.list().iter().any(|index| {
            index.table == foreign_key.foreign_table
                && index_supports_columns(index, target.iter().copied())
        }) || (target.len() == 1
            && ddl.columns.list().iter().any(|column| {
                column.table == foreign_key.foreign_table
                    && column.name == target[0]
                    && column.unique
            }));
        if !primary_key_matches && !unique_matches {
            return Err(DiffError::NonUniqueForeignKeyTarget {
                name: foreign_key.name.to_string(),
                table: foreign_key.foreign_table.to_string(),
            });
        }
    }
    Ok(())
}

fn hint_database_matches(hint: Option<&str>, selected: Option<&str>) -> bool {
    hint.is_none() || effective_database(hint, selected) == selected.map(str::to_string)
}

fn apply_rename_hints(
    prev: &mut MySQLDDL,
    cur: &MySQLDDL,
    selected: Option<&str>,
    options: &DiffOptions,
) -> Result<(Vec<MySQLStatement>, Vec<String>), DiffError> {
    let mut statements = Vec::new();
    let mut tracked = Vec::new();

    for hint in &options.renames.tables {
        if !hint_database_matches(hint.database.as_deref(), selected)
            || hint.from.is_empty()
            || hint.to.is_empty()
            || hint.from == hint.to
        {
            if options.strict_renames {
                return Err(DiffError::Rename(format!(
                    "table {:?} -> {:?}",
                    hint.from, hint.to
                )));
            }
            continue;
        }
        let matches = prev
            .tables
            .list()
            .iter()
            .any(|table| table.name == hint.from)
            && cur.tables.list().iter().any(|table| table.name == hint.to)
            && !prev.tables.list().iter().any(|table| table.name == hint.to);
        if !matches {
            if options.strict_renames {
                return Err(DiffError::Rename(format!(
                    "table {:?} -> {:?} did not match snapshots",
                    hint.from, hint.to
                )));
            }
            continue;
        }
        let from = hint.from.as_str();
        let to = hint.to.as_str();
        for table in prev.tables.list_mut() {
            if table.name == from {
                table.name = Cow::Owned(to.to_string());
            }
        }
        for column in prev.columns.list_mut() {
            if column.table == from {
                column.table = Cow::Owned(to.to_string());
            }
        }
        for index in prev.indexes.list_mut() {
            if index.table == from {
                index.table = Cow::Owned(to.to_string());
            }
        }
        for primary_key in prev.pks.list_mut() {
            if primary_key.table == from {
                primary_key.table = Cow::Owned(to.to_string());
            }
        }
        for unique in prev.uniques.list_mut() {
            if unique.table == from {
                unique.table = Cow::Owned(to.to_string());
            }
        }
        for check in prev.checks.list_mut() {
            if check.table == from {
                check.table = Cow::Owned(to.to_string());
            }
        }
        for foreign_key in prev.fks.list_mut() {
            if foreign_key.table == from {
                foreign_key.table = Cow::Owned(to.to_string());
            }
            if foreign_key.foreign_table == from
                && effective_database(foreign_key.foreign_database.as_deref(), selected)
                    == selected.map(str::to_string)
            {
                foreign_key.foreign_table = Cow::Owned(to.to_string());
            }
        }
        statements.push(MySQLStatement::RenameTable {
            database: selected.map(str::to_string),
            from: hint.from.clone(),
            to: hint.to.clone(),
        });
        tracked.push(format!("table:{}:{}", hint.from, hint.to));
    }

    for hint in &options.renames.columns {
        if !hint_database_matches(hint.database.as_deref(), selected)
            || hint.table.is_empty()
            || hint.from.is_empty()
            || hint.to.is_empty()
            || hint.from == hint.to
        {
            if options.strict_renames {
                return Err(DiffError::Rename(format!(
                    "column {}.{:?} -> {:?}",
                    hint.table, hint.from, hint.to
                )));
            }
            continue;
        }
        let matches = prev
            .columns
            .list()
            .iter()
            .any(|column| column.table == hint.table && column.name == hint.from)
            && cur
                .columns
                .list()
                .iter()
                .any(|column| column.table == hint.table && column.name == hint.to)
            && !prev
                .columns
                .list()
                .iter()
                .any(|column| column.table == hint.table && column.name == hint.to);
        if !matches {
            if options.strict_renames {
                return Err(DiffError::Rename(format!(
                    "column {}.{:?} -> {:?} did not match snapshots",
                    hint.table, hint.from, hint.to
                )));
            }
            continue;
        }
        for column in prev.columns.list_mut() {
            if column.table == hint.table && column.name == hint.from {
                column.name = Cow::Owned(hint.to.clone());
            }
        }
        for index in prev.indexes.list_mut() {
            if index.table == hint.table {
                for column in &mut index.columns {
                    if !column.is_expression && column.expression == hint.from {
                        column.expression = Cow::Owned(hint.to.clone());
                    }
                }
            }
        }
        for primary_key in prev.pks.list_mut() {
            if primary_key.table == hint.table {
                replace_name(&mut primary_key.columns, &hint.from, &hint.to);
            }
        }
        for unique in prev.uniques.list_mut() {
            if unique.table == hint.table {
                replace_name(&mut unique.columns, &hint.from, &hint.to);
            }
        }
        for foreign_key in prev.fks.list_mut() {
            if foreign_key.table == hint.table {
                replace_name(&mut foreign_key.columns, &hint.from, &hint.to);
            }
            if foreign_key.foreign_table == hint.table {
                replace_name(&mut foreign_key.foreign_columns, &hint.from, &hint.to);
            }
        }
        statements.push(MySQLStatement::RenameColumn {
            database: selected.map(str::to_string),
            table: hint.table.clone(),
            from: hint.from.clone(),
            to: hint.to.clone(),
        });
        tracked.push(format!("column:{}:{}:{}", hint.table, hint.from, hint.to));
    }

    for hint in &options.renames.views {
        if !hint_database_matches(hint.database.as_deref(), selected)
            || hint.from.is_empty()
            || hint.to.is_empty()
            || hint.from == hint.to
        {
            if options.strict_renames {
                return Err(DiffError::Rename(format!(
                    "view {:?} -> {:?}",
                    hint.from, hint.to
                )));
            }
            continue;
        }
        let matches = prev.views.list().iter().any(|view| view.name == hint.from)
            && cur.views.list().iter().any(|view| view.name == hint.to)
            && !prev.views.list().iter().any(|view| view.name == hint.to);
        if !matches {
            if options.strict_renames {
                return Err(DiffError::Rename(format!(
                    "view {:?} -> {:?} did not match snapshots",
                    hint.from, hint.to
                )));
            }
            continue;
        }
        for view in prev.views.list_mut() {
            if view.name == hint.from {
                view.name = Cow::Owned(hint.to.clone());
            }
        }
        statements.push(MySQLStatement::RenameView {
            database: selected.map(str::to_string),
            from: hint.from.clone(),
            to: hint.to.clone(),
        });
        tracked.push(format!("view:{}:{}", hint.from, hint.to));
    }

    Ok((statements, tracked))
}

fn replace_name(values: &mut [Cow<'static, str>], from: &str, to: &str) {
    for value in values {
        if value == from {
            *value = Cow::Owned(to.to_string());
        }
    }
}

fn generated_kind(generated: Option<&model::Generated>) -> Option<GeneratedKind> {
    generated.map(|generated| match generated.generation_type {
        model::GeneratedType::Virtual => GeneratedKind::Virtual,
        model::GeneratedType::Stored => GeneratedKind::Stored,
    })
}

fn column_type(column: &model::Column) -> ColumnType {
    match &column.inline_type {
        Some(model::InlineType::Enum(values)) => ColumnType::InlineEnum {
            values: values.values.iter().map(ToString::to_string).collect(),
        },
        Some(model::InlineType::Set(values)) => ColumnType::InlineSet {
            values: values.values.iter().map(ToString::to_string).collect(),
        },
        None => ColumnType::Sql {
            sql: column.sql_type.to_string(),
        },
    }
}

fn column_definition(column: &model::Column) -> ColumnDefinition {
    ColumnDefinition {
        name: column.name.to_string(),
        column_type: column_type(column),
        not_null: column.not_null,
        auto_increment: column.autoincrement,
        primary_key: column.primary_key,
        unique: column.unique,
        default: column.default.as_deref().map(str::to_string),
        on_update: column.on_update.as_deref().map(str::to_string),
        charset: column.charset.as_deref().map(str::to_string),
        collation: column.collation.as_deref().map(str::to_string),
        generated: column
            .generated
            .as_ref()
            .map(|generated| GeneratedDefinition {
                expression: generated.expression.to_string(),
                kind: generated_kind(Some(generated)).expect("generated value has a storage kind"),
            }),
        comment: column.comment.as_deref().map(str::to_string),
    }
}

fn column_definition_for_ddl(column: &model::Column, ddl: &MySQLDDL) -> ColumnDefinition {
    let mut definition = column_definition(column);
    if ddl.pks.list().iter().any(|primary_key| {
        primary_key.table == column.table
            && primary_key.columns.iter().any(|name| name == &column.name)
    }) {
        definition.primary_key = false;
    }
    if ddl.uniques.list().iter().any(|unique| {
        unique.table == column.table
            && unique.columns.len() == 1
            && unique.columns[0] == column.name
    }) {
        definition.unique = false;
    }
    definition
}

fn index_column(column: &model::IndexColumn) -> IndexColumnDefinition {
    let order = column.ascending.map(|ascending| {
        if ascending {
            SortOrder::Asc
        } else {
            SortOrder::Desc
        }
    });
    if column.is_expression {
        IndexColumnDefinition::Expression {
            sql: column.expression.to_string(),
            order,
        }
    } else {
        IndexColumnDefinition::Column {
            name: column.expression.to_string(),
            length: column.length,
            order,
        }
    }
}

fn index_definition(index: &model::Index) -> IndexDefinition {
    IndexDefinition {
        database: database(&index.database),
        table: index.table.to_string(),
        name: index.name.to_string(),
        columns: index.columns.iter().map(index_column).collect(),
        unique: index.unique,
        using: index.using.map(|using| match using {
            model::IndexMethod::Btree => IndexUsing::Btree,
            model::IndexMethod::Hash => IndexUsing::Hash,
        }),
        algorithm: index.algorithm.map(|algorithm| match algorithm {
            model::IndexAlgorithm::Default => IndexAlgorithm::Default,
            model::IndexAlgorithm::Inplace => IndexAlgorithm::Inplace,
            model::IndexAlgorithm::Copy => IndexAlgorithm::Copy,
        }),
        lock: index.lock.map(|lock| match lock {
            model::IndexLock::Default => IndexLock::Default,
            model::IndexLock::None => IndexLock::None,
            model::IndexLock::Shared => IndexLock::Shared,
            model::IndexLock::Exclusive => IndexLock::Exclusive,
        }),
        comment: index.comment.as_deref().map(str::to_string),
        visible: index.visible,
    }
}

fn primary_key_definition(primary_key: &model::PrimaryKey) -> PrimaryKeyDefinition {
    PrimaryKeyDefinition {
        database: database(&primary_key.database),
        table: primary_key.table.to_string(),
        columns: primary_key
            .columns
            .iter()
            .map(ToString::to_string)
            .collect(),
    }
}

fn unique_definition(unique: &model::UniqueConstraint) -> UniqueDefinition {
    UniqueDefinition {
        database: database(&unique.database),
        table: unique.table.to_string(),
        name: unique.name.to_string(),
        columns: unique
            .columns
            .iter()
            .map(|column| IndexColumnDefinition::Column {
                name: column.to_string(),
                length: None,
                order: None,
            })
            .collect(),
    }
}

fn referential_action(action: model::ReferentialAction) -> ReferentialAction {
    match action {
        model::ReferentialAction::Cascade => ReferentialAction::Cascade,
        model::ReferentialAction::SetNull => ReferentialAction::SetNull,
        model::ReferentialAction::Restrict => ReferentialAction::Restrict,
        model::ReferentialAction::NoAction => ReferentialAction::NoAction,
    }
}

fn foreign_key_definition(foreign_key: &model::ForeignKey) -> ForeignKeyDefinition {
    let local_database = database(&foreign_key.database);
    let referenced_database =
        database(&foreign_key.foreign_database).or_else(|| local_database.clone());
    ForeignKeyDefinition {
        database: local_database,
        table: foreign_key.table.to_string(),
        name: foreign_key.name.to_string(),
        columns: foreign_key
            .columns
            .iter()
            .map(ToString::to_string)
            .collect(),
        referenced_database,
        referenced_table: foreign_key.foreign_table.to_string(),
        referenced_columns: foreign_key
            .foreign_columns
            .iter()
            .map(ToString::to_string)
            .collect(),
        on_delete: foreign_key.on_delete.map(referential_action),
        on_update: foreign_key.on_update.map(referential_action),
    }
}

fn check_definition(check: &model::CheckConstraint) -> CheckDefinition {
    CheckDefinition {
        database: database(&check.database),
        table: check.table.to_string(),
        name: check.name.to_string(),
        expression: check.expression.to_string(),
        enforced: check.enforced,
    }
}

fn view_definition(view: &model::View) -> Option<ViewDefinition> {
    if view.is_existing {
        return None;
    }
    Some(ViewDefinition {
        database: database(&view.database),
        name: view.name.to_string(),
        definition: view.definition.as_deref()?.to_string(),
        algorithm: view.algorithm.map(|algorithm| match algorithm {
            model::ViewAlgorithm::Undefined => ViewAlgorithm::Undefined,
            model::ViewAlgorithm::Merge => ViewAlgorithm::Merge,
            model::ViewAlgorithm::Temptable => ViewAlgorithm::Temptable,
        }),
        definer: view.definer.as_deref().map(str::to_string),
        security: view.sql_security.map(|security| match security {
            model::ViewSqlSecurity::Definer => ViewSecurity::Definer,
            model::ViewSqlSecurity::Invoker => ViewSecurity::Invoker,
        }),
        check_option: view.check_option.map(|option| match option {
            model::ViewCheckOption::Cascaded => ViewCheckOption::Cascaded,
            model::ViewCheckOption::Local => ViewCheckOption::Local,
        }),
    })
}

fn views_equivalent(left: &model::View, right: &model::View) -> bool {
    view_definition(left) == view_definition(right)
}

fn table_definition(table: &model::Table, ddl: &MySQLDDL) -> TableDefinition {
    let columns: Vec<_> = ddl
        .columns
        .list()
        .iter()
        .filter(|column| column.table == table.name)
        .map(|column| column_definition_for_ddl(column, ddl))
        .collect();
    let primary_key = ddl
        .pks
        .list()
        .iter()
        .find(|primary_key| primary_key.table == table.name)
        .map(primary_key_definition);
    let mut uniques: Vec<_> = ddl
        .uniques
        .list()
        .iter()
        .filter(|unique| unique.table == table.name)
        .map(unique_definition)
        .collect();
    uniques.sort_by(|left, right| left.name.cmp(&right.name));
    let mut checks: Vec<_> = ddl
        .checks
        .list()
        .iter()
        .filter(|check| check.table == table.name)
        .map(check_definition)
        .collect();
    checks.sort_by(|left, right| left.name.cmp(&right.name));
    TableDefinition {
        database: database(&table.database),
        name: table.name.to_string(),
        temporary: table.temporary,
        columns,
        primary_key,
        uniques,
        checks,
        engine: table.engine.as_deref().map(str::to_string),
        charset: table.charset.as_deref().map(str::to_string),
        collation: table.collation.as_deref().map(str::to_string),
        comment: table.comment.as_deref().map(str::to_string),
    }
}

fn table_map(ddl: &MySQLDDL) -> BTreeMap<String, &model::Table> {
    ddl.tables
        .list()
        .iter()
        .map(|table| (table.name.to_string(), table))
        .collect()
}

fn column_map(ddl: &MySQLDDL) -> BTreeMap<(String, String), &model::Column> {
    ddl.columns
        .list()
        .iter()
        .map(|column| ((column.table.to_string(), column.name.to_string()), column))
        .collect()
}

macro_rules! named_table_map {
    ($name:ident, $field:ident, $type:ty) => {
        fn $name(ddl: &MySQLDDL) -> BTreeMap<(String, String), &$type> {
            ddl.$field
                .list()
                .iter()
                .map(|entity| ((entity.table.to_string(), entity.name.to_string()), entity))
                .collect()
        }
    };
}

named_table_map!(index_map, indexes, model::Index);
named_table_map!(foreign_key_map, fks, model::ForeignKey);
named_table_map!(unique_map, uniques, model::UniqueConstraint);
named_table_map!(check_map, checks, model::CheckConstraint);

fn primary_key_map(ddl: &MySQLDDL) -> BTreeMap<String, &model::PrimaryKey> {
    ddl.pks
        .list()
        .iter()
        .map(|primary_key| (primary_key.table.to_string(), primary_key))
        .collect()
}

fn view_map(ddl: &MySQLDDL) -> BTreeMap<String, &model::View> {
    ddl.views
        .list()
        .iter()
        .map(|view| (view.name.to_string(), view))
        .collect()
}

fn inline_values(column: &model::Column) -> Option<Vec<&str>> {
    match column.inline_type.as_ref()? {
        model::InlineType::Enum(values) | model::InlineType::Set(values) => {
            Some(values.values.iter().map(AsRef::as_ref).collect())
        }
    }
}

fn collect_column_warnings(
    warnings: &mut BTreeSet<MySQLWarning>,
    old: &model::Column,
    new: &model::Column,
) {
    let table = new.table.to_string();
    let column = new.name.to_string();
    if old.sql_type != new.sql_type || old.inline_type != new.inline_type {
        warnings.insert(MySQLWarning::ChangeColumnType {
            table: table.clone(),
            column: column.clone(),
        });
    }
    if !old.not_null && new.not_null {
        warnings.insert(MySQLWarning::TightenNullability {
            table: table.clone(),
            column: column.clone(),
        });
    }
    if old.charset != new.charset || old.collation != new.collation {
        warnings.insert(MySQLWarning::ChangeCharsetOrCollation {
            table: table.clone(),
            column: Some(column.clone()),
        });
    }
    if old.generated != new.generated {
        warnings.insert(MySQLWarning::ChangeGeneratedColumn {
            table: table.clone(),
            column: column.clone(),
        });
    }
    let inline_shape_changed = match (&old.inline_type, &new.inline_type) {
        (Some(old), Some(new)) => std::mem::discriminant(old) != std::mem::discriminant(new),
        (Some(_), None) => true,
        _ => false,
    };
    let inline_values_changed = matches!(
        (inline_values(old), inline_values(new)),
        (Some(old), Some(new))
            if new.len() < old.len() || !old.iter().zip(&new).all(|(old, new)| old == new)
    );
    if inline_shape_changed || inline_values_changed {
        warnings.insert(MySQLWarning::RemoveOrReorderInlineValues { table, column });
    }
}

fn depends_on_recreated_column(
    table: &str,
    columns: impl IntoIterator<Item = String>,
    recreated: &BTreeSet<(String, String)>,
) -> bool {
    columns
        .into_iter()
        .any(|column| recreated.contains(&(table.to_string(), column)))
}

fn foreign_key_touches_columns(
    foreign_key: &model::ForeignKey,
    columns: &BTreeSet<(String, String)>,
) -> bool {
    foreign_key
        .columns
        .iter()
        .any(|column| columns.contains(&(foreign_key.table.to_string(), column.to_string())))
        || foreign_key.foreign_columns.iter().any(|column| {
            columns.contains(&(foreign_key.foreign_table.to_string(), column.to_string()))
        })
}

fn foreign_key_uses_index(foreign_key: &model::ForeignKey, index: &model::Index) -> bool {
    (index.table == foreign_key.table
        && index_supports_columns(index, foreign_key.columns.iter().map(AsRef::as_ref)))
        || (index.table == foreign_key.foreign_table
            && index_supports_columns(index, foreign_key.foreign_columns.iter().map(AsRef::as_ref)))
}

fn generated_dependents_of_renames(
    ddl: &MySQLDDL,
    renames: &[(String, String, String)],
) -> BTreeSet<(String, String)> {
    let mut changed_by_table: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for (table, from, _) in renames {
        changed_by_table
            .entry(table.clone())
            .or_default()
            .insert(from.clone());
    }

    let mut dependents = BTreeSet::new();
    loop {
        let newly_dependent: Vec<_> = ddl
            .columns
            .list()
            .iter()
            .filter_map(|column| {
                let generated = column.generated.as_ref()?;
                let changed = changed_by_table.get(column.table.as_ref())?;
                let key = (column.table.to_string(), column.name.to_string());
                (!dependents.contains(&key)
                    && !identifier_tokens(&generated.expression).is_disjoint(changed))
                .then_some(key)
            })
            .collect();
        if newly_dependent.is_empty() {
            return dependents;
        }
        for key in newly_dependent {
            changed_by_table
                .entry(key.0.clone())
                .or_default()
                .insert(key.1.clone());
            dependents.insert(key);
        }
    }
}

fn rewrite_identifier(sql: &str, from: &str, to: &str) -> String {
    let characters: Vec<_> = sql.chars().collect();
    let mut rewritten = String::with_capacity(sql.len());
    let mut position = 0;
    while position < characters.len() {
        let character = characters[position];
        if character == '\'' || character == '"' {
            let quote = character;
            rewritten.push(character);
            position += 1;
            while position < characters.len() {
                let character = characters[position];
                rewritten.push(character);
                position += 1;
                if character == '\\' && position < characters.len() {
                    rewritten.push(characters[position]);
                    position += 1;
                } else if character == quote {
                    if position < characters.len() && characters[position] == quote {
                        rewritten.push(characters[position]);
                        position += 1;
                    } else {
                        break;
                    }
                }
            }
        } else if character == '`' {
            let start = position;
            position += 1;
            let identifier_start = position;
            while position < characters.len() && characters[position] != '`' {
                position += 1;
            }
            let identifier: String = characters[identifier_start..position].iter().collect();
            if identifier == from {
                rewritten.push('`');
                rewritten.push_str(&to.replace('`', "``"));
                rewritten.push('`');
            } else {
                rewritten.extend(&characters[start..position.min(characters.len() - 1) + 1]);
            }
            if position < characters.len() {
                position += 1;
            }
        } else if character.is_ascii_alphanumeric() || character == '_' || character == '$' {
            let start = position;
            position += 1;
            while position < characters.len()
                && (characters[position].is_ascii_alphanumeric()
                    || characters[position] == '_'
                    || characters[position] == '$')
            {
                position += 1;
            }
            let identifier: String = characters[start..position].iter().collect();
            rewritten.push_str(if identifier == from { to } else { &identifier });
        } else {
            rewritten.push(character);
            position += 1;
        }
    }
    rewritten
}

fn rewrite_column_definition(
    mut definition: ColumnDefinition,
    table: &str,
    renames: &[(String, String, String)],
) -> ColumnDefinition {
    if let Some(generated) = &mut definition.generated {
        for (_, from, to) in renames
            .iter()
            .filter(|(rename_table, _, _)| rename_table == table)
        {
            generated.expression = rewrite_identifier(&generated.expression, from, to);
        }
    }
    definition
}

fn rewrite_check_definition(
    mut definition: CheckDefinition,
    renames: &[(String, String, String)],
) -> CheckDefinition {
    for (_, from, to) in renames
        .iter()
        .filter(|(table, _, _)| table == &definition.table)
    {
        definition.expression = rewrite_identifier(&definition.expression, from, to);
    }
    definition
}

fn flush_identifier(tokens: &mut BTreeSet<String>, token: &mut String) {
    if !token.is_empty() {
        tokens.insert(std::mem::take(token));
    }
}

fn identifier_tokens(sql: &str) -> BTreeSet<String> {
    #[derive(Clone, Copy)]
    enum State {
        Sql,
        QuotedIdentifier,
        SingleQuotedString,
        DoubleQuotedString,
        LineComment,
        BlockComment,
    }

    let mut tokens = BTreeSet::new();
    let mut token = String::new();
    let characters: Vec<_> = sql.chars().collect();
    let mut state = State::Sql;
    let mut index = 0;

    while index < characters.len() {
        let character = characters[index];
        match state {
            State::Sql => match character {
                '`' => {
                    flush_identifier(&mut tokens, &mut token);
                    state = State::QuotedIdentifier;
                }
                '\'' => {
                    flush_identifier(&mut tokens, &mut token);
                    state = State::SingleQuotedString;
                }
                '"' => {
                    flush_identifier(&mut tokens, &mut token);
                    state = State::DoubleQuotedString;
                }
                '#' => {
                    flush_identifier(&mut tokens, &mut token);
                    state = State::LineComment;
                }
                '-' if characters.get(index + 1) == Some(&'-')
                    && characters
                        .get(index + 2)
                        .is_none_or(|next| next.is_whitespace() || next.is_control()) =>
                {
                    flush_identifier(&mut tokens, &mut token);
                    state = State::LineComment;
                    index += 1;
                }
                '/' if characters.get(index + 1) == Some(&'*') => {
                    flush_identifier(&mut tokens, &mut token);
                    state = State::BlockComment;
                    index += 1;
                }
                character if character.is_alphanumeric() || matches!(character, '_' | '$') => {
                    token.push(character);
                }
                _ => flush_identifier(&mut tokens, &mut token),
            },
            State::QuotedIdentifier => {
                if character == '`' {
                    if characters.get(index + 1) == Some(&'`') {
                        token.push('`');
                        index += 1;
                    } else {
                        flush_identifier(&mut tokens, &mut token);
                        state = State::Sql;
                    }
                } else {
                    token.push(character);
                }
            }
            State::SingleQuotedString | State::DoubleQuotedString => {
                let quote = match state {
                    State::SingleQuotedString => '\'',
                    State::DoubleQuotedString => '"',
                    _ => unreachable!(),
                };
                if character == '\\' {
                    index += usize::from(index + 1 < characters.len());
                } else if character == quote {
                    if characters.get(index + 1) == Some(&quote) {
                        index += 1;
                    } else {
                        state = State::Sql;
                    }
                }
            }
            State::LineComment => {
                if matches!(character, '\n' | '\r') {
                    state = State::Sql;
                }
            }
            State::BlockComment => {
                if character == '*' && characters.get(index + 1) == Some(&'/') {
                    state = State::Sql;
                    index += 1;
                }
            }
        }
        index += 1;
    }
    flush_identifier(&mut tokens, &mut token);
    tokens
}

fn order_views(views: Vec<&model::View>) -> Result<Vec<&model::View>, DiffError> {
    let pending_names: BTreeSet<_> = views.iter().map(|view| view.name.to_string()).collect();
    let mut dependencies: BTreeMap<String, BTreeSet<String>> = views
        .iter()
        .map(|view| {
            let tokens = view
                .definition
                .as_deref()
                .map(identifier_tokens)
                .unwrap_or_default();
            let deps = pending_names
                .iter()
                .filter(|name| name.as_str() != view.name.as_ref() && tokens.contains(*name))
                .cloned()
                .collect();
            (view.name.to_string(), deps)
        })
        .collect();
    let by_name: BTreeMap<_, _> = views
        .into_iter()
        .map(|view| (view.name.to_string(), view))
        .collect();
    let mut ordered = Vec::new();
    while !dependencies.is_empty() {
        let ready: Vec<_> = dependencies
            .iter()
            .filter(|(_, dependencies)| dependencies.is_empty())
            .map(|(name, _)| name.clone())
            .collect();
        if ready.is_empty() {
            return Err(DiffError::ViewDependencyCycle {
                views: dependencies.into_keys().collect(),
            });
        }
        for name in ready {
            dependencies.remove(&name);
            for remaining in dependencies.values_mut() {
                remaining.remove(&name);
            }
            if let Some(view) = by_name.get(&name) {
                ordered.push(*view);
            }
        }
    }
    Ok(ordered)
}

/// Computes a MySQL migration without rename inference.
pub fn compute_migration(prev: &MySQLDDL, cur: &MySQLDDL) -> Result<MigrationDiff, DiffError> {
    compute_migration_with(prev, cur, &DiffOptions::default())
}

/// Computes a deterministic, dependency-phased MySQL migration.
pub fn compute_migration_with(
    prev: &MySQLDDL,
    cur: &MySQLDDL,
    options: &DiffOptions,
) -> Result<MigrationDiff, DiffError> {
    let mut prev = MySQLDDL::try_from_entities(prev.to_entities())?;
    let cur = MySQLDDL::try_from_entities(cur.to_entities())?;
    let selected = selected_database(&prev, &cur)?;
    validate_foreign_key_scope(&prev, selected.as_deref())?;
    validate_foreign_key_scope(&cur, selected.as_deref())?;
    validate_foreign_key_targets(&cur)?;

    let (rename_statements, renames) =
        apply_rename_hints(&mut prev, &cur, selected.as_deref(), options)?;
    let renamed_columns: Vec<_> = rename_statements
        .iter()
        .filter_map(|statement| match statement {
            MySQLStatement::RenameColumn {
                table, from, to, ..
            } => Some((table.clone(), from.clone(), to.clone())),
            _ => None,
        })
        .collect();
    let (column_rename_statements, rename_statements): (Vec<_>, Vec<_>) = rename_statements
        .into_iter()
        .partition(|statement| matches!(statement, MySQLStatement::RenameColumn { .. }));
    let prev_tables = table_map(&prev);
    let cur_tables = table_map(&cur);
    let prev_columns = column_map(&prev);
    let cur_columns = column_map(&cur);
    let prev_indexes = index_map(&prev);
    let cur_indexes = index_map(&cur);
    let prev_fks = foreign_key_map(&prev);
    let cur_fks = foreign_key_map(&cur);
    let prev_pks = primary_key_map(&prev);
    let cur_pks = primary_key_map(&cur);
    let prev_uniques = unique_map(&prev);
    let cur_uniques = unique_map(&cur);
    let prev_checks = check_map(&prev);
    let cur_checks = check_map(&cur);
    let prev_views = view_map(&prev);
    let cur_views = view_map(&cur);

    let created_tables: BTreeSet<_> = cur_tables
        .keys()
        .filter(|name| !prev_tables.contains_key(*name))
        .cloned()
        .collect();
    let dropped_tables: BTreeSet<_> = prev_tables
        .keys()
        .filter(|name| !cur_tables.contains_key(*name))
        .cloned()
        .collect();

    let mut warnings = BTreeSet::new();
    for table in &dropped_tables {
        warnings.insert(MySQLWarning::DropTable {
            table: table.clone(),
        });
    }

    let altered_columns: Vec<_> = prev_columns
        .iter()
        .filter_map(|(key, old)| {
            cur_columns
                .get(key)
                .filter(|new| *old != **new)
                .map(|new| (key.clone(), *old, *new))
        })
        .collect();
    let rename_generated_dependents = generated_dependents_of_renames(&prev, &renamed_columns);
    let mut recreated_columns: BTreeSet<_> = altered_columns
        .iter()
        .filter(|(_, old, new)| {
            generated_alter_strategy(
                generated_kind(old.generated.as_ref()),
                generated_kind(new.generated.as_ref()),
            ) == ColumnAlterStrategy::Recreate
        })
        .map(|(key, _, _)| key.clone())
        .collect();
    recreated_columns.extend(rename_generated_dependents.iter().cloned());
    for (key, old, new) in &altered_columns {
        collect_column_warnings(&mut warnings, old, new);
        if recreated_columns.contains(key) {
            warnings.insert(MySQLWarning::RecreateColumn {
                table: key.0.clone(),
                column: key.1.clone(),
            });
        }
    }
    for (table, column) in &rename_generated_dependents {
        warnings.insert(MySQLWarning::RecreateColumn {
            table: table.clone(),
            column: column.clone(),
        });
    }

    let dropped_columns: BTreeSet<_> = prev_columns
        .keys()
        .filter(|key| !cur_columns.contains_key(*key) && !dropped_tables.contains(&key.0))
        .cloned()
        .collect();
    for (table, column) in &dropped_columns {
        warnings.insert(MySQLWarning::DropColumn {
            table: table.clone(),
            column: column.clone(),
        });
    }

    let changed_columns: BTreeSet<_> = altered_columns
        .iter()
        .map(|(key, _, _)| key.clone())
        .chain(dropped_columns.iter().cloned())
        .chain(
            renamed_columns
                .iter()
                .map(|(table, _, to)| (table.clone(), to.clone())),
        )
        .chain(rename_generated_dependents.iter().cloned())
        .collect();
    let changed_column_tables: BTreeSet<_> = changed_columns
        .iter()
        .map(|(table, _)| table.clone())
        .collect();

    let drop_indexes: BTreeSet<_> = prev_indexes
        .iter()
        .filter(|(key, old)| {
            !dropped_tables.contains(&key.0)
                && (cur_indexes.get(*key).is_none_or(|new| **old != *new)
                    || old.columns.iter().any(|column| {
                        !column.is_expression
                            && recreated_columns
                                .contains(&(key.0.clone(), column.expression.to_string()))
                    }))
        })
        .map(|(key, _)| key.clone())
        .collect();

    let altered_key_tables: BTreeSet<_> = prev_pks
        .iter()
        .filter(|(table, old)| cur_pks.get(*table).is_none_or(|new| **old != *new))
        .map(|(table, _)| table.clone())
        .chain(prev_uniques.iter().filter_map(|(key, old)| {
            cur_uniques
                .get(key)
                .filter(|new| *old != **new)
                .map(|_| key.0.clone())
        }))
        .chain(
            prev_uniques
                .keys()
                .filter(|key| !cur_uniques.contains_key(*key))
                .map(|key| key.0.clone()),
        )
        .collect();

    let affected_fk_tables: BTreeSet<_> = altered_key_tables
        .iter()
        .cloned()
        .chain(dropped_tables.iter().cloned())
        .collect();
    let drop_fk_keys: BTreeSet<_> = prev_fks
        .iter()
        .filter(|(key, old)| {
            cur_fks.get(*key).is_none_or(|new| **old != *new)
                || affected_fk_tables.contains(old.table.as_ref())
                || affected_fk_tables.contains(old.foreign_table.as_ref())
                || foreign_key_touches_columns(old, &changed_columns)
                || drop_indexes
                    .iter()
                    .filter_map(|key| prev_indexes.get(key))
                    .any(|index| foreign_key_uses_index(old, index))
        })
        .map(|(key, _)| key.clone())
        .collect();
    let mut add_fk_keys: BTreeSet<_> = cur_fks
        .iter()
        .filter(|(key, new)| {
            prev_fks.get(*key).is_none_or(|old| *old != **new)
                || affected_fk_tables.contains(new.table.as_ref())
                || affected_fk_tables.contains(new.foreign_table.as_ref())
                || foreign_key_touches_columns(new, &changed_columns)
                || drop_indexes
                    .iter()
                    .filter_map(|key| prev_indexes.get(key))
                    .any(|index| foreign_key_uses_index(new, index))
        })
        .map(|(key, _)| key.clone())
        .collect();
    add_fk_keys.retain(|key| {
        cur_fks.get(key).is_some_and(|foreign_key| {
            !dropped_tables.contains(foreign_key.table.as_ref())
                && !dropped_tables.contains(foreign_key.foreign_table.as_ref())
        })
    });

    let mut statements = rename_statements;

    let mut drop_view_names: BTreeSet<_> = prev_views
        .iter()
        .filter(|(name, old)| {
            !old.is_existing
                && cur_views
                    .get(*name)
                    .is_none_or(|new| !new.is_existing && !views_equivalent(old, new))
        })
        .map(|(name, _)| name.clone())
        .collect();
    let mut changed_identifiers: BTreeSet<_> = dropped_tables
        .iter()
        .cloned()
        .chain(dropped_columns.iter().map(|(table, _)| table.clone()))
        .chain(recreated_columns.iter().map(|(table, _)| table.clone()))
        .chain(drop_view_names.iter().cloned())
        .collect();
    loop {
        let newly_dependent: Vec<_> = prev_views
            .iter()
            .filter(|(name, view)| {
                !drop_view_names.contains(*name)
                    && !view.is_existing
                    && cur_views
                        .get(*name)
                        .is_none_or(|current| !current.is_existing)
                    && view.definition.as_deref().is_some_and(|definition| {
                        !identifier_tokens(definition).is_disjoint(&changed_identifiers)
                    })
            })
            .map(|(name, _)| name.clone())
            .collect();
        if newly_dependent.is_empty() {
            break;
        }
        for name in newly_dependent {
            changed_identifiers.insert(name.clone());
            drop_view_names.insert(name);
        }
    }
    let mut views_to_drop = order_views(
        drop_view_names
            .iter()
            .filter_map(|name| prev_views.get(name).copied())
            .collect(),
    )?;
    views_to_drop.reverse();
    for view in views_to_drop {
        let name = view.name.to_string();
        statements.push(MySQLStatement::DropView {
            database: database(&view.database),
            view: name.clone(),
        });
        if !cur_views.contains_key(&name) {
            warnings.insert(MySQLWarning::DropView { view: name });
        }
    }

    for key in &drop_fk_keys {
        if let Some(foreign_key) = prev_fks.get(key) {
            statements.push(MySQLStatement::DropForeignKey {
                database: database(&foreign_key.database),
                table: foreign_key.table.to_string(),
                name: foreign_key.name.to_string(),
            });
            warnings.insert(MySQLWarning::DropConstraint {
                table: key.0.clone(),
                kind: "foreign key",
                name: key.1.clone(),
            });
        }
    }

    let drop_checks: BTreeSet<_> = prev_checks
        .iter()
        .filter(|(key, old)| {
            !dropped_tables.contains(&key.0)
                && (cur_checks.get(*key).is_none_or(|new| **old != *new)
                    || changed_column_tables.contains(&key.0))
        })
        .map(|(key, _)| key.clone())
        .collect();
    for key in &drop_checks {
        let check = prev_checks[key];
        statements.push(MySQLStatement::DropCheck {
            database: database(&check.database),
            table: key.0.clone(),
            name: key.1.clone(),
        });
        warnings.insert(MySQLWarning::DropConstraint {
            table: key.0.clone(),
            kind: "check constraint",
            name: key.1.clone(),
        });
    }

    for key in &drop_indexes {
        let index = prev_indexes[key];
        statements.push(MySQLStatement::DropIndex {
            database: database(&index.database),
            table: key.0.clone(),
            name: key.1.clone(),
        });
        warnings.insert(MySQLWarning::DropConstraint {
            table: key.0.clone(),
            kind: "index",
            name: key.1.clone(),
        });
    }

    let drop_uniques: BTreeSet<_> = prev_uniques
        .iter()
        .filter(|(key, old)| {
            !dropped_tables.contains(&key.0)
                && (cur_uniques.get(*key).is_none_or(|new| **old != *new)
                    || depends_on_recreated_column(
                        &key.0,
                        old.columns.iter().map(ToString::to_string),
                        &recreated_columns,
                    ))
        })
        .map(|(key, _)| key.clone())
        .collect();
    for key in &drop_uniques {
        let unique = prev_uniques[key];
        statements.push(MySQLStatement::DropUnique {
            database: database(&unique.database),
            table: key.0.clone(),
            name: key.1.clone(),
        });
        warnings.insert(MySQLWarning::DropConstraint {
            table: key.0.clone(),
            kind: "unique constraint",
            name: key.1.clone(),
        });
    }

    let drop_pks: BTreeSet<_> = prev_pks
        .iter()
        .filter(|(table, old)| {
            !dropped_tables.contains(*table)
                && (cur_pks.get(*table).is_none_or(|new| **old != *new)
                    || depends_on_recreated_column(
                        table,
                        old.columns.iter().map(ToString::to_string),
                        &recreated_columns,
                    ))
        })
        .map(|(table, _)| table.clone())
        .collect();
    for table in &drop_pks {
        let primary_key = prev_pks[table];
        statements.push(MySQLStatement::DropPrimaryKey {
            database: database(&primary_key.database),
            table: table.clone(),
        });
        warnings.insert(MySQLWarning::DropConstraint {
            table: table.clone(),
            kind: "primary key",
            name: "PRIMARY".to_string(),
        });
    }

    for column in prev.columns.list().iter().rev().filter(|column| {
        rename_generated_dependents.contains(&(column.table.to_string(), column.name.to_string()))
    }) {
        statements.push(MySQLStatement::DropColumn {
            database: database(&column.database),
            table: column.table.to_string(),
            column: column.name.to_string(),
        });
    }
    statements.extend(column_rename_statements);

    for (table, column) in &dropped_columns {
        if rename_generated_dependents.contains(&(table.clone(), column.clone())) {
            continue;
        }
        let old = prev_columns[&(table.clone(), column.clone())];
        statements.push(MySQLStatement::DropColumn {
            database: database(&old.database),
            table: table.clone(),
            column: column.clone(),
        });
    }
    for table in &dropped_tables {
        let old = prev_tables[table];
        statements.push(MySQLStatement::DropTable {
            database: database(&old.database),
            table: table.clone(),
        });
    }

    for table in &created_tables {
        if !cur_tables[table].options.is_empty() {
            return Err(DiffError::UnsupportedTableOptions {
                table: table.clone(),
                options: cur_tables[table]
                    .options
                    .iter()
                    .map(|option| option.name.to_string())
                    .collect(),
            });
        }
        statements.push(MySQLStatement::CreateTable {
            table: table_definition(cur_tables[table], &cur),
        });
    }

    for (name, old) in &prev_tables {
        let Some(new) = cur_tables.get(name) else {
            continue;
        };
        if created_tables.contains(name) || *old == *new {
            continue;
        }
        if old.temporary != new.temporary {
            return Err(DiffError::TemporaryTableAlter {
                table: name.clone(),
            });
        }
        if old.options != new.options {
            return Err(DiffError::UnsupportedTableOptions {
                table: name.clone(),
                options: new
                    .options
                    .iter()
                    .map(|option| option.name.to_string())
                    .collect(),
            });
        }
        if old.engine.is_some() && new.engine.is_none() {
            return Err(DiffError::CannotUnsetTableOption {
                table: name.clone(),
                option: "engine",
            });
        }
        if old.collation.is_some() && new.collation.is_none() && old.charset == new.charset {
            return Err(DiffError::CannotUnsetTableOption {
                table: name.clone(),
                option: "collation without also resetting character set",
            });
        }
        if old.charset != new.charset || old.collation != new.collation {
            warnings.insert(MySQLWarning::ChangeCharsetOrCollation {
                table: name.clone(),
                column: None,
            });
        }
        statements.push(MySQLStatement::AlterTableOptions {
            database: database(&new.database),
            table: name.clone(),
            engine: (old.engine != new.engine)
                .then(|| new.engine.as_deref().map(str::to_string))
                .flatten(),
            charset: (old.charset != new.charset).then(|| {
                new.charset
                    .as_deref()
                    .map_or_else(|| "DEFAULT".to_string(), str::to_string)
            }),
            collation: (old.collation != new.collation)
                .then(|| new.collation.as_deref().map(str::to_string))
                .flatten(),
            comment: (old.comment != new.comment).then(|| {
                new.comment
                    .as_deref()
                    .map_or_else(String::new, str::to_string)
            }),
        });
    }

    for column in cur.columns.list() {
        let key = (column.table.to_string(), column.name.to_string());
        if created_tables.contains(&key.0) || prev_columns.contains_key(&key) {
            continue;
        }
        statements.push(MySQLStatement::AddColumn {
            database: database(&column.database),
            table: key.0.clone(),
            column: rewrite_column_definition(
                column_definition_for_ddl(column, &cur),
                &key.0,
                &renamed_columns,
            ),
        });
        if column.not_null && column.default.is_none() && column.generated.is_none() {
            warnings.insert(MySQLWarning::TightenNullability {
                table: key.0.clone(),
                column: key.1.clone(),
            });
        }
    }
    for column in cur.columns.list().iter().filter(|column| {
        rename_generated_dependents.contains(&(column.table.to_string(), column.name.to_string()))
    }) {
        statements.push(MySQLStatement::AddColumn {
            database: database(&column.database),
            table: column.table.to_string(),
            column: rewrite_column_definition(
                column_definition_for_ddl(column, &cur),
                column.table.as_ref(),
                &renamed_columns,
            ),
        });
    }
    for new in cur.columns.list() {
        let key = (new.table.to_string(), new.name.to_string());
        if !altered_columns
            .iter()
            .any(|(altered_key, _, _)| altered_key == &key)
        {
            continue;
        }
        if dropped_tables.contains(&key.0) {
            continue;
        }
        if rename_generated_dependents.contains(&key) {
            continue;
        }
        let definition = rewrite_column_definition(
            column_definition_for_ddl(new, &cur),
            &key.0,
            &renamed_columns,
        );
        let statement = if recreated_columns.contains(&key) {
            MySQLStatement::RecreateColumn {
                database: database(&new.database),
                table: key.0.clone(),
                column: definition,
            }
        } else {
            MySQLStatement::ModifyColumn {
                database: database(&new.database),
                table: key.0.clone(),
                column: definition,
            }
        };
        statements.push(statement);
    }

    for (table, primary_key) in &cur_pks {
        if created_tables.contains(table) {
            continue;
        }
        if !prev_pks.contains_key(table) || drop_pks.contains(table) {
            statements.push(MySQLStatement::AddPrimaryKey {
                primary_key: primary_key_definition(primary_key),
            });
        }
    }
    for (key, unique) in &cur_uniques {
        if created_tables.contains(&key.0) {
            continue;
        }
        if !prev_uniques.contains_key(key) || drop_uniques.contains(key) {
            statements.push(MySQLStatement::AddUnique {
                unique: unique_definition(unique),
            });
        }
    }
    for (key, index) in &cur_indexes {
        if !prev_indexes.contains_key(key) || drop_indexes.contains(key) {
            statements.push(MySQLStatement::CreateIndex {
                index: index_definition(index),
            });
        }
    }
    for (key, check) in &cur_checks {
        if created_tables.contains(&key.0) {
            continue;
        }
        if !prev_checks.contains_key(key) || drop_checks.contains(key) {
            statements.push(MySQLStatement::AddCheck {
                check: rewrite_check_definition(check_definition(check), &renamed_columns),
            });
        }
    }
    for key in add_fk_keys {
        if let Some(foreign_key) = cur_fks.get(&key) {
            statements.push(MySQLStatement::AddForeignKey {
                foreign_key: foreign_key_definition(foreign_key),
            });
        }
    }

    let views_to_create: Vec<_> = cur_views
        .iter()
        .filter(|(name, new)| {
            drop_view_names.contains(*name)
                || prev_views
                    .get(*name)
                    .is_none_or(|old| !views_equivalent(old, new))
        })
        .map(|(_, view)| *view)
        .collect();
    for view in order_views(views_to_create)? {
        if prev_views
            .get(view.name.as_ref())
            .is_some_and(|previous| previous.is_existing)
        {
            continue;
        }
        if let Some(view) = view_definition(view) {
            statements.push(MySQLStatement::CreateView { view });
        }
    }

    let sql_statements = render_statements(&statements)?;
    let typed_warnings: Vec<_> = warnings.into_iter().collect();
    let warnings = typed_warnings.iter().map(ToString::to_string).collect();
    Ok(MigrationDiff {
        statements,
        sql_statements,
        renames,
        typed_warnings,
        warnings,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn table_with_columns(name: &str, columns: &[&str]) -> MySQLDDL {
        let mut ddl = MySQLDDL::new();
        ddl.tables.push(model::Table::new(name.to_string()));
        for column in columns {
            let mut definition =
                model::Column::new(name.to_string(), (*column).to_string(), "bigint");
            definition.not_null = true;
            ddl.columns.push(definition);
        }
        ddl
    }

    fn schema_with_indexed_foreign_key() -> MySQLDDL {
        let mut ddl = table_with_columns("parent", &["id", "tenant_id"]);
        let child = table_with_columns("child", &["id", "parent_id"]);
        ddl.tables.extend(child.tables.clone().into_vec());
        ddl.columns.extend(child.columns.clone().into_vec());
        ddl.indexes.push(model::Index::new(
            "parent",
            "parent_lookup",
            vec![
                model::IndexColumn::column("id"),
                model::IndexColumn::column("tenant_id"),
            ],
        ));
        ddl.indexes.push(model::Index::new(
            "child",
            "child_parent_lookup",
            vec![model::IndexColumn::column("parent_id")],
        ));
        ddl.fks.push(model::ForeignKey::new(
            "child",
            "child_parent_fk",
            ["parent_id"],
            "parent",
            ["id"],
        ));
        ddl
    }

    #[test]
    fn generated_transition_matrix_matches_mysql_alter_rules() {
        use ColumnAlterStrategy::{Modify, Recreate};
        use GeneratedKind::{Stored, Virtual};

        assert_eq!(generated_alter_strategy(None, None), Modify);
        assert_eq!(
            generated_alter_strategy(Some(Virtual), Some(Virtual)),
            Modify
        );
        assert_eq!(generated_alter_strategy(Some(Stored), Some(Stored)), Modify);
        assert_eq!(generated_alter_strategy(None, Some(Stored)), Modify);
        assert_eq!(generated_alter_strategy(Some(Stored), None), Modify);
        assert_eq!(generated_alter_strategy(None, Some(Virtual)), Recreate);
        assert_eq!(generated_alter_strategy(Some(Virtual), None), Recreate);
        assert_eq!(
            generated_alter_strategy(Some(Virtual), Some(Stored)),
            Recreate
        );
        assert_eq!(
            generated_alter_strategy(Some(Stored), Some(Virtual)),
            Recreate
        );
    }

    #[test]
    fn view_dependency_order_is_stable() {
        let base = model::View::new("base", "select 1 as id");
        let dependent = model::View::new("dependent", "select id from `base`");
        let ordered = order_views(vec![&dependent, &base]).unwrap();
        assert_eq!(
            ordered
                .iter()
                .map(|view| view.name.as_ref())
                .collect::<Vec<_>>(),
            ["base", "dependent"]
        );
    }

    #[test]
    fn view_dependency_cycles_are_rejected() {
        let a = model::View::new("a", "select * from b");
        let b = model::View::new("b", "select * from a");
        assert!(matches!(
            order_views(vec![&a, &b]),
            Err(DiffError::ViewDependencyCycle { .. })
        ));
    }

    #[test]
    fn view_dependency_scanning_ignores_strings_and_comments() {
        let a = model::View::new("a", "select 'b' as label /* b */ -- b\n# b\nfrom source_a");
        let b = model::View::new("b", "select \"a\" as label /* a */ from source_b");

        let ordered = order_views(vec![&b, &a]).unwrap();
        assert_eq!(
            ordered
                .iter()
                .map(|view| view.name.as_ref())
                .collect::<Vec<_>>(),
            ["a", "b"]
        );
    }

    #[test]
    fn view_dependency_scanning_unescapes_quoted_identifiers() {
        let base = model::View::new("z`base", "select 1 as id");
        let dependent = model::View::new("a_dependent", "select id from `z``base`");

        let ordered = order_views(vec![&dependent, &base]).unwrap();
        assert_eq!(
            ordered
                .iter()
                .map(|view| view.name.as_ref())
                .collect::<Vec<_>>(),
            ["z`base", "a_dependent"]
        );
    }

    #[test]
    fn existing_views_never_emit_create_or_drop_statements() {
        let mut existing = model::View::new("external_users", "select id from users");
        existing.is_existing = true;
        let mut existing_schema = MySQLDDL::new();
        existing_schema.views.push(existing);

        let mut managed_schema = MySQLDDL::new();
        managed_schema
            .views
            .push(model::View::new("external_users", "select id from users"));

        let empty = MySQLDDL::new();
        assert!(
            compute_migration(&empty, &existing_schema)
                .unwrap()
                .sql_statements
                .is_empty()
        );
        assert!(
            compute_migration(&existing_schema, &empty)
                .unwrap()
                .sql_statements
                .is_empty()
        );
        assert!(
            compute_migration(&existing_schema, &managed_schema)
                .unwrap()
                .sql_statements
                .is_empty()
        );
        assert!(
            compute_migration(&managed_schema, &existing_schema)
                .unwrap()
                .sql_statements
                .is_empty()
        );
    }

    #[test]
    fn existing_views_are_not_dropped_with_changed_dependencies() {
        let mut prev = table_with_columns("users", &["id"]);
        let mut existing = model::View::new("external_users", "select id from users");
        existing.is_existing = true;
        prev.views.push(existing);

        let migration = compute_migration(&prev, &MySQLDDL::new()).unwrap();
        assert!(
            migration
                .statements
                .iter()
                .all(|statement| !matches!(statement, MySQLStatement::DropView { .. }))
        );
    }

    #[test]
    fn existing_views_still_participate_in_dependency_validation() {
        let mut existing = model::View::new("external", "select * from managed");
        existing.is_existing = true;
        let managed = model::View::new("managed", "select * from external");
        let mut cur = MySQLDDL::new();
        cur.views.push(existing);
        cur.views.push(managed);

        assert!(matches!(
            compute_migration(&MySQLDDL::new(), &cur),
            Err(DiffError::ViewDependencyCycle { .. })
        ));
    }

    #[test]
    fn informational_view_metadata_does_not_emit_ineffective_ddl() {
        let mut prev = MySQLDDL::new();
        let mut view = model::View::new("active_users", "select 1");
        view.charset = Some("latin1".into());
        view.collation = Some("latin1_swedish_ci".into());
        prev.views.push(view);

        let mut cur = prev.clone();
        cur.views.list_mut()[0].charset = Some("utf8mb4".into());
        cur.views.list_mut()[0].collation = Some("utf8mb4_0900_ai_ci".into());
        let migration = compute_migration(&prev, &cur).unwrap();
        assert!(migration.sql_statements.is_empty());

        cur.views.list_mut()[0].algorithm = Some(model::ViewAlgorithm::Merge);
        let migration = compute_migration(&prev, &cur).unwrap();
        assert_eq!(migration.sql_statements.len(), 2);
        assert_eq!(migration.sql_statements[0], "DROP VIEW `active_users`;");
        assert!(migration.sql_statements[1].contains("ALGORITHM=MERGE"));
    }

    #[test]
    fn explicit_rename_hints_emit_only_renames() {
        let prev = table_with_columns("users", &["id"]);
        let cur = table_with_columns("accounts", &["user_id"]);
        let options = DiffOptions {
            renames: RenameHints::new()
                .table("users", "accounts")
                .column("accounts", "id", "user_id"),
            strict_renames: true,
        };

        let migration = compute_migration_with(&prev, &cur, &options).unwrap();

        assert_eq!(
            migration.sql_statements,
            [
                "RENAME TABLE `users` TO `accounts`;",
                "ALTER TABLE `accounts` RENAME COLUMN `id` TO `user_id`;",
            ]
        );
        assert_eq!(
            migration.renames,
            ["table:users:accounts", "column:accounts:id:user_id"]
        );
    }

    #[test]
    fn primary_key_change_drops_referencing_fk_before_key_and_restores_it_after() {
        let mut prev = table_with_columns("parent", &["id", "tenant_id"]);
        let child = table_with_columns("child", &["id", "parent_id"]);
        prev.tables.extend(child.tables.clone().into_vec());
        prev.columns.extend(child.columns.clone().into_vec());
        prev.pks.push(model::PrimaryKey {
            database: None,
            table: "parent".into(),
            name: None,
            columns: vec!["id".into()],
        });
        prev.uniques.push(model::UniqueConstraint {
            database: None,
            table: "parent".into(),
            name: "parent_id_unique".into(),
            columns: vec!["id".into()],
        });
        prev.fks.push(model::ForeignKey {
            database: None,
            table: "child".into(),
            name: "child_parent_fk".into(),
            columns: vec!["parent_id".into()],
            foreign_database: None,
            foreign_table: "parent".into(),
            foreign_columns: vec!["id".into()],
            on_delete: Some(model::ReferentialAction::Cascade),
            on_update: None,
        });
        let mut cur = prev.clone();
        cur.pks.list_mut()[0].columns.push("tenant_id".into());

        let migration = compute_migration(&prev, &cur).unwrap();
        let drop_fk = migration
            .statements
            .iter()
            .position(|statement| matches!(statement, MySQLStatement::DropForeignKey { .. }))
            .unwrap();
        let drop_pk = migration
            .statements
            .iter()
            .position(|statement| matches!(statement, MySQLStatement::DropPrimaryKey { .. }))
            .unwrap();
        let add_pk = migration
            .statements
            .iter()
            .position(|statement| matches!(statement, MySQLStatement::AddPrimaryKey { .. }))
            .unwrap();
        let add_fk = migration
            .statements
            .iter()
            .position(|statement| matches!(statement, MySQLStatement::AddForeignKey { .. }))
            .unwrap();

        assert!(drop_fk < drop_pk && drop_pk < add_pk && add_pk < add_fk);
        assert!(
            migration
                .sql_statements
                .iter()
                .all(|sql| !sql.contains("BEGIN"))
        );
        assert!(
            migration
                .sql_statements
                .iter()
                .all(|sql| !sql.contains("COMMIT"))
        );
    }

    #[test]
    fn foreign_key_target_without_eligible_index_is_rejected() {
        let mut cur = table_with_columns("parent", &["id"]);
        let child = table_with_columns("child", &["parent_id"]);
        cur.tables.extend(child.tables.clone().into_vec());
        cur.columns.extend(child.columns.clone().into_vec());
        cur.fks.push(model::ForeignKey {
            database: None,
            table: "child".into(),
            name: "child_parent_fk".into(),
            columns: vec!["parent_id".into()],
            foreign_database: None,
            foreign_table: "parent".into(),
            foreign_columns: vec!["id".into()],
            on_delete: None,
            on_update: None,
        });

        assert!(matches!(
            compute_migration(&MySQLDDL::new(), &cur),
            Err(DiffError::NonUniqueForeignKeyTarget { .. })
        ));
    }

    #[test]
    fn database_scope_change_is_rejected_without_database_ddl() {
        let mut prev = table_with_columns("users", &["id"]);
        prev.tables.list_mut()[0].database = Some("one".into());
        prev.columns.list_mut()[0].database = Some("one".into());
        let mut cur = table_with_columns("users", &["id"]);
        cur.tables.list_mut()[0].database = Some("two".into());
        cur.columns.list_mut()[0].database = Some("two".into());

        assert!(matches!(
            compute_migration(&prev, &cur),
            Err(DiffError::DatabaseScopeChange { .. })
        ));
    }

    #[test]
    fn empty_origin_adopts_first_schema_database_scope() {
        let prev = MySQLDDL::new();
        let mut cur = table_with_columns("users", &["id"]);
        cur.tables.list_mut()[0].database = Some("app".into());
        cur.columns.list_mut()[0].database = Some("app".into());

        let migration = compute_migration(&prev, &cur).unwrap();

        assert_eq!(migration.sql_statements.len(), 1);
        assert!(migration.sql_statements[0].starts_with("CREATE TABLE `app`.`users`"));
        assert!(
            migration
                .sql_statements
                .iter()
                .all(|sql| !sql.contains("DATABASE"))
        );
    }

    #[test]
    fn enum_removal_and_charset_changes_emit_structural_warnings() {
        let mut prev = table_with_columns("users", &["status"]);
        prev.tables.list_mut()[0].charset = Some("latin1".into());
        prev.tables.list_mut()[0].collation = Some("latin1_swedish_ci".into());
        prev.columns.list_mut()[0].inline_type =
            Some(model::InlineType::Enum(model::InlineEnum::new([
                "new", "active", "disabled",
            ])));
        prev.columns.list_mut()[0].charset = Some("latin1".into());
        prev.columns.list_mut()[0].collation = Some("latin1_swedish_ci".into());
        let mut cur = prev.clone();
        cur.tables.list_mut()[0].charset = Some("utf8mb4".into());
        cur.tables.list_mut()[0].collation = Some("utf8mb4_0900_ai_ci".into());
        cur.columns.list_mut()[0].inline_type =
            Some(model::InlineType::Enum(model::InlineEnum::new([
                "new", "disabled",
            ])));
        cur.columns.list_mut()[0].charset = Some("utf8mb4".into());
        cur.columns.list_mut()[0].collation = Some("utf8mb4_0900_ai_ci".into());

        let migration = compute_migration(&prev, &cur).unwrap();

        assert!(
            migration
                .typed_warnings
                .contains(&MySQLWarning::RemoveOrReorderInlineValues {
                    table: "users".to_string(),
                    column: "status".to_string(),
                })
        );
        assert!(
            migration
                .typed_warnings
                .contains(&MySQLWarning::ChangeCharsetOrCollation {
                    table: "users".to_string(),
                    column: Some("status".to_string()),
                })
        );
        assert!(
            migration
                .typed_warnings
                .contains(&MySQLWarning::ChangeCharsetOrCollation {
                    table: "users".to_string(),
                    column: None,
                })
        );
        assert!(migration.sql_statements.iter().any(|sql| {
            sql == "ALTER TABLE `users` DEFAULT CHARACTER SET=utf8mb4 COLLATE=utf8mb4_0900_ai_ci;"
        }));
        assert!(migration.sql_statements.iter().any(|sql| {
            sql.contains("MODIFY COLUMN `status` enum('new', 'disabled')")
                && sql.contains("CHARACTER SET utf8mb4")
                && sql.contains("COLLATE utf8mb4_0900_ai_ci")
        }));
    }

    #[test]
    fn adding_virtual_generated_column_uses_warned_drop_add_transition() {
        let mut prev = table_with_columns("users", &["name", "slug"]);
        prev.views
            .push(model::View::new("user_slugs", "select `slug` from `users`"));
        let mut cur = prev.clone();
        cur.columns.list_mut()[1].generated = Some(model::Generated {
            expression: "lower(`name`)".into(),
            generation_type: model::GeneratedType::Virtual,
        });

        let migration = compute_migration(&prev, &cur).unwrap();

        assert!(
            migration
                .typed_warnings
                .contains(&MySQLWarning::RecreateColumn {
                    table: "users".to_string(),
                    column: "slug".to_string(),
                })
        );
        let drop = migration
            .sql_statements
            .iter()
            .position(|sql| sql == "ALTER TABLE `users` DROP COLUMN `slug`;")
            .unwrap();
        let add = migration
            .sql_statements
            .iter()
            .position(|sql| sql.contains("ADD COLUMN `slug`") && sql.contains("VIRTUAL"))
            .unwrap();
        let drop_view = migration
            .sql_statements
            .iter()
            .position(|sql| sql == "DROP VIEW `user_slugs`;")
            .unwrap();
        let create_view = migration
            .sql_statements
            .iter()
            .position(|sql| sql.starts_with("CREATE ") && sql.contains(" VIEW `user_slugs` AS "))
            .unwrap();
        assert!(drop_view < drop && drop < add && add < create_view);
    }

    #[test]
    fn dropping_cyclic_foreign_key_tables_drops_constraints_first() {
        let mut prev = table_with_columns("left", &["id", "right_id"]);
        let right = table_with_columns("right", &["id", "left_id"]);
        prev.tables.extend(right.tables.clone().into_vec());
        prev.columns.extend(right.columns.clone().into_vec());
        prev.fks.push(model::ForeignKey::new(
            "left",
            "left_right_fk",
            ["right_id"],
            "right",
            ["id"],
        ));
        prev.fks.push(model::ForeignKey::new(
            "right",
            "right_left_fk",
            ["left_id"],
            "left",
            ["id"],
        ));

        let migration = compute_migration(&prev, &MySQLDDL::new()).unwrap();
        let first_table_drop = migration
            .statements
            .iter()
            .position(|statement| matches!(statement, MySQLStatement::DropTable { .. }))
            .unwrap();
        let foreign_key_drops: Vec<_> = migration
            .statements
            .iter()
            .enumerate()
            .filter(|(_, statement)| matches!(statement, MySQLStatement::DropForeignKey { .. }))
            .map(|(position, _)| position)
            .collect();

        assert_eq!(foreign_key_drops.len(), 2);
        assert!(
            foreign_key_drops
                .iter()
                .all(|position| *position < first_table_drop)
        );
    }

    #[test]
    fn modifying_local_and_referenced_columns_suspends_foreign_key() {
        let prev = schema_with_indexed_foreign_key();
        let mut cur = prev.clone();
        cur.columns
            .list_mut()
            .iter_mut()
            .filter(|column| column.name == "id" || column.name == "parent_id")
            .for_each(|column| column.sql_type = "int".into());

        let migration = compute_migration(&prev, &cur).unwrap();
        let drop_fk = migration
            .statements
            .iter()
            .position(|statement| matches!(statement, MySQLStatement::DropForeignKey { .. }))
            .unwrap();
        let first_modify = migration
            .statements
            .iter()
            .position(|statement| matches!(statement, MySQLStatement::ModifyColumn { .. }))
            .unwrap();
        let add_fk = migration
            .statements
            .iter()
            .position(|statement| matches!(statement, MySQLStatement::AddForeignKey { .. }))
            .unwrap();

        assert!(drop_fk < first_modify && first_modify < add_fk);
    }

    #[test]
    fn column_changes_conservatively_suspend_table_checks() {
        let mut prev = table_with_columns("items", &["value", "untouched"]);
        prev.checks.push(model::CheckConstraint::new(
            "items",
            "items_value_check",
            "`value` > 0",
        ));

        let mut modified = prev.clone();
        modified.columns.list_mut()[0].sql_type = "int".into();
        let migration = compute_migration(&prev, &modified).unwrap();
        let drop_check = migration
            .statements
            .iter()
            .position(|statement| matches!(statement, MySQLStatement::DropCheck { .. }))
            .unwrap();
        let modify = migration
            .statements
            .iter()
            .position(|statement| matches!(statement, MySQLStatement::ModifyColumn { .. }))
            .unwrap();
        let add_check = migration
            .statements
            .iter()
            .position(|statement| matches!(statement, MySQLStatement::AddCheck { .. }))
            .unwrap();
        assert!(drop_check < modify && modify < add_check);

        let mut dropped = prev.clone();
        dropped
            .columns
            .list_mut()
            .retain(|column| column.name != "untouched");
        let migration = compute_migration(&prev, &dropped).unwrap();
        let drop_check = migration
            .statements
            .iter()
            .position(|statement| matches!(statement, MySQLStatement::DropCheck { .. }))
            .unwrap();
        let drop_column = migration
            .statements
            .iter()
            .position(|statement| matches!(statement, MySQLStatement::DropColumn { .. }))
            .unwrap();
        let add_check = migration
            .statements
            .iter()
            .position(|statement| matches!(statement, MySQLStatement::AddCheck { .. }))
            .unwrap();
        assert!(drop_check < drop_column && drop_column < add_check);

        let mut renamed = prev.clone();
        renamed.columns.list_mut()[0].name = "amount".into();
        let options = DiffOptions {
            renames: RenameHints::new().column("items", "value", "amount"),
            strict_renames: true,
        };
        let migration = compute_migration_with(&prev, &renamed, &options).unwrap();
        let drop_check = migration
            .statements
            .iter()
            .position(|statement| matches!(statement, MySQLStatement::DropCheck { .. }))
            .unwrap();
        let rename = migration
            .statements
            .iter()
            .position(|statement| matches!(statement, MySQLStatement::RenameColumn { .. }))
            .unwrap();
        let add_check = migration
            .statements
            .iter()
            .position(|statement| matches!(statement, MySQLStatement::AddCheck { .. }))
            .unwrap();
        assert!(drop_check < rename && rename < add_check);
        assert!(migration.sql_statements[add_check].contains("CHECK (`amount` > 0)"));
    }

    #[test]
    fn changing_foreign_key_support_index_suspends_foreign_key() {
        let prev = schema_with_indexed_foreign_key();
        let mut cur = prev.clone();
        cur.indexes
            .list_mut()
            .iter_mut()
            .for_each(|index| index.comment = Some("rebuilt".into()));

        let migration = compute_migration(&prev, &cur).unwrap();
        let drop_fk = migration
            .statements
            .iter()
            .position(|statement| matches!(statement, MySQLStatement::DropForeignKey { .. }))
            .unwrap();
        let drop_indexes: Vec<_> = migration
            .statements
            .iter()
            .enumerate()
            .filter(|(_, statement)| matches!(statement, MySQLStatement::DropIndex { .. }))
            .map(|(position, _)| position)
            .collect();
        let create_indexes: Vec<_> = migration
            .statements
            .iter()
            .enumerate()
            .filter(|(_, statement)| matches!(statement, MySQLStatement::CreateIndex { .. }))
            .map(|(position, _)| position)
            .collect();
        let add_fk = migration
            .statements
            .iter()
            .position(|statement| matches!(statement, MySQLStatement::AddForeignKey { .. }))
            .unwrap();

        assert_eq!(drop_indexes.len(), 2);
        assert_eq!(create_indexes.len(), 2);
        assert!(drop_indexes.iter().all(|position| drop_fk < *position));
        assert!(create_indexes.iter().all(|position| *position < add_fk));
    }

    #[test]
    fn direct_diff_call_canonicalizes_mixed_database_qualification() {
        let mut cur = table_with_columns("users", &["id"]);
        cur.tables.list_mut()[0].database = Some("app".into());

        let migration = compute_migration(&MySQLDDL::new(), &cur).unwrap();

        assert!(migration.sql_statements[0].starts_with("CREATE TABLE `app`.`users`"));
    }

    #[test]
    fn create_table_preserves_generated_column_dependency_order() {
        let mut cur = table_with_columns("metrics", &["z_base", "a_derived"]);
        cur.columns.list_mut()[1].generated = Some(model::Generated {
            expression: "`z_base` + 1".into(),
            generation_type: model::GeneratedType::Stored,
        });

        let migration = compute_migration(&MySQLDDL::new(), &cur).unwrap();
        let sql = &migration.sql_statements[0];

        assert!(sql.find("`z_base`").unwrap() < sql.find("`a_derived`").unwrap());
    }

    #[test]
    fn foreign_key_target_accepts_nonunique_index_prefix() {
        let cur = schema_with_indexed_foreign_key();

        let migration = compute_migration(&MySQLDDL::new(), &cur).unwrap();

        assert!(
            migration
                .statements
                .iter()
                .any(|statement| matches!(statement, MySQLStatement::AddForeignKey { .. }))
        );
    }

    #[test]
    fn composite_unique_does_not_suppress_inline_unique() {
        let mut cur = table_with_columns("users", &["id", "tenant_id"]);
        cur.columns.list_mut()[0].unique = true;
        cur.uniques.push(model::UniqueConstraint::new(
            "users",
            "users_id_tenant_unique",
            ["id", "tenant_id"],
        ));

        let migration = compute_migration(&MySQLDDL::new(), &cur).unwrap();
        let sql = &migration.sql_statements[0];

        assert!(sql.contains("`id` bigint NOT NULL UNIQUE"));
        assert!(sql.contains("CONSTRAINT `users_id_tenant_unique` UNIQUE"));
    }

    #[test]
    fn rename_recreates_generated_dependents_with_rewritten_expression() {
        let mut prev = table_with_columns("metrics", &["old_value", "derived"]);
        prev.columns.list_mut()[1].generated = Some(model::Generated {
            expression: "`old_value` + 1".into(),
            generation_type: model::GeneratedType::Stored,
        });
        let mut cur = prev.clone();
        cur.columns.list_mut()[0].name = "new_value".into();
        let options = DiffOptions {
            renames: RenameHints::new().column("metrics", "old_value", "new_value"),
            strict_renames: true,
        };

        let migration = compute_migration_with(&prev, &cur, &options).unwrap();
        let drop_dependent = migration
            .sql_statements
            .iter()
            .position(|sql| sql == "ALTER TABLE `metrics` DROP COLUMN `derived`;")
            .unwrap();
        let rename = migration
            .sql_statements
            .iter()
            .position(|sql| {
                sql == "ALTER TABLE `metrics` RENAME COLUMN `old_value` TO `new_value`;"
            })
            .unwrap();
        let add_dependent = migration
            .sql_statements
            .iter()
            .position(|sql| {
                sql.contains("ADD COLUMN `derived`")
                    && sql.contains("GENERATED ALWAYS AS (`new_value` + 1)")
            })
            .unwrap();

        assert!(drop_dependent < rename && rename < add_dependent);
    }
}

//! Deterministic MySQL schema diffing.
//!
//! [`compute_migration_with`] compares two [`MySQLDDL`]s and returns typed
//! statements, their SQL, and data-loss warnings. Statements are ordered in
//! dependency phases (drops before creates, tables before the indexes and
//! foreign keys that need them, views in dependency order) rather than entity
//! insertion order. The planner never guesses renames, rejects changes to the
//! selected database scope, and never emits `CREATE`/`DROP DATABASE`.
//!
//! Most callers go through [`crate::diff_with`], which forwards its rename
//! hints and strict mode here.

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
    ViewSecurity, render_column_type, render_statements,
};

/// Explicit table rename within the selected database.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TableRename {
    /// Database qualifier; `None` for the selected database.
    pub database: Option<String>,
    /// Current table name.
    pub from: String,
    /// New table name.
    pub to: String,
}

/// Explicit column rename within the selected database.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColumnRename {
    /// Database qualifier; `None` for the selected database.
    pub database: Option<String>,
    /// Table containing the column (its name after any table rename).
    pub table: String,
    /// Current column name.
    pub from: String,
    /// New column name.
    pub to: String,
}

/// Explicit view rename within the selected database.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ViewRename {
    /// Database qualifier; `None` for the selected database.
    pub database: Option<String>,
    /// Current view name.
    pub from: String,
    /// New view name.
    pub to: String,
}

/// Explicit renames for the MySQL planner, which never guesses one: without
/// a hint, a rename becomes a drop plus a create.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RenameHints {
    /// Table renames.
    pub tables: Vec<TableRename>,
    /// Column renames.
    pub columns: Vec<ColumnRename>,
    /// View renames.
    pub views: Vec<ViewRename>,
}

impl RenameHints {
    /// Creates an empty set of hints.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a table rename in the selected database.
    #[must_use]
    pub fn table(mut self, from: impl Into<String>, to: impl Into<String>) -> Self {
        self.tables.push(TableRename {
            database: None,
            from: from.into(),
            to: to.into(),
        });
        self
    }

    /// Adds a column rename on `table` in the selected database.
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

    /// Adds a view rename in the selected database.
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

/// Options for [`compute_migration_with`].
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DiffOptions {
    /// Explicit renames, applied before diffing.
    pub renames: RenameHints,
    /// When `true`, a hint that does not match the snapshots is an error
    /// instead of being skipped.
    pub strict_renames: bool,
    /// Live database defaults, for push planning against an introspected
    /// database.
    pub catalog_defaults: Option<MySQLCatalogDefaults>,
}

/// Effective defaults reported by the selected MySQL database.
///
/// Push planning uses this live catalog context to compare schema options
/// which introspection intentionally omits when they are inherited.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MySQLCatalogDefaults {
    /// Default storage engine (`@@default_storage_engine`).
    pub engine: Option<String>,
    /// Database default character set.
    pub charset: Option<String>,
    /// Database default collation.
    pub collation: Option<String>,
}

impl MySQLCatalogDefaults {
    /// Creates defaults with every value unknown.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the default engine.
    #[must_use]
    pub fn engine(mut self, engine: impl Into<String>) -> Self {
        self.engine = Some(engine.into());
        self
    }

    /// Sets the default character set.
    #[must_use]
    pub fn charset(mut self, charset: impl Into<String>) -> Self {
        self.charset = Some(charset.into());
        self
    }

    /// Sets the default collation.
    #[must_use]
    pub fn collation(mut self, collation: impl Into<String>) -> Self {
        self.collation = Some(collation.into());
        self
    }
}

/// A possible data-loss or integrity risk in a planned migration.
///
/// Decided from the schemas alone (no row counts). The [`Display`]
/// impl gives the human-readable message that also appears in
/// [`MigrationDiff::warnings`].
///
/// [`Display`]: std::fmt::Display
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
    AddNotNullColumn {
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
            Self::AddNotNullColumn { table, column } => write!(
                formatter,
                "adding NOT NULL column {table}.{column} without a DEFAULT fills existing rows with its type's implicit default"
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

/// A planned MySQL migration.
#[derive(Debug, Clone, Default)]
pub struct MigrationDiff {
    /// Typed operations, in execution order.
    pub statements: Vec<MySQLStatement>,
    /// Rendered SQL; one operation may render to more than one statement.
    pub sql_statements: Vec<String>,
    /// Applied rename hints as `table:from:to`, `column:table:from:to`, or
    /// `view:from:to` strings.
    pub renames: Vec<String>,
    /// Data-loss and integrity warnings.
    pub typed_warnings: Vec<MySQLWarning>,
    /// [`typed_warnings`](Self::typed_warnings) as messages.
    pub warnings: Vec<String>,
}

/// Why a MySQL migration could not be planned. Each variant's message
/// describes the problem.
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
    #[error(
        "MySQL foreign key {name:?} cannot use {action:?} for ON {event} because {table}.{column} is {role} a stored generated column"
    )]
    InvalidGeneratedForeignKeyAction {
        name: String,
        table: String,
        column: String,
        event: &'static str,
        action: model::ReferentialAction,
        role: &'static str,
    },
    #[error(
        "MySQL foreign key {name:?} cannot reference virtual generated column {table}.{column}"
    )]
    VirtualGeneratedForeignKeyTarget {
        name: String,
        table: String,
        column: String,
    },
    #[error(
        "MySQL generated column {table}.{column} cannot depend on AUTO_INCREMENT column {dependency}"
    )]
    GeneratedColumnReferencesAutoIncrement {
        table: String,
        column: String,
        dependency: String,
    },
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
enum ColumnAlterStrategy {
    Modify,
    Recreate,
}

/// Implements MySQL's generated-column ALTER transition matrix.
#[must_use]
const fn generated_alter_strategy(
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
    let databases = ddl
        .tables
        .list()
        .iter()
        .map(|table| table.database.as_deref())
        .chain(
            ddl.columns
                .list()
                .iter()
                .map(|column| column.database.as_deref()),
        )
        .chain(
            ddl.indexes
                .list()
                .iter()
                .map(|index| index.database.as_deref()),
        )
        .chain(
            ddl.fks
                .list()
                .iter()
                .map(|foreign_key| foreign_key.database.as_deref()),
        )
        .chain(
            ddl.pks
                .list()
                .iter()
                .map(|primary_key| primary_key.database.as_deref()),
        )
        .chain(
            ddl.uniques
                .list()
                .iter()
                .map(|unique| unique.database.as_deref()),
        )
        .chain(
            ddl.checks
                .list()
                .iter()
                .map(|check| check.database.as_deref()),
        )
        .chain(ddl.views.list().iter().map(|view| view.database.as_deref()))
        .flatten()
        .map(str::to_string)
        .collect::<BTreeSet<_>>();
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

fn index_columns_equivalent(left: &model::IndexColumn, right: &model::IndexColumn) -> bool {
    left.expression == right.expression
        && left.is_expression == right.is_expression
        && left.length == right.length
        && left.ascending.unwrap_or(true) == right.ascending.unwrap_or(true)
}

fn indexes_equivalent(left: &model::Index, right: &model::Index) -> bool {
    // INFORMATION_SCHEMA materializes omitted BTREE, ASC, and visibility defaults.
    // ALGORITHM and LOCK are execution directives, so the server cannot return them.
    left.database == right.database
        && left.table == right.table
        && left.name == right.name
        && left.unique == right.unique
        && left.columns.len() == right.columns.len()
        && left
            .columns
            .iter()
            .zip(&right.columns)
            .all(|(left, right)| index_columns_equivalent(left, right))
        && left.using.unwrap_or(model::IndexMethod::Btree)
            == right.using.unwrap_or(model::IndexMethod::Btree)
        && left.comment == right.comment
        && left.visible.unwrap_or(true) == right.visible.unwrap_or(true)
}

fn unique_backing_index(
    database: &Option<Cow<'static, str>>,
    table: &str,
    name: &str,
    columns: impl IntoIterator<Item = Cow<'static, str>>,
) -> model::Index {
    let mut index = model::Index::new(
        table.to_string(),
        name.to_string(),
        columns
            .into_iter()
            .map(model::IndexColumn::column)
            .collect(),
    );
    index.database = database.clone();
    index.unique = true;
    index
}

fn reconcile_unique_index_representations(current: &mut MySQLDDL, desired: &MySQLDDL) {
    let mut reconciled_indexes = BTreeSet::new();
    let mut reconciled_uniques = Vec::new();

    for unique in desired.uniques.list() {
        if current
            .uniques
            .list()
            .iter()
            .any(|current| current == unique)
        {
            continue;
        }
        let expected = unique_backing_index(
            &unique.database,
            &unique.table,
            &unique.name,
            unique.columns.iter().cloned(),
        );
        let key = (unique.table.to_string(), unique.name.to_string());
        if !reconciled_indexes.contains(&key)
            && current
                .indexes
                .list()
                .iter()
                .any(|index| indexes_equivalent(index, &expected))
        {
            reconciled_indexes.insert(key);
            reconciled_uniques.push(unique.clone());
        }
    }
    current.uniques.extend(reconciled_uniques);

    for desired_column in desired.columns.list().iter().filter(|column| column.unique) {
        let represented_by_constraint = desired.uniques.list().iter().any(|unique| {
            unique.table == desired_column.table
                && unique.columns.len() == 1
                && unique.columns[0] == desired_column.name
        });
        if represented_by_constraint {
            continue;
        }
        let key = (
            desired_column.table.to_string(),
            desired_column.name.to_string(),
        );
        let expected = unique_backing_index(
            &desired_column.database,
            &desired_column.table,
            &desired_column.name,
            [desired_column.name.clone()],
        );
        if !current
            .indexes
            .list()
            .iter()
            .any(|index| !reconciled_indexes.contains(&key) && indexes_equivalent(index, &expected))
        {
            continue;
        }
        if let Some(column) = current
            .columns
            .list_mut()
            .iter_mut()
            .find(|column| column.table == key.0.as_str() && column.name == key.1.as_str())
        {
            column.unique = true;
            reconciled_indexes.insert(key);
        }
    }

    current.indexes.list_mut().retain(|index| {
        !reconciled_indexes.contains(&(index.table.to_string(), index.name.to_string()))
    });
}

fn same_option(left: Option<&str>, right: Option<&str>) -> bool {
    matches!((left, right), (Some(left), Some(right)) if left.eq_ignore_ascii_case(right))
}

fn reconcile_catalog_defaults(
    current: &mut MySQLDDL,
    desired: &MySQLDDL,
    defaults: &MySQLCatalogDefaults,
) {
    for table in current.tables.list_mut() {
        let Some(desired_table) = desired
            .tables
            .one(table.database.as_deref(), table.name.as_ref())
        else {
            continue;
        };
        if table.engine.is_none()
            && same_option(desired_table.engine.as_deref(), defaults.engine.as_deref())
        {
            table.engine = desired_table.engine.clone();
        }
        if table.charset.is_none()
            && same_option(
                desired_table.charset.as_deref(),
                defaults.charset.as_deref(),
            )
        {
            table.charset = desired_table.charset.clone();
        }
        if table.collation.is_none()
            && same_option(
                desired_table.collation.as_deref(),
                defaults.collation.as_deref(),
            )
        {
            table.collation = desired_table.collation.clone();
        }
    }

    for column in current.columns.list_mut() {
        let Some(desired_column) = desired.columns.one(
            column.database.as_deref(),
            column.table.as_ref(),
            column.name.as_ref(),
        ) else {
            continue;
        };
        let Some(table) = current
            .tables
            .one(column.database.as_deref(), column.table.as_ref())
        else {
            continue;
        };
        let inherited_charset = table.charset.as_deref().or(defaults.charset.as_deref());
        let inherited_collation = table.collation.as_deref().or(defaults.collation.as_deref());
        if column.charset.is_none()
            && same_option(desired_column.charset.as_deref(), inherited_charset)
        {
            column.charset = desired_column.charset.clone();
        }
        if column.collation.is_none()
            && same_option(desired_column.collation.as_deref(), inherited_collation)
        {
            column.collation = desired_column.collation.clone();
        }
    }
}

fn reconcile_primary_key_nullability(current: &mut MySQLDDL, desired: &MySQLDDL) {
    let current_members: BTreeSet<_> = current
        .pks
        .list()
        .iter()
        .flat_map(|primary_key| {
            primary_key.columns.iter().map(|column| {
                (
                    primary_key.database.as_deref().map(str::to_string),
                    primary_key.table.to_string(),
                    column.to_string(),
                )
            })
        })
        .collect();
    let desired_members: BTreeSet<_> = desired
        .pks
        .list()
        .iter()
        .flat_map(|primary_key| {
            primary_key.columns.iter().map(|column| {
                (
                    primary_key.database.as_deref().map(str::to_string),
                    primary_key.table.to_string(),
                    column.to_string(),
                )
            })
        })
        .collect();
    for column in current.columns.list_mut() {
        let key = (
            column.database.as_deref().map(str::to_string),
            column.table.to_string(),
            column.name.to_string(),
        );
        if !current_members.contains(&key) && !desired_members.contains(&key) {
            continue;
        }
        if let Some(desired_column) = desired.columns.one(
            column.database.as_deref(),
            column.table.as_ref(),
            column.name.as_ref(),
        ) {
            column.primary_key = desired_column.primary_key;
        }
    }

    for primary_key in current.pks.list() {
        let Some(desired_primary_key) = desired
            .pks
            .for_table(primary_key.database.as_deref(), primary_key.table.as_ref())
        else {
            continue;
        };
        if primary_key.columns != desired_primary_key.columns {
            continue;
        }
        for name in &primary_key.columns {
            let desired_not_null = desired
                .columns
                .one(
                    desired_primary_key.database.as_deref(),
                    desired_primary_key.table.as_ref(),
                    name.as_ref(),
                )
                .map(|column| column.not_null);
            let Some(desired_not_null) = desired_not_null else {
                continue;
            };
            if let Some(column) = current.columns.list_mut().iter_mut().find(|column| {
                column.database == primary_key.database
                    && column.table == primary_key.table
                    && column.name == *name
            }) {
                column.not_null = desired_not_null;
            }
        }
    }
}

fn reconcile_column_type_spellings(current: &mut MySQLDDL, desired: &MySQLDDL) {
    for current_column in current.columns.list_mut() {
        let Some(desired_column) = desired.columns.one(
            current_column.database.as_deref(),
            current_column.table.as_ref(),
            current_column.name.as_ref(),
        ) else {
            continue;
        };
        let equivalent = match (&current_column.inline_type, &desired_column.inline_type) {
            (Some(current), Some(desired)) => current == desired,
            (None, None) => super::codegen::canonical_sql_type(&current_column.sql_type)
                .zip(super::codegen::canonical_sql_type(&desired_column.sql_type))
                .is_some_and(|(current, desired)| current == desired),
            _ => false,
        };
        if equivalent {
            current_column.sql_type = desired_column.sql_type.clone();
        }
    }
}

fn canonical_column_default(column: &model::Column) -> Option<String> {
    let default = column.default.as_deref()?;
    let column_type = render_column_type(&column_type(column)).unwrap_or_default();
    Some(drizzle_types::mysql::canonical_default(&column_type, default))
}

/// Treats defaults that render to the same canonical `DEFAULT` clause as
/// unchanged, so snapshots written before defaults were canonicalized (a bare
/// `'hello'` on a TEXT column, an unparenthesized `UUID()`) do not churn.
fn reconcile_default_spellings(current: &mut MySQLDDL, desired: &MySQLDDL) {
    for current_column in current.columns.list_mut() {
        let Some(desired_column) = desired.columns.one(
            current_column.database.as_deref(),
            current_column.table.as_ref(),
            current_column.name.as_ref(),
        ) else {
            continue;
        };
        if current_column.default != desired_column.default
            && canonical_column_default(current_column) == canonical_column_default(desired_column)
        {
            current_column.default = desired_column.default.clone();
        }
    }
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

fn invalid_generated_action(
    action: Option<model::ReferentialAction>,
    event: &'static str,
    base_column: bool,
) -> Option<(model::ReferentialAction, &'static str)> {
    let action = action?;
    let invalid = if base_column || event == "UPDATE" {
        matches!(
            action,
            model::ReferentialAction::Cascade | model::ReferentialAction::SetNull
        )
    } else {
        matches!(action, model::ReferentialAction::SetNull)
    };
    invalid.then_some((
        action,
        if base_column {
            "a base column of"
        } else {
            "itself"
        },
    ))
}

fn validate_generated_column_constraints(ddl: &MySQLDDL) -> Result<(), DiffError> {
    for generated_column in ddl
        .columns
        .list()
        .iter()
        .filter(|column| column.generated.is_some())
    {
        let dependencies = identifier_tokens(
            &generated_column
                .generated
                .as_ref()
                .expect("filtered generated columns")
                .expression,
        );
        if let Some(dependency) = ddl.columns.list().iter().find(|column| {
            column.database == generated_column.database
                && column.table == generated_column.table
                && column.autoincrement
                && dependencies.contains(column.name.as_ref())
        }) {
            return Err(DiffError::GeneratedColumnReferencesAutoIncrement {
                table: generated_column.table.to_string(),
                column: generated_column.name.to_string(),
                dependency: dependency.name.to_string(),
            });
        }
    }

    for foreign_key in ddl.fks.list() {
        for column_name in &foreign_key.foreign_columns {
            let Some(column) = ddl.columns.list().iter().find(|column| {
                column.database.as_deref()
                    == foreign_key
                        .foreign_database
                        .as_deref()
                        .or(foreign_key.database.as_deref())
                    && column.table == foreign_key.foreign_table
                    && column.name.as_ref() == column_name.as_ref()
            }) else {
                continue;
            };
            if column
                .generated
                .as_ref()
                .is_some_and(|generated| generated.generation_type == model::GeneratedType::Virtual)
            {
                return Err(DiffError::VirtualGeneratedForeignKeyTarget {
                    name: foreign_key.name.to_string(),
                    table: foreign_key.foreign_table.to_string(),
                    column: column_name.to_string(),
                });
            }
        }

        let stored_columns: Vec<_> = ddl
            .columns
            .list()
            .iter()
            .filter(|column| {
                column.database == foreign_key.database
                    && column.table == foreign_key.table
                    && column.generated.as_ref().is_some_and(|generated| {
                        generated.generation_type == model::GeneratedType::Stored
                    })
            })
            .collect();
        for local_column in &foreign_key.columns {
            let generated_role = stored_columns
                .iter()
                .find_map(|column| {
                    let dependencies = identifier_tokens(
                        &column
                            .generated
                            .as_ref()
                            .expect("filtered stored generated columns")
                            .expression,
                    );
                    dependencies
                        .contains(local_column.as_ref())
                        .then(|| (local_column.to_string(), true))
                })
                .or_else(|| {
                    stored_columns
                        .iter()
                        .find(|column| column.name.as_ref() == local_column.as_ref())
                        .map(|column| (column.name.to_string(), false))
                });
            let Some((column, base_column)) = generated_role else {
                continue;
            };
            for (event, action) in [
                ("DELETE", foreign_key.on_delete),
                ("UPDATE", foreign_key.on_update),
            ] {
                if let Some((action, role)) = invalid_generated_action(action, event, base_column) {
                    return Err(DiffError::InvalidGeneratedForeignKeyAction {
                        name: foreign_key.name.to_string(),
                        table: foreign_key.table.to_string(),
                        column: column.clone(),
                        event,
                        action,
                        role,
                    });
                }
            }
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

/// Whether two versions of a column need `MODIFY COLUMN` (or a recreate).
/// Key membership is excluded: primary keys, unique constraints and the
/// column-level UNIQUE index are separate key steps.
fn columns_differ_structurally(old: &model::Column, new: &model::Column) -> bool {
    let mut old = old.clone();
    let mut new = new.clone();
    old.unique = false;
    new.unique = false;
    old.primary_key = false;
    new.primary_key = false;
    old != new
}

/// Whether `column` carries MySQL's column-level unique index (named after
/// the column) rather than a named single-column unique constraint.
fn column_unique_index(ddl: &MySQLDDL, column: &model::Column) -> bool {
    column.unique
        && !column.primary_key
        && !ddl.uniques.list().iter().any(|unique| {
            unique.table == column.table && unique.columns.len() == 1 && unique.columns[0] == column.name
        })
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
    let mut left = view_definition(left);
    let mut right = view_definition(right);
    if let Some(left) = &mut left {
        left.definer = None;
    }
    if let Some(right) = &mut right {
        right.definer = None;
    }
    left == right
}

/// Indexes of a new table that must be declared inside `CREATE TABLE`:
/// those leading with an `AUTO_INCREMENT` column that neither the primary key
/// nor a unique key leads with. InnoDB requires such a column to start some
/// index when the table is created.
fn inline_index_names(table: &model::Table, ddl: &MySQLDDL) -> BTreeSet<String> {
    let keyed_first = |column: &model::Column| {
        column.unique
            || ddl.pks.list().iter().any(|primary_key| {
                primary_key.table == table.name && primary_key.columns.first() == Some(&column.name)
            })
            || ddl.uniques.list().iter().any(|unique| {
                unique.table == table.name && unique.columns.first() == Some(&column.name)
            })
    };
    ddl.indexes
        .list()
        .iter()
        .filter(|index| index.table == table.name)
        .filter(|index| {
            index.columns.first().is_some_and(|first| {
                !first.is_expression
                    && ddl.columns.list().iter().any(|column| {
                        column.table == table.name
                            && column.name == first.expression
                            && column.autoincrement
                            && !keyed_first(column)
                    })
            })
        })
        .map(|index| index.name.to_string())
        .collect()
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
    let inline = inline_index_names(table, ddl);
    let indexes = ddl
        .indexes
        .list()
        .iter()
        .filter(|index| index.table == table.name && inline.contains(index.name.as_ref()))
        .map(index_definition)
        .collect();
    TableDefinition {
        database: database(&table.database),
        name: table.name.to_string(),
        temporary: table.temporary,
        columns,
        primary_key,
        uniques,
        checks,
        indexes,
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

fn index_map(ddl: &MySQLDDL) -> BTreeMap<(String, String), &model::Index> {
    ddl.indexes
        .list()
        .iter()
        .map(|index| ((index.table.to_string(), index.name.to_string()), index))
        .collect()
}

fn foreign_key_map(ddl: &MySQLDDL) -> BTreeMap<(String, String), &model::ForeignKey> {
    ddl.fks
        .list()
        .iter()
        .map(|foreign_key| {
            (
                (foreign_key.table.to_string(), foreign_key.name.to_string()),
                foreign_key,
            )
        })
        .collect()
}

fn unique_map(ddl: &MySQLDDL) -> BTreeMap<(String, String), &model::UniqueConstraint> {
    ddl.uniques
        .list()
        .iter()
        .map(|unique| ((unique.table.to_string(), unique.name.to_string()), unique))
        .collect()
}

fn check_map(ddl: &MySQLDDL) -> BTreeMap<(String, String), &model::CheckConstraint> {
    ddl.checks
        .list()
        .iter()
        .map(|check| ((check.table.to_string(), check.name.to_string()), check))
        .collect()
}

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

#[derive(Clone, Copy, PartialEq, Eq)]
enum DependencyOrder {
    /// Generated columns before the columns they read (dropping).
    DependentsFirst,
    /// Columns before the generated columns that read them (adding).
    DependenciesFirst,
}

/// Orders columns so a generated column and the columns its expression
/// reads are dropped or added in an order MySQL accepts (error 3108 when a
/// referenced column is dropped first, 1054 when it is added later). The
/// input order is kept otherwise; a dependency cycle keeps the input order.
fn order_by_generated_dependencies(
    columns: Vec<&model::Column>,
    order: DependencyOrder,
) -> Vec<&model::Column> {
    let depends_on = |dependent: &model::Column, dependency: &model::Column| {
        dependent.table == dependency.table
            && dependent.name != dependency.name
            && dependent.generated.as_ref().is_some_and(|generated| {
                identifier_tokens(&generated.expression).contains(dependency.name.as_ref())
            })
    };
    let mut remaining = columns;
    let mut ordered = Vec::with_capacity(remaining.len());
    while !remaining.is_empty() {
        let ready = remaining.iter().position(|column| {
            !remaining.iter().any(|other| match order {
                DependencyOrder::DependentsFirst => depends_on(other, column),
                DependencyOrder::DependenciesFirst => depends_on(column, other),
            })
        });
        ordered.push(remaining.remove(ready.unwrap_or(0)));
    }
    ordered
}

/// The table an operation that may join an `ALTER TABLE` batch acts on.
fn batchable_table(statement: &MySQLStatement) -> Option<&str> {
    match statement {
        MySQLStatement::AddColumn { table, .. }
        | MySQLStatement::ModifyColumn { table, .. }
        | MySQLStatement::DropColumn { table, .. }
        | MySQLStatement::DropPrimaryKey { table, .. }
        | MySQLStatement::DropIndex { table, .. }
        | MySQLStatement::DropUnique { table, .. } => Some(table),
        MySQLStatement::AddPrimaryKey { primary_key } => Some(&primary_key.table),
        MySQLStatement::AddUnique { unique } => Some(&unique.table),
        MySQLStatement::CreateIndex { index } if index.algorithm.is_none() && index.lock.is_none() => {
            Some(&index.table)
        }
        _ => None,
    }
}

/// Whether `statement` changes a primary key or touches an `AUTO_INCREMENT`
/// column (as a column change or as a key part).
fn touches_auto_increment_key(
    statement: &MySQLStatement,
    auto_increment: &BTreeSet<(String, String)>,
    prev_indexes: &BTreeMap<(String, String), &model::Index>,
) -> bool {
    let is_auto = |table: &str, column: &str| {
        auto_increment.contains(&(table.to_string(), column.to_string()))
    };
    let index_parts_auto = |table: &str, columns: &[IndexColumnDefinition]| {
        columns.iter().any(|column| {
            matches!(column, IndexColumnDefinition::Column { name, .. } if is_auto(table, name))
        })
    };
    match statement {
        MySQLStatement::DropPrimaryKey { .. } | MySQLStatement::AddPrimaryKey { .. } => true,
        MySQLStatement::AddColumn { table, column, .. }
        | MySQLStatement::ModifyColumn { table, column, .. } => is_auto(table, &column.name),
        MySQLStatement::DropColumn { table, column, .. } => is_auto(table, column),
        MySQLStatement::DropIndex { table, name, .. }
        | MySQLStatement::DropUnique { table, name, .. } => prev_indexes
            .get(&(table.clone(), name.clone()))
            .is_some_and(|index| {
                index.columns.iter().any(|column| {
                    !column.is_expression && is_auto(table, column.expression.as_ref())
                })
            }) || is_auto(table, name),
        MySQLStatement::AddUnique { unique } => index_parts_auto(&unique.table, &unique.columns),
        MySQLStatement::CreateIndex { index } => index_parts_auto(&index.table, &index.columns),
        _ => false,
    }
}

/// Merges the column and key operations of a table whose primary key or
/// `AUTO_INCREMENT` key changes into one `ALTER TABLE`, placed where the last
/// of them was. Separately, `DROP PRIMARY KEY` fails while an
/// `AUTO_INCREMENT` column depends on it (1075), an `AUTO_INCREMENT` column
/// cannot be added or modified before its key exists (1075), and a new
/// primary key cannot be added before the old one is gone (1068).
fn batch_auto_increment_key_changes(
    statements: Vec<MySQLStatement>,
    prev: &MySQLDDL,
    cur: &MySQLDDL,
) -> Vec<MySQLStatement> {
    let auto_increment: BTreeSet<_> = prev
        .columns
        .list()
        .iter()
        .chain(cur.columns.list())
        .filter(|column| column.autoincrement)
        .map(|column| (column.table.to_string(), column.name.to_string()))
        .collect();
    if auto_increment.is_empty() {
        return statements;
    }
    let prev_indexes = index_map(prev);
    let batched_tables: BTreeSet<String> = statements
        .iter()
        .filter(|statement| touches_auto_increment_key(statement, &auto_increment, &prev_indexes))
        .filter_map(|statement| batchable_table(statement).map(str::to_string))
        .filter(|table| auto_increment.iter().any(|(auto_table, _)| auto_table == table))
        .collect();
    let mut members: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    for (position, statement) in statements.iter().enumerate() {
        if let Some(table) = batchable_table(statement).filter(|table| batched_tables.contains(*table)) {
            members.entry(table.to_string()).or_default().push(position);
        }
    }
    members.retain(|_, positions| positions.len() > 1);
    if members.is_empty() {
        return statements;
    }
    let last_member: BTreeMap<usize, String> = members
        .iter()
        .map(|(table, positions)| (*positions.last().expect("non-empty"), table.clone()))
        .collect();
    let mut batches: BTreeMap<String, Vec<MySQLStatement>> = BTreeMap::new();
    let mut output = Vec::with_capacity(statements.len());
    for (position, statement) in statements.into_iter().enumerate() {
        let table = batchable_table(&statement).map(str::to_string);
        match table.filter(|table| members.contains_key(table)) {
            Some(table) => {
                let database = match &statement {
                    MySQLStatement::AddPrimaryKey { primary_key } => primary_key.database.clone(),
                    MySQLStatement::AddUnique { unique } => unique.database.clone(),
                    MySQLStatement::CreateIndex { index } => index.database.clone(),
                    MySQLStatement::AddColumn { database, .. }
                    | MySQLStatement::ModifyColumn { database, .. }
                    | MySQLStatement::DropColumn { database, .. }
                    | MySQLStatement::DropPrimaryKey { database, .. }
                    | MySQLStatement::DropIndex { database, .. }
                    | MySQLStatement::DropUnique { database, .. } => database.clone(),
                    _ => None,
                };
                batches.entry(table.clone()).or_default().push(statement);
                if last_member.get(&position) == Some(&table) {
                    output.push(MySQLStatement::AlterTable {
                        database,
                        table: table.clone(),
                        operations: batches.remove(&table).unwrap_or_default(),
                    });
                }
            }
            None => output.push(statement),
        }
    }
    output
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

/// Whether `ddl` has a key (other than an index named `except`) whose
/// leading columns are `columns`, which lets InnoDB enforce a foreign key on
/// them without creating an index of its own.
fn has_supporting_key(ddl: &MySQLDDL, table: &str, columns: &[Cow<'static, str>], except: &str) -> bool {
    let columns: Vec<&str> = columns.iter().map(AsRef::as_ref).collect();
    ddl.pks.list().iter().any(|primary_key| {
        primary_key.table == table
            && named_columns_have_prefix(primary_key.columns.iter(), columns.iter().copied())
    }) || ddl.uniques.list().iter().any(|unique| {
        unique.table == table && named_columns_have_prefix(unique.columns.iter(), columns.iter().copied())
    }) || ddl.indexes.list().iter().any(|index| {
        index.table == table && index.name != except && index_supports_columns(index, columns.iter().copied())
    }) || (columns.len() == 1
        && ddl.columns.list().iter().any(|column| {
            column.table == table && column.name == columns[0] && (column.unique || column.primary_key)
        }))
}

/// Whether dropping `foreign_key` must also drop the index InnoDB created
/// for it. That index carries the constraint's name and outlives the
/// constraint (drizzle-kit's `dropAutoIndex`). It exists only when no other
/// key could enforce the constraint, and it stays when the new schema keeps
/// an index or a foreign key of that name, or another remaining foreign key
/// needs it.
fn drops_auto_created_index(
    foreign_key: &model::ForeignKey,
    prev: &MySQLDDL,
    cur: &MySQLDDL,
    dropped_tables: &BTreeSet<String>,
) -> bool {
    let table = foreign_key.table.as_ref();
    let name = foreign_key.name.as_ref();
    if dropped_tables.contains(table)
        || cur.fks.list().iter().any(|other| other.table == table && other.name == name)
        || cur.indexes.list().iter().any(|index| index.table == table && index.name == name)
        || has_supporting_key(prev, table, &foreign_key.columns, name)
    {
        return false;
    }
    !cur.fks.list().iter().any(|other| {
        other.table == table
            && named_columns_have_prefix(foreign_key.columns.iter(), other.columns.iter().map(AsRef::as_ref))
            && !has_supporting_key(cur, table, &other.columns, name)
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

/// Computes a MySQL migration with default [`DiffOptions`] (no rename hints).
///
/// # Errors
///
/// See [`compute_migration_with`].
pub fn compute_migration(prev: &MySQLDDL, cur: &MySQLDDL) -> Result<MigrationDiff, DiffError> {
    compute_migration_with(prev, cur, &DiffOptions::default())
}

/// Computes a deterministic, dependency-phased MySQL migration from `prev` to
/// `cur`.
///
/// # Errors
///
/// Returns a [`DiffError`] if either DDL fails validation, the two use
/// different database scopes, a foreign key or generated column breaks a
/// MySQL rule, a rename hint is invalid (or unmatched under
/// `strict_renames`), a table option change cannot be expressed, views form
/// a dependency cycle, or a statement cannot be rendered.
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
    validate_generated_column_constraints(&cur)?;

    let (rename_statements, renames) =
        apply_rename_hints(&mut prev, &cur, selected.as_deref(), options)?;
    reconcile_unique_index_representations(&mut prev, &cur);
    reconcile_primary_key_nullability(&mut prev, &cur);
    reconcile_column_type_spellings(&mut prev, &cur);
    reconcile_default_spellings(&mut prev, &cur);
    if let Some(defaults) = &options.catalog_defaults {
        reconcile_catalog_defaults(&mut prev, &cur, defaults);
    }
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
                .filter(|new| columns_differ_structurally(old, new))
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

    // A column-level UNIQUE is MySQL's index named after the column. It is
    // added and dropped as an index step, never through MODIFY COLUMN (which
    // would add another index each time and could never remove one).
    let column_unique_changes: Vec<_> = prev_columns
        .iter()
        .filter_map(|(key, old)| {
            let new = cur_columns.get(key)?;
            if recreated_columns.contains(key) {
                return None;
            }
            let old_unique = column_unique_index(&prev, old);
            let new_unique = column_unique_index(&cur, new);
            (old_unique != new_unique).then(|| (key.clone(), *old, *new, new_unique))
        })
        .collect();

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
                && (cur_indexes
                    .get(*key)
                    .is_none_or(|new| !indexes_equivalent(old, new))
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
        .chain(
            column_unique_changes
                .iter()
                .filter(|(_, _, _, added)| !added)
                .map(|(key, _, _, _)| key.0.clone()),
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
            if drops_auto_created_index(foreign_key, &prev, &cur, &dropped_tables) {
                statements.push(MySQLStatement::DropIndex {
                    database: database(&foreign_key.database),
                    table: foreign_key.table.to_string(),
                    name: foreign_key.name.to_string(),
                });
            }
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
    for (key, old, _, _) in column_unique_changes
        .iter()
        .filter(|(_, _, _, added)| !added)
    {
        statements.push(MySQLStatement::DropUnique {
            database: database(&old.database),
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

    let dropped_in_order = order_by_generated_dependencies(
        dropped_columns
            .iter()
            .filter(|key| !rename_generated_dependents.contains(key))
            .map(|key| prev_columns[key])
            .collect(),
        DependencyOrder::DependentsFirst,
    );
    for old in dropped_in_order {
        let (table, column) = (old.table.to_string(), old.name.to_string());
        statements.push(MySQLStatement::DropColumn {
            database: database(&old.database),
            table,
            column,
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

    let added_in_order = order_by_generated_dependencies(
        cur.columns
            .list()
            .iter()
            .filter(|column| {
                !created_tables.contains(column.table.as_ref())
                    && !prev_columns
                        .contains_key(&(column.table.to_string(), column.name.to_string()))
            })
            .collect(),
        DependencyOrder::DependenciesFirst,
    );
    for column in added_in_order {
        let key = (column.table.to_string(), column.name.to_string());
        statements.push(MySQLStatement::AddColumn {
            database: database(&column.database),
            table: key.0.clone(),
            column: column_definition_for_ddl(column, &cur),
        });
        if column.not_null
            && column.default.is_none()
            && column.generated.is_none()
            && !column.autoincrement
        {
            warnings.insert(MySQLWarning::AddNotNullColumn {
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
            column: column_definition_for_ddl(column, &cur),
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
        let mut definition = column_definition_for_ddl(new, &cur);
        let statement = if recreated_columns.contains(&key) {
            MySQLStatement::RecreateColumn {
                database: database(&new.database),
                table: key.0.clone(),
                column: definition,
            }
        } else {
            // Keys are separate steps: an inline UNIQUE or PRIMARY KEY in
            // MODIFY COLUMN adds a duplicate index or a second primary key.
            definition.unique = false;
            definition.primary_key = false;
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
    for (key, _, new, _) in column_unique_changes
        .iter()
        .filter(|(_, _, _, added)| *added)
    {
        statements.push(MySQLStatement::AddUnique {
            unique: UniqueDefinition {
                database: database(&new.database),
                table: key.0.clone(),
                name: key.1.clone(),
                columns: vec![IndexColumnDefinition::Column {
                    name: key.1.clone(),
                    length: None,
                    order: None,
                }],
            },
        });
    }
    let inline_created_indexes: BTreeSet<_> = created_tables
        .iter()
        .flat_map(|table| {
            inline_index_names(cur_tables[table], &cur)
                .into_iter()
                .map(move |name| (table.clone(), name))
        })
        .collect();
    for (key, index) in &cur_indexes {
        if inline_created_indexes.contains(key) {
            continue;
        }
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
                check: check_definition(check),
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

    let statements = batch_auto_increment_key_changes(statements, &prev, &cur);
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

    fn server_unique_index(table: &str, name: &str, columns: &[&str]) -> model::Index {
        let mut index = model::Index::new(
            table.to_string(),
            name.to_string(),
            columns
                .iter()
                .map(|column| {
                    let mut column = model::IndexColumn::column((*column).to_string());
                    column.ascending = Some(true);
                    column
                })
                .collect(),
        );
        index.unique = true;
        index.using = Some(model::IndexMethod::Btree);
        index.visible = Some(true);
        index
    }

    fn primary_key(table: &str, columns: &[&str]) -> model::PrimaryKey {
        model::PrimaryKey::new(
            table.to_string(),
            columns.iter().map(|column| (*column).to_string()),
        )
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
    fn stored_generated_base_columns_reject_cascading_foreign_key_actions() {
        let mut ddl = schema_with_indexed_foreign_key();
        let mut generated = model::Column::new("child", "derived_parent_id", "bigint");
        generated.generated = Some(model::Generated::stored("parent_id + 1"));
        ddl.columns.push(generated);
        ddl.fks.list_mut()[0].on_delete = Some(model::ReferentialAction::Cascade);

        assert!(matches!(
            compute_migration(&MySQLDDL::new(), &ddl),
            Err(DiffError::InvalidGeneratedForeignKeyAction {
                event: "DELETE",
                action: model::ReferentialAction::Cascade,
                role: "a base column of",
                ..
            })
        ));
    }

    #[test]
    fn stored_generated_base_columns_allow_non_cascading_foreign_key_actions() {
        let mut ddl = schema_with_indexed_foreign_key();
        let mut generated = model::Column::new("child", "derived_parent_id", "bigint");
        generated.generated = Some(model::Generated::stored("parent_id + 1"));
        ddl.columns.push(generated);
        ddl.fks.list_mut()[0].on_delete = Some(model::ReferentialAction::Restrict);

        compute_migration(&MySQLDDL::new(), &ddl)
            .expect("RESTRICT is valid for a foreign-key base of a stored generated column");
    }

    #[test]
    fn generated_columns_reject_auto_increment_dependencies() {
        let mut ddl = table_with_columns("jobs", &["id", "derived_id"]);
        ddl.columns.list_mut()[0].autoincrement = true;
        ddl.columns.list_mut()[0].primary_key = true;
        ddl.columns.list_mut()[1].generated = Some(model::Generated::stored("id + 1"));

        assert!(matches!(
            compute_migration(&MySQLDDL::new(), &ddl),
            Err(DiffError::GeneratedColumnReferencesAutoIncrement {
                table,
                column,
                dependency,
            }) if table == "jobs" && column == "derived_id" && dependency == "id"
        ));
    }

    #[test]
    fn foreign_keys_reject_virtual_generated_targets() {
        let mut ddl = schema_with_indexed_foreign_key();
        ddl.columns.list_mut()[0].generated =
            Some(model::Generated::virtual_column("tenant_id + 1"));

        assert!(matches!(
            compute_migration(&MySQLDDL::new(), &ddl),
            Err(DiffError::VirtualGeneratedForeignKeyTarget {
                name,
                table,
                column,
            }) if name == "child_parent_fk" && table == "parent" && column == "id"
        ));
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
            ..DiffOptions::default()
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
    fn inherited_mysql_defaults_match_explicit_desired_options_with_catalog_context() {
        let mut prev = table_with_columns("users", &["name"]);
        prev.columns.list_mut()[0].sql_type = "varchar(255)".into();
        let mut cur = prev.clone();
        cur.tables.list_mut()[0].engine = Some("InnoDB".into());
        cur.tables.list_mut()[0].charset = Some("utf8mb4".into());
        cur.tables.list_mut()[0].collation = Some("utf8mb4_0900_ai_ci".into());
        cur.columns.list_mut()[0].charset = Some("utf8mb4".into());
        cur.columns.list_mut()[0].collation = Some("utf8mb4_0900_ai_ci".into());
        let options = DiffOptions {
            catalog_defaults: Some(
                MySQLCatalogDefaults::new()
                    .engine("InnoDB")
                    .charset("utf8mb4")
                    .collation("utf8mb4_0900_ai_ci"),
            ),
            ..DiffOptions::default()
        };

        let migration = compute_migration_with(&prev, &cur, &options).unwrap();

        assert!(migration.statements.is_empty());
    }

    #[test]
    fn catalog_context_does_not_hide_distinct_mysql_options() {
        let mut prev = table_with_columns("users", &["name"]);
        prev.columns.list_mut()[0].sql_type = "varchar(255)".into();
        let mut cur = prev.clone();
        cur.tables.list_mut()[0].engine = Some("MyISAM".into());
        cur.tables.list_mut()[0].charset = Some("latin1".into());
        cur.tables.list_mut()[0].collation = Some("latin1_swedish_ci".into());
        cur.columns.list_mut()[0].charset = Some("latin1".into());
        cur.columns.list_mut()[0].collation = Some("latin1_swedish_ci".into());
        let options = DiffOptions {
            catalog_defaults: Some(
                MySQLCatalogDefaults::new()
                    .engine("InnoDB")
                    .charset("utf8mb4")
                    .collation("utf8mb4_0900_ai_ci"),
            ),
            ..DiffOptions::default()
        };

        let migration = compute_migration_with(&prev, &cur, &options).unwrap();

        assert!(migration.sql_statements.iter().any(|sql| {
            sql.contains("ENGINE=MyISAM")
                && sql.contains("DEFAULT CHARACTER SET=latin1")
                && sql.contains("COLLATE=latin1_swedish_ci")
        }));
        assert!(migration.sql_statements.iter().any(|sql| {
            sql.contains("MODIFY COLUMN `name`")
                && sql.contains("CHARACTER SET latin1")
                && sql.contains("COLLATE latin1_swedish_ci")
        }));
    }

    #[test]
    fn ordinary_diff_keeps_explicit_mysql_default_options_structural() {
        let mut prev = table_with_columns("users", &["name"]);
        prev.columns.list_mut()[0].sql_type = "varchar(255)".into();
        let mut cur = prev.clone();
        cur.tables.list_mut()[0].engine = Some("InnoDB".into());
        cur.tables.list_mut()[0].charset = Some("utf8mb4".into());
        cur.tables.list_mut()[0].collation = Some("utf8mb4_0900_ai_ci".into());
        cur.columns.list_mut()[0].charset = Some("utf8mb4".into());
        cur.columns.list_mut()[0].collation = Some("utf8mb4_0900_ai_ci".into());

        let migration = compute_migration(&prev, &cur).unwrap();

        assert!(
            migration
                .statements
                .iter()
                .any(|statement| matches!(statement, MySQLStatement::AlterTableOptions { .. }))
        );
        assert!(
            migration
                .statements
                .iter()
                .any(|statement| matches!(statement, MySQLStatement::ModifyColumn { .. }))
        );
    }

    #[test]
    fn primary_key_catalog_not_null_is_ignored_while_membership_is_stable() {
        let mut prev = table_with_columns("users", &["id"]);
        prev.columns.list_mut()[0].primary_key = true;
        prev.pks.push(primary_key("users", &["id"]));
        let mut cur = prev.clone();
        cur.columns.list_mut()[0].not_null = false;

        let migration = compute_migration(&prev, &cur).unwrap();

        assert!(migration.statements.is_empty());
    }

    #[test]
    fn leaving_primary_key_restores_desired_nullable_column() {
        let mut prev = table_with_columns("users", &["id"]);
        prev.columns.list_mut()[0].primary_key = true;
        prev.pks.push(primary_key("users", &["id"]));
        let mut cur = prev.clone();
        cur.pks.list_mut().clear();
        cur.columns.list_mut()[0].not_null = false;
        cur.columns.list_mut()[0].primary_key = false;

        let migration = compute_migration(&prev, &cur).unwrap();

        assert!(
            migration
                .statements
                .iter()
                .any(|statement| matches!(statement, MySQLStatement::DropPrimaryKey { .. }))
        );
        assert!(
            migration
                .statements
                .iter()
                .any(|statement| matches!(statement, MySQLStatement::ModifyColumn { .. }))
        );
    }

    #[test]
    fn entering_primary_key_does_not_emit_redundant_nullability_change() {
        let mut prev = table_with_columns("users", &["id"]);
        prev.columns.list_mut()[0].not_null = false;
        let mut cur = prev.clone();
        cur.columns.list_mut()[0].primary_key = true;
        cur.pks.push(primary_key("users", &["id"]));

        let migration = compute_migration(&prev, &cur).unwrap();

        assert_eq!(migration.statements.len(), 1);
        assert!(matches!(
            migration.statements[0],
            MySQLStatement::AddPrimaryKey { .. }
        ));
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
        renamed.checks.list_mut()[0].expression = "`amount` > 0".into();
        let options = DiffOptions {
            renames: RenameHints::new().column("items", "value", "amount"),
            strict_renames: true,
            ..DiffOptions::default()
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
    fn inline_unique_matches_server_unique_index() {
        let mut prev = table_with_columns("users", &["email"]);
        prev.indexes
            .push(server_unique_index("users", "email", &["email"]));
        let mut cur = table_with_columns("users", &["email"]);
        cur.columns.list_mut()[0].unique = true;

        let migration = compute_migration(&prev, &cur).unwrap();

        assert!(migration.statements.is_empty());
    }

    #[test]
    fn inline_unique_reconciliation_preserves_foreign_key_target_validation() {
        let mut prev = table_with_columns("users", &["email"]);
        let child = table_with_columns("profiles", &["email"]);
        prev.tables.extend(child.tables.clone().into_vec());
        prev.columns.extend(child.columns.clone().into_vec());
        prev.indexes
            .push(server_unique_index("users", "email", &["email"]));
        prev.fks.push(model::ForeignKey::new(
            "profiles",
            "profiles_email_fk",
            ["email"],
            "users",
            ["email"],
        ));

        let mut cur = prev.clone();
        cur.indexes.list_mut().clear();
        cur.columns
            .list_mut()
            .iter_mut()
            .find(|column| column.table == "users" && column.name == "email")
            .unwrap()
            .unique = true;

        let migration = compute_migration(&prev, &cur).unwrap();

        assert!(migration.statements.is_empty());
    }

    #[test]
    fn named_unique_constraint_matches_server_unique_index() {
        let mut prev = table_with_columns("users", &["email", "tenant_id"]);
        prev.indexes.push(server_unique_index(
            "users",
            "users_email_tenant_unique",
            &["email", "tenant_id"],
        ));
        let mut cur = table_with_columns("users", &["email", "tenant_id"]);
        cur.uniques.push(model::UniqueConstraint::new(
            "users",
            "users_email_tenant_unique",
            ["email", "tenant_id"],
        ));

        let migration = compute_migration(&prev, &cur).unwrap();

        assert!(migration.statements.is_empty());
    }

    #[test]
    fn explicit_unique_index_matches_catalog_defaults_and_nonpersistent_options() {
        let mut prev = table_with_columns("users", &["email"]);
        prev.indexes.push(server_unique_index(
            "users",
            "users_email_unique",
            &["email"],
        ));
        let mut cur = table_with_columns("users", &["email"]);
        let mut desired = model::Index::new(
            "users",
            "users_email_unique",
            vec![model::IndexColumn::column("email")],
        );
        desired.unique = true;
        desired.using = Some(model::IndexMethod::Btree);
        desired.algorithm = Some(model::IndexAlgorithm::Copy);
        desired.lock = Some(model::IndexLock::Exclusive);
        cur.indexes.push(desired);

        let migration = compute_migration(&prev, &cur).unwrap();

        assert!(migration.statements.is_empty());
    }

    #[test]
    fn meaningful_unique_index_changes_are_rebuilt() {
        type IndexChange = fn(&mut model::Index);
        let cases: [(&str, IndexChange); 8] = [
            ("name", |index| index.name = "renamed".into()),
            ("columns", |index| {
                index.columns = vec![model::IndexColumn::column("tenant_id")];
            }),
            ("method", |index| {
                index.using = Some(model::IndexMethod::Hash)
            }),
            ("visibility", |index| index.visible = Some(false)),
            ("comment", |index| index.comment = Some("lookup".into())),
            ("prefix", |index| index.columns[0].length = Some(16)),
            ("order", |index| index.columns[0].ascending = Some(false)),
            ("expression", |index| {
                index.columns = vec![model::IndexColumn::expression("lower(`email`)")];
            }),
        ];

        for (case, change) in cases {
            let mut prev = table_with_columns("users", &["email", "tenant_id"]);
            prev.indexes.push(server_unique_index(
                "users",
                "users_email_unique",
                &["email"],
            ));
            let mut cur = table_with_columns("users", &["email", "tenant_id"]);
            let mut desired = model::Index::new(
                "users",
                "users_email_unique",
                vec![model::IndexColumn::column("email")],
            );
            desired.unique = true;
            desired.using = Some(model::IndexMethod::Btree);
            change(&mut desired);
            cur.indexes.push(desired);

            let migration = compute_migration(&prev, &cur).unwrap();

            assert!(
                migration
                    .statements
                    .iter()
                    .any(|statement| matches!(statement, MySQLStatement::DropIndex { .. })),
                "{case} change did not drop the old index"
            );
            assert!(
                migration
                    .statements
                    .iter()
                    .any(|statement| matches!(statement, MySQLStatement::CreateIndex { .. })),
                "{case} change did not create the desired index"
            );
        }
    }

    #[test]
    fn unique_constraint_does_not_absorb_meaningful_index_changes() {
        for case in ["name", "columns", "visibility"] {
            let mut prev = table_with_columns("users", &["email", "tenant_id"]);
            let mut index = server_unique_index("users", "users_email_unique", &["email"]);
            match case {
                "name" => index.name = "legacy_email_unique".into(),
                "columns" => {
                    index.columns = vec![model::IndexColumn::column("tenant_id")];
                }
                "visibility" => index.visible = Some(false),
                _ => unreachable!(),
            }
            prev.indexes.push(index);
            let mut cur = table_with_columns("users", &["email", "tenant_id"]);
            cur.uniques.push(model::UniqueConstraint::new(
                "users",
                "users_email_unique",
                ["email"],
            ));

            let migration = compute_migration(&prev, &cur).unwrap();

            assert!(
                migration
                    .statements
                    .iter()
                    .any(|statement| matches!(statement, MySQLStatement::DropIndex { .. })),
                "{case} change was absorbed"
            );
            assert!(
                migration
                    .statements
                    .iter()
                    .any(|statement| matches!(statement, MySQLStatement::AddUnique { .. })),
                "{case} change did not add the desired constraint"
            );
        }
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
        cur.columns.list_mut()[1].generated = Some(model::Generated {
            expression: "`new_value` + 1".into(),
            generation_type: model::GeneratedType::Stored,
        });
        let options = DiffOptions {
            renames: RenameHints::new().column("metrics", "old_value", "new_value"),
            strict_renames: true,
            ..DiffOptions::default()
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

    #[test]
    fn rename_hints_never_rewrite_current_definitions() {
        // The current schema renames qty -> amount and introduces a new
        // column that happens to be called qty. Current CHECK and generated
        // definitions already use current names and must be emitted as-is.
        let mut prev = table_with_columns("orders", &["qty"]);
        prev.checks.push(model::CheckConstraint::new(
            "orders",
            "orders_chk",
            "qty > 0",
        ));
        let mut cur = table_with_columns("orders", &["amount", "qty", "qty_next"]);
        cur.columns.list_mut()[2].generated = Some(model::Generated {
            expression: "qty + 1".into(),
            generation_type: model::GeneratedType::Stored,
        });
        cur.checks.push(model::CheckConstraint::new(
            "orders",
            "orders_chk",
            "amount > 0 AND qty < 100",
        ));
        let options = DiffOptions {
            renames: RenameHints::new().column("orders", "qty", "amount"),
            strict_renames: true,
            ..DiffOptions::default()
        };

        let sql = compute_migration_with(&prev, &cur, &options)
            .unwrap()
            .sql_statements
            .join("\n");
        assert!(sql.contains("RENAME COLUMN `qty` TO `amount`"), "{sql}");
        assert!(sql.contains("GENERATED ALWAYS AS (qty + 1)"), "{sql}");
        assert!(sql.contains("CHECK (amount > 0 AND qty < 100)"), "{sql}");
        assert!(!sql.contains("amount + 1"), "{sql}");
        assert!(!sql.contains("amount < 100"), "{sql}");
    }

    #[test]
    fn legacy_unparenthesized_defaults_do_not_churn() {
        let mut prev = table_with_columns("docs", &["id"]);
        let mut body = model::Column::new("docs", "body", "text");
        body.default = Some("'hello'".into());
        prev.columns.push(body);
        let mut uid = model::Column::new("docs", "uid", "varchar(36)");
        uid.default = Some("UUID()".into());
        prev.columns.push(uid);

        let mut cur = prev.clone();
        cur.columns.list_mut()[1].default = Some("('hello')".into());
        cur.columns.list_mut()[2].default = Some("(UUID())".into());

        assert!(compute_migration(&prev, &cur).unwrap().statements.is_empty());

        // A real default change still renders an accepted clause.
        cur.columns.list_mut()[1].default = Some("('bye')".into());
        let sql = compute_migration(&prev, &cur).unwrap().sql_statements;
        assert_eq!(
            sql,
            ["ALTER TABLE `docs` MODIFY COLUMN `body` text NULL DEFAULT ('bye');"]
        );
    }

    #[test]
    fn column_unique_changes_are_index_steps_not_modify_clauses() {
        let mut prev = table_with_columns("users", &["id", "email"]);
        prev.pks.push(primary_key("users", &["id"]));
        let mut cur = prev.clone();
        cur.columns.list_mut()[1].unique = true;

        let added = compute_migration(&prev, &cur).unwrap().sql_statements;
        assert_eq!(
            added,
            ["ALTER TABLE `users` ADD CONSTRAINT `email` UNIQUE (`email`);"]
        );

        let removed = compute_migration(&cur, &prev).unwrap().sql_statements;
        assert_eq!(removed, ["ALTER TABLE `users` DROP INDEX `email`;"]);

        // A type change on a unique column must not add another index.
        let mut retyped = cur.clone();
        retyped.columns.list_mut()[1].sql_type = "int".into();
        let modified = compute_migration(&cur, &retyped).unwrap().sql_statements;
        assert_eq!(
            modified,
            ["ALTER TABLE `users` MODIFY COLUMN `email` int NOT NULL;"]
        );
    }

    #[test]
    fn adding_columns_does_not_claim_a_nullability_change() {
        let prev = table_with_columns("users", &["id"]);
        let mut cur = prev.clone();
        let mut serial = model::Column::new("users", "seq", "bigint");
        serial.not_null = true;
        serial.autoincrement = true;
        serial.unique = true;
        cur.columns.push(serial);
        let mut required = model::Column::new("users", "required", "int");
        required.not_null = true;
        cur.columns.push(required);

        let migration = compute_migration(&prev, &cur).unwrap();
        assert_eq!(
            migration.typed_warnings,
            [MySQLWarning::AddNotNullColumn {
                table: "users".to_string(),
                column: "required".to_string(),
            }]
        );
        assert!(
            migration
                .warnings
                .iter()
                .all(|warning| !warning.contains("can fail when rows contain NULL"))
        );
    }

    #[test]
    fn auto_increment_column_keyed_by_secondary_index_declares_it_inline() {
        // PRIMARY KEY (tenant, id) does not lead with the AUTO_INCREMENT
        // column, so InnoDB needs the `id` index inside CREATE TABLE.
        let mut cur = table_with_columns("tickets", &["tenant", "id", "note"]);
        cur.columns.list_mut()[1].autoincrement = true;
        cur.pks.push(primary_key("tickets", &["tenant", "id"]));
        cur.indexes.push(model::Index::new(
            "tickets",
            "tickets_id_idx",
            vec![model::IndexColumn::column("id")],
        ));
        cur.indexes.push(model::Index::new(
            "tickets",
            "tickets_note_idx",
            vec![model::IndexColumn::column("note")],
        ));

        let sql = compute_migration(&MySQLDDL::new(), &cur)
            .unwrap()
            .sql_statements;
        assert_eq!(sql.len(), 2, "{sql:#?}");
        assert!(
            sql[0].contains("PRIMARY KEY (`tenant`, `id`),\n\tKEY `tickets_id_idx` (`id`)"),
            "{}",
            sql[0]
        );
        assert_eq!(
            sql[1],
            "CREATE INDEX `tickets_note_idx` ON `tickets` (`note`);"
        );
    }

    #[test]
    fn generated_column_dependencies_order_column_drops_and_adds() {
        let generated = |table: &str, name: &str, expression: &str| {
            let mut column = model::Column::new(table.to_string(), name.to_string(), "int");
            column.generated = Some(model::Generated {
                expression: expression.to_string().into(),
                generation_type: model::GeneratedType::Stored,
            });
            column
        };
        let base = table_with_columns("metrics", &["id"]);
        let mut with_generated = base.clone();
        with_generated
            .columns
            .push(generated("metrics", "a_double", "z_base * 2"));
        with_generated
            .columns
            .push(model::Column::new("metrics", "z_base", "int"));
        with_generated
            .columns
            .push(generated("metrics", "b_triple", "`z_base` * 3"));

        // Dependents are dropped before the column they read (3108 otherwise).
        let drops = compute_migration(&with_generated, &base)
            .unwrap()
            .sql_statements;
        let position = |sql: &[String], column: &str| {
            sql.iter()
                .position(|statement| statement.contains(&format!("COLUMN `{column}`")))
                .unwrap()
        };
        assert!(position(&drops, "a_double") < position(&drops, "z_base"));
        assert!(position(&drops, "b_triple") < position(&drops, "z_base"));

        // The column a generated column reads is added first (1054 otherwise).
        let adds = compute_migration(&base, &with_generated)
            .unwrap()
            .sql_statements;
        assert!(position(&adds, "z_base") < position(&adds, "a_double"));
        assert!(position(&adds, "z_base") < position(&adds, "b_triple"));
    }

    #[test]
    fn auto_increment_key_changes_are_one_alter_table() {
        let auto_table = |pk: &[&str], auto: bool| {
            let mut ddl = table_with_columns("tickets", &["id", "tenant"]);
            ddl.columns.list_mut()[0].autoincrement = auto;
            for column in ddl.columns.list_mut() {
                column.primary_key = pk.contains(&column.name.as_ref());
            }
            ddl.pks.push(primary_key("tickets", pk));
            ddl
        };

        // Widening the key of an AUTO_INCREMENT column: a lone DROP PRIMARY
        // KEY fails with error 1075.
        let widened = compute_migration(&auto_table(&["id"], true), &auto_table(&["id", "tenant"], true))
            .unwrap()
            .sql_statements;
        assert_eq!(
            widened,
            ["ALTER TABLE `tickets` DROP PRIMARY KEY, ADD PRIMARY KEY (`id`, `tenant`);"]
        );

        // Moving the key off an AUTO_INCREMENT column that stops being one.
        let moved = compute_migration(&auto_table(&["id"], true), &auto_table(&["tenant"], false))
            .unwrap()
            .sql_statements;
        assert_eq!(
            moved,
            [
                "ALTER TABLE `tickets` DROP PRIMARY KEY, MODIFY COLUMN `id` bigint NOT NULL, ADD PRIMARY KEY (`tenant`);"
            ]
        );

        // Introducing a new AUTO_INCREMENT primary key column.
        let mut before = table_with_columns("tickets", &["tenant"]);
        before.columns.list_mut()[0].primary_key = true;
        before.pks.push(primary_key("tickets", &["tenant"]));
        let mut after = table_with_columns("tickets", &["tenant", "id"]);
        after.columns.list_mut()[1].autoincrement = true;
        after.columns.list_mut()[1].primary_key = true;
        after.pks.push(primary_key("tickets", &["id"]));
        let added = compute_migration(&before, &after).unwrap().sql_statements;
        assert_eq!(
            added,
            [
                "ALTER TABLE `tickets` DROP PRIMARY KEY, ADD COLUMN `id` bigint NOT NULL AUTO_INCREMENT, ADD PRIMARY KEY (`id`);"
            ]
        );
    }

    #[test]
    fn dropping_a_foreign_key_drops_the_index_innodb_created_for_it() {
        let mut with_fk = table_with_columns("parents", &["id"]);
        with_fk.columns.list_mut()[0].primary_key = true;
        with_fk.pks.push(primary_key("parents", &["id"]));
        with_fk.tables.push(model::Table::new("children"));
        for name in ["id", "parent_id"] {
            let mut column = model::Column::new("children", name, "bigint");
            column.not_null = true;
            with_fk.columns.push(column);
        }
        with_fk.pks.push(primary_key("children", &["id"]));
        with_fk.fks.push(model::ForeignKey::new(
            "children",
            "children_parent_id_fkey",
            ["parent_id"],
            "parents",
            ["id"],
        ));
        let mut without_fk = with_fk.clone();
        without_fk.fks.list_mut().clear();

        assert_eq!(
            compute_migration(&with_fk, &without_fk).unwrap().sql_statements,
            [
                "ALTER TABLE `children` DROP FOREIGN KEY `children_parent_id_fkey`;",
                "ALTER TABLE `children` DROP INDEX `children_parent_id_fkey`;",
            ]
        );

        // With another key on the column InnoDB never created an index.
        let index = model::Index::new(
            "children",
            "children_parent_idx",
            vec![model::IndexColumn::column("parent_id")],
        );
        with_fk.indexes.push(index.clone());
        without_fk.indexes.push(index);
        assert_eq!(
            compute_migration(&with_fk, &without_fk).unwrap().sql_statements,
            ["ALTER TABLE `children` DROP FOREIGN KEY `children_parent_id_fkey`;"]
        );
    }
}

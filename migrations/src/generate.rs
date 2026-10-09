//! Diff two schemas in memory and get the migration SQL.
//!
//! No file I/O and no CLI. Use [`diff`] / [`diff_with`] for two
//! [`Snapshot`]s, or [`diff_schemas`] / [`diff_schemas_with`] for two
//! [`Schema`] values. To write a migration folder, use
//! [`build::run`](crate::build::run) instead.
//!
//! # Examples
//!
//! Snapshot to snapshot:
//!
//! ```rust
//! use drizzle_migrations::{Snapshot, diff, parser::SchemaParser};
//! use drizzle_types::Dialect;
//!
//! let parsed = SchemaParser::parse(r#"
//!     #[SQLiteTable]
//!     pub struct Users {
//!         #[column(primary)]
//!         pub id: i64,
//!         pub name: String,
//!     }
//! "#);
//! let current = Snapshot::from_parse_result(&parsed, Dialect::SQLite, None);
//!
//! let plan = diff(&Snapshot::empty(Dialect::SQLite), &current)?;
//! assert_eq!(plan.statements.len(), 1);
//! assert!(plan.statements[0].starts_with("CREATE TABLE `users`"));
//! # Ok::<(), drizzle_migrations::MigrationError>(())
//! ```
//!
//! Schema to schema, with rename hints:
//!
//! ```rust,no_run
//! use drizzle_migrations::{DiffOptions, diff_schemas_with};
//! use drizzle_migrations::{Schema, Snapshot};
//! use drizzle_types::Dialect;
//!
//! # #[derive(Default)]
//! # struct V1;
//! # #[derive(Default)]
//! # struct V2;
//! # impl Schema for V1 {
//! #     fn to_snapshot(&self) -> Snapshot { Snapshot::empty(Dialect::SQLite) }
//! #     fn dialect(&self) -> Dialect { Dialect::SQLite }
//! # }
//! # impl Schema for V2 {
//! #     fn to_snapshot(&self) -> Snapshot { Snapshot::empty(Dialect::SQLite) }
//! #     fn dialect(&self) -> Dialect { Dialect::SQLite }
//! # }
//!
//! let generated = diff_schemas_with(
//!     &V1,
//!     &V2,
//!     &DiffOptions::new()
//!         .rename_table("users_old", "users")
//!         .rename_column("users", "full_name", "name")
//!         .strict_renames(true),
//! )?;
//!
//! if !generated.is_empty() {
//!     let _sql = generated.to_sql();
//! }
//! # Ok::<(), drizzle_migrations::MigrationError>(())
//! ```

use crate::mysql::collection::MySQLDDL;
use crate::postgres::collection::PostgresDDL;
use crate::renames::CreateHint;
use crate::schema::{Schema, Snapshot};
use crate::sqlite::collection::SQLiteDDL;
use crate::version::ORIGIN_UUID;
use crate::writer::MigrationError;
use std::borrow::Cow;
use std::io::{self, Write};

/// The result of a diff: SQL statements, warnings, and the new snapshot.
#[derive(Clone, Debug)]
pub struct Plan {
    /// SQL statements for the migration.
    pub statements: Vec<String>,
    /// Warning messages emitted while planning the migration.
    pub warnings: Vec<String>,
    /// Schema snapshot after this migration is applied.
    pub snapshot: Snapshot,
    /// The tables and columns the statements drop. Each loses data when the
    /// database holds rows in it, which is why drizzle-kit's push asks first.
    pub data_loss: Vec<DataLoss>,
}

/// A table or column a [`Plan`] drops.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DataLoss {
    /// PostgreSQL schema or MySQL database; `None` for SQLite and an
    /// unqualified MySQL table.
    pub schema: Option<String>,
    /// The table dropped, or the table the column is dropped from.
    pub table: String,
    /// The dropped column, or `None` when the whole table is dropped.
    pub column: Option<String>,
}

impl DataLoss {
    /// A query returning one integer: the rows of a dropped table, or the
    /// rows where a dropped column is not NULL.
    #[must_use]
    pub fn count_sql(&self, dialect: drizzle_types::Dialect) -> String {
        let quote = |ident: &str| match dialect {
            drizzle_types::Dialect::MySQL => format!("`{}`", ident.replace('`', "``")),
            drizzle_types::Dialect::SQLite | drizzle_types::Dialect::PostgreSQL => {
                format!("\"{}\"", ident.replace('"', "\"\""))
            }
        };
        let table = match &self.schema {
            Some(schema) => format!("{}.{}", quote(schema), quote(&self.table)),
            None => quote(&self.table),
        };
        match &self.column {
            Some(column) => format!(
                "SELECT COUNT(*) FROM {table} WHERE {} IS NOT NULL",
                quote(column)
            ),
            None => format!("SELECT COUNT(*) FROM {table}"),
        }
    }
}

impl std::fmt::Display for DataLoss {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let kind = if self.column.is_some() {
            "column"
        } else {
            "table"
        };
        write!(f, "{kind} `")?;
        if let Some(schema) = &self.schema {
            write!(f, "{schema}.")?;
        }
        write!(f, "{}", self.table)?;
        if let Some(column) = &self.column {
            write!(f, ".{column}")?;
        }
        f.write_str("`")
    }
}

/// The tables and columns of `prev` that `current` no longer has, once the
/// rename hints are applied.
fn data_loss(prev: &Snapshot, current: &Snapshot, renames: &RenameHints) -> Vec<DataLoss> {
    type Table = (Option<String>, String);
    fn shape(snapshot: &Snapshot) -> (Vec<Table>, Vec<(Table, String)>) {
        let mut tables = Vec::new();
        let mut columns = Vec::new();
        match snapshot {
            Snapshot::Sqlite(snapshot) => {
                for entity in &snapshot.ddl {
                    match entity {
                        crate::sqlite::SqliteEntity::Table(table) => {
                            tables.push((None, table.name.to_string()));
                        }
                        crate::sqlite::SqliteEntity::Column(column) => columns
                            .push(((None, column.table.to_string()), column.name.to_string())),
                        _ => {}
                    }
                }
            }
            Snapshot::Postgres(snapshot) => {
                for entity in &snapshot.ddl {
                    match entity {
                        crate::postgres::PostgresEntity::Table(table) => {
                            tables.push((Some(table.schema.to_string()), table.name.to_string()));
                        }
                        crate::postgres::PostgresEntity::Column(column) => columns.push((
                            (Some(column.schema.to_string()), column.table.to_string()),
                            column.name.to_string(),
                        )),
                        _ => {}
                    }
                }
            }
            Snapshot::MySQL(snapshot) => {
                for entity in &snapshot.ddl {
                    match entity {
                        crate::mysql::MySQLEntity::Table(table) => tables.push((
                            table.database.as_ref().map(ToString::to_string),
                            table.name.to_string(),
                        )),
                        crate::mysql::MySQLEntity::Column(column) => columns.push((
                            (
                                column.database.as_ref().map(ToString::to_string),
                                column.table.to_string(),
                            ),
                            column.name.to_string(),
                        )),
                        _ => {}
                    }
                }
            }
        }
        (tables, columns)
    }
    // A hint without a schema names the default one: `public`, or none.
    let in_schema = |hint: Option<&String>, schema: &Option<String>| match hint {
        Some(hint) => schema.as_ref() == Some(hint),
        None => {
            schema.as_deref().is_none_or(|schema| schema == "public")
                || matches!(prev, Snapshot::MySQL(_))
        }
    };
    let rename_table = |table: &mut Table| {
        if let Some(hint) = renames
            .schema_renames
            .iter()
            .find(|hint| table.0.as_ref() == Some(&hint.from))
        {
            table.0 = Some(hint.to.clone());
        }
        if let Some(hint) = renames
            .table_renames
            .iter()
            .find(|hint| hint.from == table.1 && in_schema(hint.schema.as_ref(), &table.0))
        {
            table.1.clone_from(&hint.to);
        }
    };

    let (mut prev_tables, mut prev_columns) = shape(prev);
    let (tables, columns) = shape(current);
    prev_tables.iter_mut().for_each(rename_table);
    for (table, column) in &mut prev_columns {
        rename_table(table);
        if let Some(hint) = renames.column_renames.iter().find(|hint| {
            hint.from == *column
                && hint.table == table.1
                && in_schema(hint.schema.as_ref(), &table.0)
        }) {
            column.clone_from(&hint.to);
        }
    }

    let dropped_tables = prev_tables
        .into_iter()
        .filter(|table| !tables.contains(table))
        .map(|(schema, table)| DataLoss {
            schema,
            table,
            column: None,
        });
    let dropped_columns = prev_columns
        .into_iter()
        .filter(|(table, column)| {
            tables.contains(table)
                && !columns
                    .iter()
                    .any(|kept| kept.0 == *table && kept.1 == *column)
        })
        .map(|((schema, table), column)| DataLoss {
            schema,
            table,
            column: Some(column),
        });
    dropped_tables.chain(dropped_columns).collect()
}

impl Plan {
    /// Returns `true` when every statement is blank (nothing to run).
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.statements.is_empty()
            || self
                .statements
                .iter()
                .all(|statement| statement.trim().is_empty())
    }

    /// Joins the statements with `--> statement-breakpoint` lines, the format
    /// used in `migration.sql`.
    #[must_use]
    pub fn to_sql(&self) -> String {
        self.statements.join("\n--> statement-breakpoint\n")
    }

    /// Writes [`to_sql`](Self::to_sql) to `writer`.
    ///
    /// # Errors
    ///
    /// Returns the underlying [`io::Error`] if writing to `writer` fails.
    pub fn write(&self, writer: impl Write) -> io::Result<()> {
        let mut writer = writer;
        writer.write_all(self.to_sql().as_bytes())
    }
}

/// Rename hints that turn a drop + create into a rename.
///
/// The differ detects only some renames on its own; without a hint, other
/// renames become a drop plus a create, which loses data. Usually built
/// through [`DiffOptions`]'s `rename_*` methods.
///
/// Hints also record the answers to [`rename_questions`](crate::rename_questions):
/// a rename answer is a `rename_*` hint, and a "create" answer is a
/// [`CreateHint`] in [`creates`](Self::creates). See
/// [`RenameQuestion::answer`](crate::RenameQuestion::answer).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RenameHints {
    /// PostgreSQL schema rename hints. MySQL rejects these because databases
    /// are migration scope rather than managed schema entities.
    pub schema_renames: Vec<SchemaRenameHint>,
    /// PostgreSQL enum rename hints (`ALTER TYPE ... RENAME TO`).
    pub enum_renames: Vec<EnumRenameHint>,
    /// Table rename hints.
    pub table_renames: Vec<TableRenameHint>,
    /// Column rename hints.
    pub column_renames: Vec<ColumnRenameHint>,
    /// PostgreSQL index rename hints (`ALTER INDEX ... RENAME TO`).
    pub index_renames: Vec<IndexRenameHint>,
    /// PostgreSQL constraint rename hints (`ALTER TABLE ... RENAME
    /// CONSTRAINT`).
    pub constraint_renames: Vec<ConstraintRenameHint>,
    /// View rename hints.
    pub view_renames: Vec<ViewRenameHint>,
    /// Entities declared newly created rather than renamed, so
    /// [`rename_questions`](crate::rename_questions) stops asking about them
    /// and [`diff_with`] creates them.
    pub creates: Vec<CreateHint>,
}

impl RenameHints {
    /// Creates an empty set of hints.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a PostgreSQL enum rename in the default schema (`public`).
    #[must_use]
    pub fn rename_enum(mut self, from: impl Into<String>, to: impl Into<String>) -> Self {
        self.enum_renames.push(EnumRenameHint {
            schema: None,
            from: from.into(),
            to: to.into(),
        });
        self
    }

    /// Adds a PostgreSQL enum rename inside `schema`.
    #[must_use]
    pub fn rename_enum_in(
        mut self,
        schema: impl Into<String>,
        from: impl Into<String>,
        to: impl Into<String>,
    ) -> Self {
        self.enum_renames.push(EnumRenameHint {
            schema: Some(schema.into()),
            from: from.into(),
            to: to.into(),
        });
        self
    }

    /// Adds a PostgreSQL index rename on `table` in the default schema
    /// (`public`).
    #[must_use]
    pub fn rename_index(
        mut self,
        table: impl Into<String>,
        from: impl Into<String>,
        to: impl Into<String>,
    ) -> Self {
        self.index_renames.push(IndexRenameHint {
            schema: None,
            table: table.into(),
            from: from.into(),
            to: to.into(),
        });
        self
    }

    /// Adds a PostgreSQL index rename on `schema.table`.
    #[must_use]
    pub fn rename_index_in(
        mut self,
        schema: impl Into<String>,
        table: impl Into<String>,
        from: impl Into<String>,
        to: impl Into<String>,
    ) -> Self {
        self.index_renames.push(IndexRenameHint {
            schema: Some(schema.into()),
            table: table.into(),
            from: from.into(),
            to: to.into(),
        });
        self
    }

    /// Adds a PostgreSQL constraint rename on `table` in the default schema
    /// (`public`).
    #[must_use]
    pub fn rename_constraint(
        mut self,
        kind: ConstraintKind,
        table: impl Into<String>,
        from: impl Into<String>,
        to: impl Into<String>,
    ) -> Self {
        self.constraint_renames.push(ConstraintRenameHint {
            kind,
            schema: None,
            table: table.into(),
            from: from.into(),
            to: to.into(),
        });
        self
    }

    /// Adds a PostgreSQL constraint rename on `schema.table`.
    #[must_use]
    pub fn rename_constraint_in(
        mut self,
        kind: ConstraintKind,
        schema: impl Into<String>,
        table: impl Into<String>,
        from: impl Into<String>,
        to: impl Into<String>,
    ) -> Self {
        self.constraint_renames.push(ConstraintRenameHint {
            kind,
            schema: Some(schema.into()),
            table: table.into(),
            from: from.into(),
            to: to.into(),
        });
        self
    }

    /// Declares `entity` newly created, so
    /// [`rename_questions`](crate::rename_questions) no longer asks whether
    /// it was renamed.
    #[must_use]
    pub fn create(mut self, entity: CreateHint) -> Self {
        self.creates.push(entity);
        self
    }

    /// Adds a PostgreSQL schema rename.
    #[must_use]
    pub fn rename_schema(mut self, from: impl Into<String>, to: impl Into<String>) -> Self {
        self.schema_renames.push(SchemaRenameHint {
            from: from.into(),
            to: to.into(),
        });
        self
    }

    /// Adds a table rename in the default schema.
    #[must_use]
    pub fn rename_table(mut self, from: impl Into<String>, to: impl Into<String>) -> Self {
        self.table_renames.push(TableRenameHint {
            schema: None,
            from: from.into(),
            to: to.into(),
        });
        self
    }

    /// Adds a table rename inside `schema` (PostgreSQL schema or MySQL
    /// database).
    #[must_use]
    pub fn rename_table_in(
        mut self,
        schema: impl Into<String>,
        from: impl Into<String>,
        to: impl Into<String>,
    ) -> Self {
        self.table_renames.push(TableRenameHint {
            schema: Some(schema.into()),
            from: from.into(),
            to: to.into(),
        });
        self
    }

    /// Adds a column rename on `table` in the default schema.
    #[must_use]
    pub fn rename_column(
        mut self,
        table: impl Into<String>,
        from: impl Into<String>,
        to: impl Into<String>,
    ) -> Self {
        self.column_renames.push(ColumnRenameHint {
            schema: None,
            table: table.into(),
            from: from.into(),
            to: to.into(),
        });
        self
    }

    /// Adds a column rename on `schema.table`.
    #[must_use]
    pub fn rename_column_in(
        mut self,
        schema: impl Into<String>,
        table: impl Into<String>,
        from: impl Into<String>,
        to: impl Into<String>,
    ) -> Self {
        self.column_renames.push(ColumnRenameHint {
            schema: Some(schema.into()),
            table: table.into(),
            from: from.into(),
            to: to.into(),
        });
        self
    }

    /// Adds a view rename in the default schema.
    #[must_use]
    pub fn rename_view(mut self, from: impl Into<String>, to: impl Into<String>) -> Self {
        self.view_renames.push(ViewRenameHint {
            schema: None,
            from: from.into(),
            to: to.into(),
        });
        self
    }

    /// Adds a view rename inside `schema`.
    #[must_use]
    pub fn rename_view_in(
        mut self,
        schema: impl Into<String>,
        from: impl Into<String>,
        to: impl Into<String>,
    ) -> Self {
        self.view_renames.push(ViewRenameHint {
            schema: Some(schema.into()),
            from: from.into(),
            to: to.into(),
        });
        self
    }
}

/// PostgreSQL schema rename hint.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SchemaRenameHint {
    /// Current schema name.
    pub from: String,
    /// New schema name.
    pub to: String,
}

/// Table rename hint.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TableRenameHint {
    /// Optional namespace: PostgreSQL schema or selected MySQL database.
    pub schema: Option<String>,
    /// Current table name.
    pub from: String,
    /// New table name.
    pub to: String,
}

/// Column rename hint.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ColumnRenameHint {
    /// Optional namespace: PostgreSQL schema or selected MySQL database.
    pub schema: Option<String>,
    /// Table containing the column.
    pub table: String,
    /// Current column name.
    pub from: String,
    /// New column name.
    pub to: String,
}

/// View rename hint.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ViewRenameHint {
    /// Optional namespace: PostgreSQL schema or selected MySQL database.
    pub schema: Option<String>,
    /// Current view name.
    pub from: String,
    /// New view name.
    pub to: String,
}

/// PostgreSQL enum rename hint.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EnumRenameHint {
    /// PostgreSQL schema; `None` means `public`.
    pub schema: Option<String>,
    /// Current enum name.
    pub from: String,
    /// New enum name.
    pub to: String,
}

/// PostgreSQL index rename hint.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IndexRenameHint {
    /// PostgreSQL schema; `None` means `public`.
    pub schema: Option<String>,
    /// Table the index is on.
    pub table: String,
    /// Current index name.
    pub from: String,
    /// New index name.
    pub to: String,
}

/// The kind of constraint a [`ConstraintRenameHint`] renames.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ConstraintKind {
    /// Unique constraint.
    Unique,
    /// Check constraint.
    Check,
    /// Primary key.
    PrimaryKey,
    /// Foreign key.
    ForeignKey,
}

/// PostgreSQL constraint rename hint.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConstraintRenameHint {
    /// Constraint kind.
    pub kind: ConstraintKind,
    /// PostgreSQL schema; `None` means `public`.
    pub schema: Option<String>,
    /// Table the constraint is on.
    pub table: String,
    /// Current constraint name.
    pub from: String,
    /// New constraint name.
    pub to: String,
}

/// Generation options for [`diff_with`] and [`diff_schemas_with`].
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DiffOptions {
    /// Answers to the rename-or-create questions: renames, and entities
    /// declared created.
    pub renames: RenameHints,
    /// When `true`, a hint that cannot be applied (unknown object, invalid
    /// name, unsupported for the dialect) is an error instead of being skipped.
    pub strict_renames: bool,
    /// Typed data movement for SQLite table rebuilds, bound to the exact
    /// predecessor snapshot.
    pub sqlite_rebuild_data: Option<crate::sqlite::SqliteRebuildDataPlanRegistry>,
    /// Live MySQL defaults used only when planning a push against an
    /// introspected database.
    pub mysql_catalog_defaults: Option<crate::mysql::MySQLCatalogDefaults>,
}

impl DiffOptions {
    /// Creates default options: no hints, non-strict.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// See [`RenameHints::rename_enum`].
    #[must_use]
    pub fn rename_enum(mut self, from: impl Into<String>, to: impl Into<String>) -> Self {
        self.renames = self.renames.rename_enum(from, to);
        self
    }

    /// See [`RenameHints::rename_enum_in`].
    #[must_use]
    pub fn rename_enum_in(
        mut self,
        schema: impl Into<String>,
        from: impl Into<String>,
        to: impl Into<String>,
    ) -> Self {
        self.renames = self.renames.rename_enum_in(schema, from, to);
        self
    }

    /// See [`RenameHints::rename_index`].
    #[must_use]
    pub fn rename_index(
        mut self,
        table: impl Into<String>,
        from: impl Into<String>,
        to: impl Into<String>,
    ) -> Self {
        self.renames = self.renames.rename_index(table, from, to);
        self
    }

    /// See [`RenameHints::rename_index_in`].
    #[must_use]
    pub fn rename_index_in(
        mut self,
        schema: impl Into<String>,
        table: impl Into<String>,
        from: impl Into<String>,
        to: impl Into<String>,
    ) -> Self {
        self.renames = self.renames.rename_index_in(schema, table, from, to);
        self
    }

    /// See [`RenameHints::rename_constraint`].
    #[must_use]
    pub fn rename_constraint(
        mut self,
        kind: ConstraintKind,
        table: impl Into<String>,
        from: impl Into<String>,
        to: impl Into<String>,
    ) -> Self {
        self.renames = self.renames.rename_constraint(kind, table, from, to);
        self
    }

    /// See [`RenameHints::rename_constraint_in`].
    #[must_use]
    pub fn rename_constraint_in(
        mut self,
        kind: ConstraintKind,
        schema: impl Into<String>,
        table: impl Into<String>,
        from: impl Into<String>,
        to: impl Into<String>,
    ) -> Self {
        self.renames = self
            .renames
            .rename_constraint_in(kind, schema, table, from, to);
        self
    }

    /// See [`RenameHints::create`].
    #[must_use]
    pub fn create(mut self, entity: CreateHint) -> Self {
        self.renames = self.renames.create(entity);
        self
    }

    /// Replaces all rename hints with `renames`.
    #[must_use]
    pub fn with_renames(mut self, renames: RenameHints) -> Self {
        self.renames = renames;
        self
    }

    /// Sets [`strict_renames`](Self::strict_renames).
    #[must_use]
    pub const fn strict_renames(mut self, strict: bool) -> Self {
        self.strict_renames = strict;
        self
    }

    /// Attaches one SQLite rebuild-data plan.
    #[must_use]
    pub fn sqlite_rebuild_data(mut self, plan: crate::sqlite::SqliteRebuildDataPlan) -> Self {
        self.sqlite_rebuild_data = Some(crate::sqlite::SqliteRebuildDataPlanRegistry::single(plan));
        self
    }

    /// Attaches a registry of SQLite rebuild-data plans.
    #[must_use]
    pub fn sqlite_rebuild_data_registry(
        mut self,
        registry: crate::sqlite::SqliteRebuildDataPlanRegistry,
    ) -> Self {
        self.sqlite_rebuild_data = Some(registry);
        self
    }

    /// Sets live MySQL catalog defaults (for push planning only).
    #[must_use]
    pub fn mysql_catalog_defaults(mut self, defaults: crate::mysql::MySQLCatalogDefaults) -> Self {
        self.mysql_catalog_defaults = Some(defaults);
        self
    }

    /// See [`RenameHints::rename_schema`].
    #[must_use]
    pub fn rename_schema(mut self, from: impl Into<String>, to: impl Into<String>) -> Self {
        self.renames = self.renames.rename_schema(from, to);
        self
    }

    /// See [`RenameHints::rename_table`].
    #[must_use]
    pub fn rename_table(mut self, from: impl Into<String>, to: impl Into<String>) -> Self {
        self.renames = self.renames.rename_table(from, to);
        self
    }

    /// See [`RenameHints::rename_table_in`].
    #[must_use]
    pub fn rename_table_in(
        mut self,
        schema: impl Into<String>,
        from: impl Into<String>,
        to: impl Into<String>,
    ) -> Self {
        self.renames = self.renames.rename_table_in(schema, from, to);
        self
    }

    /// See [`RenameHints::rename_column`].
    #[must_use]
    pub fn rename_column(
        mut self,
        table: impl Into<String>,
        from: impl Into<String>,
        to: impl Into<String>,
    ) -> Self {
        self.renames = self.renames.rename_column(table, from, to);
        self
    }

    /// See [`RenameHints::rename_column_in`].
    #[must_use]
    pub fn rename_column_in(
        mut self,
        schema: impl Into<String>,
        table: impl Into<String>,
        from: impl Into<String>,
        to: impl Into<String>,
    ) -> Self {
        self.renames = self.renames.rename_column_in(schema, table, from, to);
        self
    }

    /// See [`RenameHints::rename_view`].
    #[must_use]
    pub fn rename_view(mut self, from: impl Into<String>, to: impl Into<String>) -> Self {
        self.renames = self.renames.rename_view(from, to);
        self
    }

    /// See [`RenameHints::rename_view_in`].
    #[must_use]
    pub fn rename_view_in(
        mut self,
        schema: impl Into<String>,
        from: impl Into<String>,
        to: impl Into<String>,
    ) -> Self {
        self.renames = self.renames.rename_view_in(schema, from, to);
        self
    }
}

/// Diffs two snapshots and returns the migration [`Plan`].
///
/// Both snapshots must use the same dialect. When nothing changed, the plan
/// has no statements ([`Plan::is_empty`]). No file I/O.
///
/// To write a migration folder (`./drizzle/<tag>/...`), use
/// [`build::run`](crate::build::run).
///
/// # Examples
///
/// ```rust
/// use drizzle_migrations::{Snapshot, diff};
/// use drizzle_types::Dialect;
///
/// let plan = diff(&Snapshot::empty(Dialect::SQLite), &Snapshot::empty(Dialect::SQLite))?;
/// assert!(plan.is_empty());
/// # Ok::<(), drizzle_migrations::MigrationError>(())
/// ```
///
/// # Errors
///
/// Returns [`MigrationError::DialectMismatch`] if the two snapshots use
/// different dialects, [`MigrationError::UnansweredRenames`] if a possible
/// rename needs a hint (see [`diff_with`]), or a
/// [`MigrationError::ConfigError`] if snapshot validation or SQL rendering
/// fails.
pub fn diff(prev: &Snapshot, current: &Snapshot) -> Result<Plan, MigrationError> {
    diff_with(prev, current, &DiffOptions::default())
}

/// Diffs two snapshots using `options` (rename hints, strict mode, etc.).
///
/// A diff never guesses a rename. When a schema, enum, table or column
/// disappears and another of the same kind appears in the same scope, the
/// snapshots alone cannot tell a rename from a drop plus a create, and the
/// wrong answer loses data. Unless a hint in `options` answers that
/// question, `diff_with` returns [`MigrationError::UnansweredRenames`],
/// whose message gives the hint for each answer. An unhinted index,
/// constraint or view is dropped and created, which loses nothing.
///
/// # Examples
///
/// ```rust
/// use drizzle_migrations::{DiffOptions, MigrationError, Snapshot, diff_with, parser::SchemaParser};
/// use drizzle_types::Dialect;
///
/// let snapshot = |src: &str| {
///     Snapshot::from_parse_result(&SchemaParser::parse(src), Dialect::SQLite, None)
/// };
/// let v1 = snapshot("#[SQLiteTable] pub struct Users { #[column(primary)] pub id: i64, pub name: String }");
/// let v2 = snapshot("#[SQLiteTable] pub struct Users { #[column(primary)] pub id: i64, pub full_name: String }");
///
/// // `name` went away and `full_name` appeared: a rename, or a new column?
/// let error = diff_with(&v1, &v2, &DiffOptions::new()).unwrap_err();
/// assert!(matches!(error, MigrationError::UnansweredRenames(_)));
/// assert!(error.to_string().contains(r#".rename_column("users", "name", "full_name")"#));
///
/// let plan = diff_with(&v1, &v2, &DiffOptions::new().rename_column("users", "name", "full_name"))?;
/// assert_eq!(plan.statements, ["ALTER TABLE `users` RENAME COLUMN `name` TO `full_name`;"]);
/// # Ok::<(), drizzle_migrations::MigrationError>(())
/// ```
///
/// # Errors
///
/// Returns [`MigrationError::DialectMismatch`] if the two snapshots use
/// different dialects, [`MigrationError::UnansweredRenames`] if a possible
/// rename has no hint, or a [`MigrationError::ConfigError`] if snapshot
/// validation, SQL rendering, or strict rename handling fails.
pub fn diff_with(
    prev: &Snapshot,
    current: &Snapshot,
    options: &DiffOptions,
) -> Result<Plan, MigrationError> {
    let unanswered = crate::rename_questions(prev, current, options)?
        .into_iter()
        .filter(|question| question.kind.holds_data())
        .collect::<Vec<_>>();
    if !unanswered.is_empty() {
        return Err(MigrationError::UnansweredRenames(unanswered));
    }
    let mut preserved_names = Vec::new();
    let (statements, warnings) = match (prev, current) {
        (Snapshot::Sqlite(p), Snapshot::Sqlite(c)) => {
            if options.mysql_catalog_defaults.is_some() {
                return Err(MigrationError::ConfigError(
                    "MySQL catalog defaults cannot be used for a SQLite migration".to_string(),
                ));
            }
            let mut prev_ddl = SQLiteDDL::from_entities(p.ddl.clone());
            let cur_ddl = crate::sqlite::collection::SQLiteDDL::from_entities(c.ddl.clone());
            let mut statements = apply_sqlite_rename_hints(&mut prev_ddl, &cur_ddl, options)?;
            let mut diff =
                crate::sqlite::diff::compute_migration_with_inference(&prev_ddl, &cur_ddl, false);
            crate::sqlite::rebuild_data::apply_rebuild_data_plan(
                prev.id(),
                &prev_ddl,
                &cur_ddl,
                &mut diff.statements,
                options.sqlite_rebuild_data.as_ref(),
            )
            .map_err(MigrationError::ConfigError)?;
            diff.sql_statements =
                crate::sqlite::statements::from_json(diff.statements.clone()).sql_statements;
            statements.extend(diff.sql_statements);
            (statements, diff.warnings)
        }
        (Snapshot::Postgres(p), Snapshot::Postgres(c)) => {
            if options.mysql_catalog_defaults.is_some() {
                return Err(MigrationError::ConfigError(
                    "MySQL catalog defaults cannot be used for a PostgreSQL migration".to_string(),
                ));
            }
            if options.sqlite_rebuild_data.is_some() {
                return Err(MigrationError::ConfigError(
                    "SQLite rebuild-data plan cannot be used for a PostgreSQL migration"
                        .to_string(),
                ));
            }
            let mut prev_ddl = PostgresDDL::from_entities(p.ddl.clone());
            let cur_ddl = PostgresDDL::from_entities(c.ddl.clone());
            let mut statements = apply_postgres_rename_hints(&mut prev_ddl, &cur_ddl, options)?;
            let diff =
                crate::postgres::diff::compute_migration_with_inference(&prev_ddl, &cur_ddl, false);
            statements.extend(diff.sql_statements);
            preserved_names = diff.preserved_names;
            (statements, diff.warnings)
        }
        (Snapshot::MySQL(p), Snapshot::MySQL(c)) => {
            if options.sqlite_rebuild_data.is_some() {
                return Err(MigrationError::ConfigError(
                    "SQLite rebuild-data plan cannot be used for a MySQL migration".to_string(),
                ));
            }
            if !options.renames.schema_renames.is_empty() {
                return Err(MigrationError::ConfigError(
                    "MySQL migration scope is a selected database; database rename hints are not supported"
                        .to_string(),
                ));
            }
            let prev_ddl = MySQLDDL::try_from_entities(p.ddl.clone())
                .map_err(|error| MigrationError::ConfigError(error.to_string()))?;
            let cur_ddl = MySQLDDL::try_from_entities(c.ddl.clone())
                .map_err(|error| MigrationError::ConfigError(error.to_string()))?;
            let mysql_options = mysql_diff_options(options)?;
            let diff =
                crate::mysql::diff::compute_migration_with(&prev_ddl, &cur_ddl, &mysql_options)
                    .map_err(|error| MigrationError::ConfigError(error.to_string()))?;
            (diff.sql_statements, diff.warnings)
        }
        _ => return Err(MigrationError::DialectMismatch),
    };

    // Link the produced snapshot into the chain: it succeeds `prev`. A fresh
    // empty baseline (no entities, still pointing at the origin) keeps the
    // origin marker instead of adopting the baseline's throwaway id.
    let mut snapshot = current.clone();
    // Kept constraint names are what the database has; record them, as
    // drizzle-kit does, so later migrations address them correctly.
    if let Snapshot::Postgres(snapshot) = &mut snapshot {
        record_preserved_names(snapshot, &preserved_names);
    }
    let prev_is_origin_baseline =
        prev.is_empty() && matches!(prev.prev_ids(), [only] if only == ORIGIN_UUID);
    if prev_is_origin_baseline {
        snapshot.set_prev_ids(vec![ORIGIN_UUID.to_string()]);
    } else {
        snapshot.set_prev_ids(vec![prev.id().to_string()]);
    }

    Ok(Plan {
        statements,
        warnings,
        snapshot,
        data_loss: data_loss(prev, current, &options.renames),
    })
}

/// Renames, in `snapshot`, each entity in `preserved` from its derived name
/// to the name it keeps.
fn record_preserved_names(
    snapshot: &mut crate::postgres::PostgresSnapshot,
    preserved: &[crate::postgres::diff::PreservedName],
) {
    use crate::postgres::diff::PreservedKind;
    use drizzle_types::postgres::ddl::PostgresEntity;

    for kept in preserved {
        let matches = |schema: &str, table: &str, name: &str| {
            schema == kept.schema && table == kept.table && name == kept.derived
        };
        for entity in &mut snapshot.ddl {
            let name = match (kept.kind, entity) {
                (PreservedKind::Unique, PostgresEntity::UniqueConstraint(u))
                    if matches(&u.schema, &u.table, &u.name) =>
                {
                    &mut u.name
                }
                (PreservedKind::Check, PostgresEntity::CheckConstraint(c))
                    if matches(&c.schema, &c.table, &c.name) =>
                {
                    &mut c.name
                }
                (PreservedKind::Index, PostgresEntity::Index(i))
                    if matches(&i.schema, &i.table, &i.name) =>
                {
                    &mut i.name
                }
                (PreservedKind::PrimaryKey, PostgresEntity::PrimaryKey(p))
                    if matches(&p.schema, &p.table, &p.name) =>
                {
                    &mut p.name
                }
                (PreservedKind::ForeignKey, PostgresEntity::ForeignKey(f))
                    if matches(&f.schema, &f.table, &f.name) =>
                {
                    &mut f.name
                }
                _ => continue,
            };
            *name = kept.kept.clone().into();
        }
    }
}

/// Diffs two [`Schema`] values (for example two `#[SQLiteSchema]` structs).
///
/// Same as [`diff`] on their [`Schema::to_snapshot`] results.
///
/// # Errors
///
/// Returns [`MigrationError::DialectMismatch`] if the two schemas use
/// different dialects.
pub fn diff_schemas<From: Schema, To: Schema>(
    prev: &From,
    current: &To,
) -> Result<Plan, MigrationError> {
    let prev = prev.to_snapshot();
    let current = current.to_snapshot();
    diff(&prev, &current)
}

/// Diffs two [`Schema`] values using `options`.
///
/// Same as [`diff_with`] on their [`Schema::to_snapshot`] results.
///
/// # Examples
///
/// ```rust,no_run
/// use drizzle_migrations::{DiffOptions, Schema, Snapshot, diff_schemas_with};
/// use drizzle_types::Dialect;
///
/// # #[derive(Default)]
/// # struct FromSchema;
/// # #[derive(Default)]
/// # struct ToSchema;
/// # impl Schema for FromSchema {
/// #     fn to_snapshot(&self) -> Snapshot { Snapshot::empty(Dialect::SQLite) }
/// #     fn dialect(&self) -> Dialect { Dialect::SQLite }
/// # }
/// # impl Schema for ToSchema {
/// #     fn to_snapshot(&self) -> Snapshot { Snapshot::empty(Dialect::SQLite) }
/// #     fn dialect(&self) -> Dialect { Dialect::SQLite }
/// # }
/// let migration = diff_schemas_with(
///     &FromSchema,
///     &ToSchema,
///     &DiffOptions::new().rename_column("users", "displayName", "display_name"),
/// )?;
/// # let _ = migration;
/// # Ok::<(), drizzle_migrations::MigrationError>(())
/// ```
///
/// # Errors
///
/// Returns [`MigrationError::DialectMismatch`] if the two schemas use
/// different dialects, or a [`MigrationError::ConfigError`] if applying
/// rename hints fails under strict mode.
pub fn diff_schemas_with<From: Schema, To: Schema>(
    prev: &From,
    current: &To,
    options: &DiffOptions,
) -> Result<Plan, MigrationError> {
    let prev = prev.to_snapshot();
    let current = current.to_snapshot();
    diff_with(&prev, &current, options)
}

/// Maps [`DiffOptions`] onto the MySQL differ's options.
pub(crate) fn mysql_diff_options(
    options: &DiffOptions,
) -> Result<crate::mysql::diff::DiffOptions, MigrationError> {
    if options.strict_renames
        && (!options.renames.enum_renames.is_empty()
            || !options.renames.index_renames.is_empty()
            || !options.renames.constraint_renames.is_empty())
    {
        return Err(MigrationError::ConfigError(
            "mysql rename_enum, rename_index and rename_constraint hints are not supported"
                .to_string(),
        ));
    }
    Ok(crate::mysql::diff::DiffOptions {
        strict_renames: options.strict_renames,
        catalog_defaults: options.mysql_catalog_defaults.clone(),
        renames: crate::mysql::diff::RenameHints {
            tables: options
                .renames
                .table_renames
                .iter()
                .map(|hint| crate::mysql::diff::TableRename {
                    database: hint.schema.clone(),
                    from: hint.from.clone(),
                    to: hint.to.clone(),
                })
                .collect(),
            columns: options
                .renames
                .column_renames
                .iter()
                .map(|hint| crate::mysql::diff::ColumnRename {
                    database: hint.schema.clone(),
                    table: hint.table.clone(),
                    from: hint.from.clone(),
                    to: hint.to.clone(),
                })
                .collect(),
            views: options
                .renames
                .view_renames
                .iter()
                .map(|hint| crate::mysql::diff::ViewRename {
                    database: hint.schema.clone(),
                    from: hint.from.clone(),
                    to: hint.to.clone(),
                })
                .collect(),
        },
    })
}

pub(crate) fn apply_sqlite_rename_hints(
    prev: &mut SQLiteDDL,
    cur: &SQLiteDDL,
    options: &DiffOptions,
) -> Result<Vec<String>, MigrationError> {
    let mut statements = Vec::new();

    if !options.renames.schema_renames.is_empty() && options.strict_renames {
        return Err(MigrationError::ConfigError(
            "sqlite rename_schema hint is not supported".to_string(),
        ));
    }

    if options.strict_renames
        && (!options.renames.enum_renames.is_empty()
            || !options.renames.index_renames.is_empty()
            || !options.renames.constraint_renames.is_empty())
    {
        return Err(MigrationError::ConfigError(
            "sqlite rename_enum, rename_index and rename_constraint hints are not supported"
                .to_string(),
        ));
    }

    for hint in &options.renames.table_renames {
        if hint.schema.is_some() {
            if options.strict_renames {
                return Err(MigrationError::ConfigError(
                    "sqlite rename_table hint does not support schema".to_string(),
                ));
            }
            continue;
        }

        if !valid_rename_name(&hint.from) || !valid_rename_name(&hint.to) || hint.from == hint.to {
            if options.strict_renames {
                return Err(MigrationError::ConfigError(format!(
                    "invalid sqlite table rename hint: {} -> {}",
                    hint.from, hint.to
                )));
            }
            continue;
        }

        let can_apply = prev.tables.one(&hint.from).is_some()
            && cur.tables.one(&hint.to).is_some()
            && prev.tables.one(&hint.to).is_none();

        if !can_apply {
            if options.strict_renames {
                return Err(MigrationError::ConfigError(format!(
                    "sqlite table rename hint did not match snapshots: {} -> {}",
                    hint.from, hint.to
                )));
            }
            continue;
        }

        statements.push(format!(
            "ALTER TABLE `{}` RENAME TO `{}`;",
            hint.from.replace('`', "``"),
            hint.to.replace('`', "``")
        ));
        apply_sqlite_table_rename(prev, &hint.from, &hint.to);
    }

    for hint in &options.renames.column_renames {
        if hint.schema.is_some() {
            if options.strict_renames {
                return Err(MigrationError::ConfigError(
                    "sqlite rename_column hint does not support schema".to_string(),
                ));
            }
            continue;
        }

        if !valid_rename_name(&hint.table)
            || !valid_rename_name(&hint.from)
            || !valid_rename_name(&hint.to)
            || hint.from == hint.to
        {
            if options.strict_renames {
                return Err(MigrationError::ConfigError(format!(
                    "invalid sqlite column rename hint: {}.{} -> {}",
                    hint.table, hint.from, hint.to
                )));
            }
            continue;
        }

        let can_apply = prev.columns.one(&hint.table, &hint.from).is_some()
            && cur.columns.one(&hint.table, &hint.to).is_some()
            && prev.columns.one(&hint.table, &hint.to).is_none();

        if !can_apply {
            if options.strict_renames {
                return Err(MigrationError::ConfigError(format!(
                    "sqlite column rename hint did not match snapshots: {}.{} -> {}",
                    hint.table, hint.from, hint.to
                )));
            }
            continue;
        }

        statements.push(format!(
            "ALTER TABLE `{}` RENAME COLUMN `{}` TO `{}`;",
            hint.table.replace('`', "``"),
            hint.from.replace('`', "``"),
            hint.to.replace('`', "``")
        ));
        apply_sqlite_column_rename(prev, &hint.table, &hint.from, &hint.to);
    }

    for hint in &options.renames.view_renames {
        if hint.schema.is_some() {
            if options.strict_renames {
                return Err(MigrationError::ConfigError(
                    "sqlite rename_view hint does not support schema".to_string(),
                ));
            }
            continue;
        }
        let current = cur.views.one(&hint.to).cloned();
        let can_apply = valid_rename_name(&hint.from)
            && valid_rename_name(&hint.to)
            && hint.from != hint.to
            && prev.views.one(&hint.from).is_some()
            && current.is_some()
            && prev.views.one(&hint.to).is_none();
        if !can_apply {
            if options.strict_renames {
                return Err(MigrationError::ConfigError(format!(
                    "sqlite view rename hint did not match snapshots: {} -> {}",
                    hint.from, hint.to
                )));
            }
            continue;
        }

        let current = current.expect("validated current SQLite view");
        statements.push(format!("DROP VIEW `{}`;", hint.from.replace('`', "``")));
        statements.push(current.create_view_sql());
        if let Some(previous) = prev
            .views
            .list_mut()
            .iter_mut()
            .find(|view| view.name.as_ref() == hint.from)
        {
            *previous = current;
        }
    }

    Ok(statements)
}

pub(crate) fn apply_postgres_rename_hints(
    prev: &mut PostgresDDL,
    cur: &PostgresDDL,
    options: &DiffOptions,
) -> Result<Vec<String>, MigrationError> {
    let mut statements = Vec::new();

    for hint in &options.renames.schema_renames {
        if !valid_rename_name(&hint.from) || !valid_rename_name(&hint.to) || hint.from == hint.to {
            if options.strict_renames {
                return Err(MigrationError::ConfigError(format!(
                    "invalid postgres schema rename hint: {} -> {}",
                    hint.from, hint.to
                )));
            }
            continue;
        }

        let can_apply = prev.schemas.one(&hint.from).is_some()
            && cur.schemas.one(&hint.to).is_some()
            && prev.schemas.one(&hint.to).is_none();

        if !can_apply {
            if options.strict_renames {
                return Err(MigrationError::ConfigError(format!(
                    "postgres schema rename hint did not match snapshots: {} -> {}",
                    hint.from, hint.to
                )));
            }
            continue;
        }

        statements.push(format!(
            "ALTER SCHEMA \"{}\" RENAME TO \"{}\";",
            hint.from, hint.to
        ));
        apply_postgres_schema_rename(prev, &hint.from, &hint.to);
    }

    for hint in &options.renames.enum_renames {
        let schema = hint.schema.as_deref().unwrap_or("public");
        let can_apply = valid_rename_name(schema)
            && valid_rename_name(&hint.from)
            && valid_rename_name(&hint.to)
            && hint.from != hint.to
            && prev.enums.one(schema, &hint.from).is_some()
            && cur.enums.one(schema, &hint.to).is_some()
            && prev.enums.one(schema, &hint.to).is_none();
        if !can_apply {
            if options.strict_renames {
                return Err(MigrationError::ConfigError(format!(
                    "postgres enum rename hint did not match snapshots: {}.{} -> {}",
                    schema, hint.from, hint.to
                )));
            }
            continue;
        }

        statements.push(format!(
            "ALTER TYPE {} RENAME TO {};",
            pg_qualified(schema, &hint.from),
            pg_ident(&hint.to)
        ));
        apply_postgres_enum_rename(prev, schema, &hint.from, &hint.to);
    }

    for hint in &options.renames.table_renames {
        let schema = hint.schema.as_deref().unwrap_or("public");
        if !valid_rename_name(schema)
            || !valid_rename_name(&hint.from)
            || !valid_rename_name(&hint.to)
            || hint.from == hint.to
        {
            if options.strict_renames {
                return Err(MigrationError::ConfigError(format!(
                    "invalid postgres table rename hint: {}.{} -> {}",
                    schema, hint.from, hint.to
                )));
            }
            continue;
        }

        let can_apply = prev.tables.one(schema, &hint.from).is_some()
            && cur.tables.one(schema, &hint.to).is_some()
            && prev.tables.one(schema, &hint.to).is_none();

        if !can_apply {
            if options.strict_renames {
                return Err(MigrationError::ConfigError(format!(
                    "postgres table rename hint did not match snapshots: {}.{} -> {}",
                    schema, hint.from, hint.to
                )));
            }
            continue;
        }

        statements.push(format!(
            "ALTER TABLE {} RENAME TO {};",
            pg_qualified(schema, &hint.from),
            pg_ident(&hint.to)
        ));
        apply_postgres_table_rename(prev, schema, &hint.from, &hint.to);
    }

    for hint in &options.renames.column_renames {
        let schema = hint.schema.as_deref().unwrap_or("public");
        if !valid_rename_name(schema)
            || !valid_rename_name(&hint.table)
            || !valid_rename_name(&hint.from)
            || !valid_rename_name(&hint.to)
            || hint.from == hint.to
        {
            if options.strict_renames {
                return Err(MigrationError::ConfigError(format!(
                    "invalid postgres column rename hint: {}.{}.{} -> {}",
                    schema, hint.table, hint.from, hint.to
                )));
            }
            continue;
        }

        let can_apply = prev.columns.one(schema, &hint.table, &hint.from).is_some()
            && cur.columns.one(schema, &hint.table, &hint.to).is_some()
            && prev.columns.one(schema, &hint.table, &hint.to).is_none();

        if !can_apply {
            if options.strict_renames {
                return Err(MigrationError::ConfigError(format!(
                    "postgres column rename hint did not match snapshots: {}.{}.{} -> {}",
                    schema, hint.table, hint.from, hint.to
                )));
            }
            continue;
        }

        statements.push(format!(
            "ALTER TABLE {} RENAME COLUMN {} TO {};",
            pg_qualified(schema, &hint.table),
            pg_ident(&hint.from),
            pg_ident(&hint.to)
        ));
        apply_postgres_column_rename(prev, schema, &hint.table, &hint.from, &hint.to);
    }

    for hint in &options.renames.constraint_renames {
        let schema = hint.schema.as_deref().unwrap_or("public");
        let (table, from, to) = (hint.table.as_str(), hint.from.as_str(), hint.to.as_str());
        let can_apply = valid_rename_name(schema)
            && valid_rename_name(table)
            && valid_rename_name(from)
            && valid_rename_name(to)
            && from != to
            && postgres_constraint_exists(prev, hint.kind, schema, table, from)
            && !postgres_constraint_exists(prev, hint.kind, schema, table, to)
            && postgres_constraint_exists(cur, hint.kind, schema, table, to);
        if !can_apply {
            if options.strict_renames {
                return Err(MigrationError::ConfigError(format!(
                    "postgres {:?} rename hint did not match snapshots: {schema}.{table}.{from} -> {to}",
                    hint.kind
                )));
            }
            continue;
        }

        statements.push(format!(
            "ALTER TABLE {} RENAME CONSTRAINT {} TO {};",
            pg_qualified(schema, table),
            pg_ident(from),
            pg_ident(to)
        ));
        rename_postgres_constraint(prev, hint.kind, schema, table, from, to);
    }

    for hint in &options.renames.index_renames {
        let schema = hint.schema.as_deref().unwrap_or("public");
        let on_table = |index: Option<&crate::postgres::ddl::Index>| {
            index.is_some_and(|index| index.table.as_ref() == hint.table)
        };
        let can_apply = valid_rename_name(schema)
            && valid_rename_name(&hint.table)
            && valid_rename_name(&hint.from)
            && valid_rename_name(&hint.to)
            && hint.from != hint.to
            && on_table(prev.indexes.one(schema, &hint.from))
            && on_table(cur.indexes.one(schema, &hint.to))
            && prev.indexes.one(schema, &hint.to).is_none();
        if !can_apply {
            if options.strict_renames {
                return Err(MigrationError::ConfigError(format!(
                    "postgres index rename hint did not match snapshots: {}.{}.{} -> {}",
                    schema, hint.table, hint.from, hint.to
                )));
            }
            continue;
        }

        statements.push(format!(
            "ALTER INDEX {} RENAME TO {};",
            pg_qualified(schema, &hint.from),
            pg_ident(&hint.to)
        ));
        if let Some(index) = prev
            .indexes
            .list_mut()
            .iter_mut()
            .find(|index| index.schema.as_ref() == schema && index.name.as_ref() == hint.from)
        {
            index.name = Cow::Owned(hint.to.clone());
        }
    }

    for hint in &options.renames.view_renames {
        let schema = hint.schema.as_deref().unwrap_or("public");
        let current = cur.views.one(schema, &hint.to);
        let previous = prev.views.one(schema, &hint.from);
        let can_apply = valid_rename_name(schema)
            && valid_rename_name(&hint.from)
            && valid_rename_name(&hint.to)
            && hint.from != hint.to
            && previous.is_some()
            && current.is_some()
            && prev.views.one(schema, &hint.to).is_none();
        if !can_apply {
            if options.strict_renames {
                return Err(MigrationError::ConfigError(format!(
                    "postgres view rename hint did not match snapshots: {}.{} -> {}",
                    schema, hint.from, hint.to
                )));
            }
            continue;
        }

        let kind = if previous.is_some_and(|view| view.materialized) {
            "MATERIALIZED VIEW"
        } else {
            "VIEW"
        };
        statements.push(format!(
            "ALTER {kind} {} RENAME TO {};",
            pg_qualified(schema, &hint.from),
            pg_ident(&hint.to)
        ));
        if let Some(previous) = prev
            .views
            .list_mut()
            .iter_mut()
            .find(|view| view.schema.as_ref() == schema && view.name.as_ref() == hint.from)
        {
            previous.name = Cow::Owned(hint.to.clone());
        }
    }

    Ok(statements)
}

fn apply_sqlite_table_rename(ddl: &mut SQLiteDDL, from: &str, to: &str) {
    let to = to.to_string();

    if let Some(t) = ddl
        .tables
        .list_mut()
        .iter_mut()
        .find(|t| t.name.as_ref() == from)
    {
        t.name = to.clone().into();
    }

    for c in ddl
        .columns
        .list_mut()
        .iter_mut()
        .filter(|c| c.table.as_ref() == from)
    {
        c.table = to.clone().into();
    }

    for pk in ddl
        .pks
        .list_mut()
        .iter_mut()
        .filter(|pk| pk.table.as_ref() == from)
    {
        pk.table = to.clone().into();
    }

    for u in ddl
        .uniques
        .list_mut()
        .iter_mut()
        .filter(|u| u.table.as_ref() == from)
    {
        u.table = to.clone().into();
    }

    for fk in ddl.fks.list_mut().iter_mut() {
        if fk.table.as_ref() == from {
            fk.table = to.clone().into();
        }
        if fk.table_to.as_ref() == from {
            fk.table_to = to.clone().into();
        }
    }

    for idx in ddl
        .indexes
        .list_mut()
        .iter_mut()
        .filter(|i| i.table.as_ref() == from)
    {
        idx.table = to.clone().into();
    }

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

    if let Some(c) = ddl
        .columns
        .list_mut()
        .iter_mut()
        .find(|c| c.table.as_ref() == table && c.name.as_ref() == from)
    {
        c.name = to.clone().into();
    }

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
    let to = to.to_string();

    for table in ddl.tables.list_mut() {
        if table.schema.as_ref() == schema && table.name.as_ref() == from {
            table.name = to.clone().into();
        }

        if table.schema.as_ref() == schema
            && let Some(inherits) = &mut table.inherits
        {
            if inherits.as_ref() == from {
                *inherits = to.clone().into();
            } else if inherits.as_ref() == format!("{schema}.{from}") {
                *inherits = format!("{schema}.{to}").into();
            }
        }
    }

    for c in ddl
        .columns
        .list_mut()
        .iter_mut()
        .filter(|c| c.schema.as_ref() == schema && c.table.as_ref() == from)
    {
        c.table = to.clone().into();
    }

    for pk in ddl
        .pks
        .list_mut()
        .iter_mut()
        .filter(|pk| pk.schema.as_ref() == schema && pk.table.as_ref() == from)
    {
        pk.table = to.clone().into();
    }

    for u in ddl
        .uniques
        .list_mut()
        .iter_mut()
        .filter(|u| u.schema.as_ref() == schema && u.table.as_ref() == from)
    {
        u.table = to.clone().into();
    }

    for fk in ddl.fks.list_mut().iter_mut() {
        if fk.schema.as_ref() == schema && fk.table.as_ref() == from {
            fk.table = to.clone().into();
        }
        if fk.schema_to.as_ref() == schema && fk.table_to.as_ref() == from {
            fk.table_to = to.clone().into();
        }
    }

    for idx in ddl
        .indexes
        .list_mut()
        .iter_mut()
        .filter(|i| i.schema.as_ref() == schema && i.table.as_ref() == from)
    {
        idx.table = to.clone().into();
    }

    for chk in ddl
        .checks
        .list_mut()
        .iter_mut()
        .filter(|c| c.schema.as_ref() == schema && c.table.as_ref() == from)
    {
        chk.table = to.clone().into();
    }

    for policy in ddl
        .policies
        .list_mut()
        .iter_mut()
        .filter(|p| p.schema.as_ref() == schema && p.table.as_ref() == from)
    {
        policy.table = to.clone().into();
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

    for c in ddl.columns.list_mut().iter_mut() {
        if c.schema.as_ref() == schema && c.table.as_ref() == table && c.name.as_ref() == from {
            c.name = to.clone().into();
        }
    }

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

fn valid_rename_name(name: &str) -> bool {
    !name.trim().is_empty()
}

/// Quotes a PostgreSQL identifier.
fn pg_ident(name: &str) -> String {
    format!("\"{}\"", name.replace('"', "\"\""))
}

/// Whether `ddl` has a constraint of `kind` named `name` on `schema.table`.
fn postgres_constraint_exists(
    ddl: &PostgresDDL,
    kind: ConstraintKind,
    schema: &str,
    table: &str,
    name: &str,
) -> bool {
    let here = |s: &str, t: &str, n: &str| s == schema && t == table && n == name;
    match kind {
        ConstraintKind::Unique => ddl
            .uniques
            .list()
            .iter()
            .any(|c| here(&c.schema, &c.table, &c.name)),
        ConstraintKind::Check => ddl
            .checks
            .list()
            .iter()
            .any(|c| here(&c.schema, &c.table, &c.name)),
        ConstraintKind::PrimaryKey => ddl
            .pks
            .list()
            .iter()
            .any(|c| here(&c.schema, &c.table, &c.name)),
        ConstraintKind::ForeignKey => ddl
            .fks
            .list()
            .iter()
            .any(|c| here(&c.schema, &c.table, &c.name)),
    }
}

/// Renames constraint `from` of `kind` on `schema.table` to `to` in `ddl`.
fn rename_postgres_constraint(
    ddl: &mut PostgresDDL,
    kind: ConstraintKind,
    schema: &str,
    table: &str,
    from: &str,
    to: &str,
) {
    let here = |s: &str, t: &str, n: &str| s == schema && t == table && n == from;
    let renamed = || Cow::Owned(to.to_string());
    match kind {
        ConstraintKind::Unique => {
            for c in ddl.uniques.list_mut() {
                if here(&c.schema, &c.table, &c.name) {
                    c.name = renamed();
                }
            }
        }
        ConstraintKind::Check => {
            for c in ddl.checks.list_mut() {
                if here(&c.schema, &c.table, &c.name) {
                    c.name = renamed();
                }
            }
        }
        ConstraintKind::PrimaryKey => {
            for c in ddl.pks.list_mut() {
                if here(&c.schema, &c.table, &c.name) {
                    c.name = renamed();
                }
            }
        }
        ConstraintKind::ForeignKey => {
            for c in ddl.fks.list_mut() {
                if here(&c.schema, &c.table, &c.name) {
                    c.name = renamed();
                }
            }
        }
    }
}

/// `"schema"."name"`, or just `"name"` in `public`, as drizzle-kit writes it.
fn pg_qualified(schema: &str, name: &str) -> String {
    if schema == "public" {
        pg_ident(name)
    } else {
        format!("{}.{}", pg_ident(schema), pg_ident(name))
    }
}

/// Renames enum `schema.from` to `to` in `ddl`, along with the columns that
/// use it.
fn apply_postgres_enum_rename(ddl: &mut PostgresDDL, schema: &str, from: &str, to: &str) {
    for enum_ in ddl.enums.list_mut() {
        if enum_.schema.as_ref() == schema && enum_.name.as_ref() == from {
            enum_.name = Cow::Owned(to.to_string());
        }
    }
    for column in ddl.columns.list_mut() {
        if column.sql_type.as_ref() == from
            && column.type_schema.as_deref().unwrap_or("public") == schema
        {
            column.sql_type = Cow::Owned(to.to_string());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mysql::{MySQLEntity, MySQLSnapshot, View as MySQLView};
    use crate::postgres::PostgresSnapshot;
    use crate::postgres::ddl::{
        Column as PgColumn, PostgresEntity, Schema as PgSchema, Table as PgTable, View as PgView,
    };
    use crate::schema::Schema as MigrationSchema;
    use crate::sqlite::SQLiteSnapshot;
    use crate::sqlite::ddl::{Column, SqliteEntity, Table, View as SqliteView};

    #[derive(Default)]
    struct EmptySqliteSchema;

    impl MigrationSchema for EmptySqliteSchema {
        fn dialect(&self) -> drizzle_types::Dialect {
            drizzle_types::Dialect::SQLite
        }

        fn to_snapshot(&self) -> Snapshot {
            Snapshot::empty(drizzle_types::Dialect::SQLite)
        }
    }

    #[test]
    fn test_generate_empty_to_empty_for_every_dialect() {
        for dialect in [
            drizzle_types::Dialect::SQLite,
            drizzle_types::Dialect::PostgreSQL,
            drizzle_types::Dialect::MySQL,
        ] {
            let prev = Snapshot::empty(dialect);
            let cur = Snapshot::empty(dialect);
            let migration = diff(&prev, &cur).unwrap();
            assert!(
                migration.statements.is_empty(),
                "{dialect:?} empty snapshots must have no diff"
            );
        }
    }

    #[test]
    fn shared_view_rename_hint_uses_each_dialects_supported_operation() {
        let mut sqlite_previous = SQLiteSnapshot::new();
        let mut sqlite_old = SqliteView::new("old_view");
        sqlite_old.definition = Some(Cow::Borrowed("SELECT 1"));
        sqlite_previous.add_entity(SqliteEntity::View(sqlite_old));
        let mut sqlite_current = SQLiteSnapshot::new();
        let mut sqlite_new = SqliteView::new("new_view");
        sqlite_new.definition = Some(Cow::Borrowed("SELECT 1"));
        sqlite_current.add_entity(SqliteEntity::View(sqlite_new));

        let mut postgres_previous = PostgresSnapshot::new();
        let mut postgres_old = PgView::new("public", "old_view");
        postgres_old.definition = Some(Cow::Borrowed("SELECT 1"));
        postgres_previous.add_entity(PostgresEntity::View(postgres_old));
        let mut postgres_current = PostgresSnapshot::new();
        let mut postgres_new = PgView::new("public", "new_view");
        postgres_new.definition = Some(Cow::Borrowed("SELECT 1"));
        postgres_current.add_entity(PostgresEntity::View(postgres_new));

        let mut mysql_previous = MySQLSnapshot::new();
        mysql_previous.add_entity(MySQLEntity::View(MySQLView::new("old_view", "SELECT 1")));
        let mut mysql_current = MySQLSnapshot::new();
        mysql_current.add_entity(MySQLEntity::View(MySQLView::new("new_view", "SELECT 1")));

        let cases = [
            (
                Snapshot::Sqlite(sqlite_previous),
                Snapshot::Sqlite(sqlite_current),
                vec![
                    "DROP VIEW `old_view`;".to_string(),
                    "CREATE VIEW `new_view` AS SELECT 1;".to_string(),
                ],
            ),
            (
                Snapshot::Postgres(postgres_previous),
                Snapshot::Postgres(postgres_current),
                vec!["ALTER VIEW \"old_view\" RENAME TO \"new_view\";".to_string()],
            ),
            (
                Snapshot::MySQL(mysql_previous),
                Snapshot::MySQL(mysql_current),
                vec!["RENAME TABLE `old_view` TO `new_view`;".to_string()],
            ),
        ];

        for (previous, current, expected) in cases {
            let plan = diff_with(
                &previous,
                &current,
                &DiffOptions::new()
                    .rename_view("old_view", "new_view")
                    .strict_renames(true),
            )
            .expect("view rename hint");
            assert_eq!(plan.statements, expected, "{:?}", previous.dialect());
        }
    }

    #[test]
    fn test_generate_create_table() {
        let prev = Snapshot::empty(drizzle_types::Dialect::SQLite);

        let mut cur_snap = SQLiteSnapshot::new();
        cur_snap.add_entity(SqliteEntity::Table(Table::new("users")));
        cur_snap.add_entity(SqliteEntity::Column(
            Column::new("users", "id", "integer").not_null(),
        ));
        cur_snap.add_entity(SqliteEntity::Column(
            Column::new("users", "name", "text").not_null(),
        ));
        let cur = Snapshot::Sqlite(cur_snap);

        let migration = diff(&prev, &cur).unwrap();
        assert!(!migration.statements.is_empty());
        assert!(migration.statements[0].contains("CREATE TABLE"));
        assert!(migration.statements[0].contains("users"));
    }

    #[test]
    fn test_generate_dialect_mismatch() {
        let prev = Snapshot::empty(drizzle_types::Dialect::SQLite);
        let cur = Snapshot::empty(drizzle_types::Dialect::PostgreSQL);
        let result = diff(&prev, &cur);
        assert!(matches!(result, Err(MigrationError::DialectMismatch)));
    }

    #[test]
    fn test_diff_schemas_empty() {
        let prev = EmptySqliteSchema;
        let cur = EmptySqliteSchema;
        let migration = diff_schemas(&prev, &cur).unwrap();
        assert!(migration.statements.is_empty());
    }

    #[test]
    fn test_diff_with_sqlite_rename_hints() {
        let mut prev_snap = SQLiteSnapshot::new();
        prev_snap.add_entity(SqliteEntity::Table(Table::new("users")));
        prev_snap.add_entity(SqliteEntity::Column(
            Column::new("users", "full_name", "text").not_null(),
        ));

        let mut cur_snap = SQLiteSnapshot::new();
        cur_snap.add_entity(SqliteEntity::Table(Table::new("accounts")));
        cur_snap.add_entity(SqliteEntity::Column(
            Column::new("accounts", "display_name", "text").not_null(),
        ));

        let prev = Snapshot::Sqlite(prev_snap);
        let cur = Snapshot::Sqlite(cur_snap);

        let options = DiffOptions::new()
            .rename_table("users", "accounts")
            .rename_column("accounts", "full_name", "display_name");

        let migration = diff_with(&prev, &cur, &options).unwrap();
        assert_eq!(
            migration.statements,
            vec![
                "ALTER TABLE `users` RENAME TO `accounts`;".to_string(),
                "ALTER TABLE `accounts` RENAME COLUMN `full_name` TO `display_name`;".to_string(),
            ]
        );
    }

    #[test]
    fn test_diff_with_sqlite_table_rename_hint_and_add_column() {
        let mut prev_snap = SQLiteSnapshot::new();
        prev_snap.add_entity(SqliteEntity::Table(Table::new("users")));
        prev_snap.add_entity(SqliteEntity::Column(
            Column::new("users", "id", "integer").not_null(),
        ));

        let mut cur_snap = SQLiteSnapshot::new();
        cur_snap.add_entity(SqliteEntity::Table(Table::new("accounts")));
        cur_snap.add_entity(SqliteEntity::Column(
            Column::new("accounts", "id", "integer").not_null(),
        ));
        cur_snap.add_entity(SqliteEntity::Column(Column::new(
            "accounts", "email", "text",
        )));

        let migration = diff_with(
            &Snapshot::Sqlite(prev_snap),
            &Snapshot::Sqlite(cur_snap),
            &DiffOptions::new().rename_table("users", "accounts"),
        )
        .unwrap();

        assert_eq!(
            migration.statements,
            vec![
                "ALTER TABLE `users` RENAME TO `accounts`;".to_string(),
                "ALTER TABLE `accounts` ADD `email` TEXT;".to_string(),
            ]
        );
    }

    #[test]
    fn test_diff_with_postgres_table_rename_hint_and_add_column() {
        let mut prev_snap = PostgresSnapshot::new();
        prev_snap.add_entity(PostgresEntity::Schema(PgSchema::new("public")));
        prev_snap.add_entity(PostgresEntity::Table(PgTable::new("public", "users")));
        prev_snap.add_entity(PostgresEntity::Column(
            PgColumn::new("public", "users", "id", "integer").not_null(),
        ));

        let mut cur_snap = PostgresSnapshot::new();
        cur_snap.add_entity(PostgresEntity::Schema(PgSchema::new("public")));
        cur_snap.add_entity(PostgresEntity::Table(PgTable::new("public", "accounts")));
        cur_snap.add_entity(PostgresEntity::Column(
            PgColumn::new("public", "accounts", "id", "integer").not_null(),
        ));
        cur_snap.add_entity(PostgresEntity::Column(PgColumn::new(
            "public", "accounts", "email", "text",
        )));

        let migration = diff_with(
            &Snapshot::Postgres(prev_snap),
            &Snapshot::Postgres(cur_snap),
            &DiffOptions::new().rename_table("users", "accounts"),
        )
        .unwrap();

        assert_eq!(
            migration.statements,
            vec![
                "ALTER TABLE \"users\" RENAME TO \"accounts\";".to_string(),
                "ALTER TABLE \"accounts\" ADD COLUMN \"email\" text;".to_string(),
            ]
        );
    }

    #[test]
    fn test_diff_with_strict_rename_hints_errors() {
        let prev = Snapshot::empty(drizzle_types::Dialect::SQLite);
        let cur = Snapshot::empty(drizzle_types::Dialect::SQLite);
        let options = DiffOptions::new()
            .strict_renames(true)
            .rename_table("missing_table", "users");

        let result = diff_with(&prev, &cur, &options);
        assert!(matches!(result, Err(MigrationError::ConfigError(_))));

        let prev = Snapshot::empty(drizzle_types::Dialect::PostgreSQL);
        let cur = Snapshot::empty(drizzle_types::Dialect::PostgreSQL);
        let options = DiffOptions::new()
            .strict_renames(true)
            .rename_schema("missing_schema", "app");

        let result = diff_with(&prev, &cur, &options);
        assert!(matches!(result, Err(MigrationError::ConfigError(_))));
    }
}

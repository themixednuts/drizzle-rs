//! Rename-or-create questions, asked the way `drizzle-kit` asks them.
//!
//! When an entity disappears from the previous snapshot and another of the
//! same kind appears in the current one, the diff alone cannot tell a rename
//! from a drop plus a create, and nothing here guesses.
//! [`rename_questions`] lists the decisions so a caller (the `drizzle` CLI)
//! can ask a person, then diff with the answers.
//! [`diff_with`](crate::diff_with) refuses to run while a question about a
//! schema, enum, table or column has no answer
//! ([`MigrationError::UnansweredRenames`]).
//!
//! Questions come in drizzle-kit's order: schemas, enums, tables, columns,
//! indexes, views. Answers are recorded as [`RenameHints`]; ask again after
//! answering, because an answer changes the later questions (a table rename
//! makes its columns comparable, and a rename consumes its source, so later
//! questions stop offering it).
//!
//! ```rust
//! use drizzle_migrations::{
//!     DiffOptions, RenameAnswer, RenameKind, Snapshot, diff_with, parser::SchemaParser,
//!     rename_questions,
//! };
//! use drizzle_types::Dialect;
//!
//! let snapshot = |src: &str| {
//!     Snapshot::from_parse_result(&SchemaParser::parse(src), Dialect::SQLite, None)
//! };
//! let v1 = snapshot("#[SQLiteTable] pub struct Users { #[column(primary)] pub id: i64, pub name: String }");
//! let v2 = snapshot("#[SQLiteTable] pub struct Users { #[column(primary)] pub id: i64, pub full_name: String }");
//!
//! let mut options = DiffOptions::new();
//! loop {
//!     let questions = rename_questions(&v1, &v2, &options)?;
//!     let Some(question) = questions.first() else { break };
//!     assert_eq!(question.kind, RenameKind::Column);
//!     assert_eq!(question.name, "full_name");
//!     assert_eq!(question.candidates, ["name"]);
//!     // A real caller asks a person here.
//!     question.answer(&mut options.renames, &RenameAnswer::RenameFrom("name".into()))?;
//! }
//!
//! let plan = diff_with(&v1, &v2, &options)?;
//! assert_eq!(plan.statements, ["ALTER TABLE `users` RENAME COLUMN `name` TO `full_name`;"]);
//! # Ok::<(), drizzle_migrations::MigrationError>(())
//! ```

use std::fmt;

use crate::generate::{
    ConstraintKind, DiffOptions, RenameHints, apply_postgres_rename_hints,
    apply_sqlite_rename_hints, mysql_diff_options,
};
use crate::mysql::collection::MySQLDDL;
use crate::postgres::collection::PostgresDDL;
use crate::schema::Snapshot;
use crate::sqlite::collection::SQLiteDDL;
use crate::writer::MigrationError;

/// The kind of entity a [`RenameQuestion`] or [`CreateHint`] is about.
///
/// [`as_str`](Self::as_str) gives drizzle-kit's hint `kind` name.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[non_exhaustive]
pub enum RenameKind {
    /// PostgreSQL schema.
    Schema,
    /// PostgreSQL enum type.
    Enum,
    /// Table.
    Table,
    /// Column of a table.
    Column,
    /// PostgreSQL unique constraint.
    Unique,
    /// PostgreSQL check constraint.
    Check,
    /// PostgreSQL index.
    Index,
    /// PostgreSQL primary key.
    PrimaryKey,
    /// PostgreSQL foreign key.
    ForeignKey,
    /// View (PostgreSQL and MySQL; SQLite cannot rename a view, so it is
    /// never asked about there).
    View,
}

impl RenameKind {
    /// Every kind, in the order questions are asked.
    pub const ALL: [Self; 10] = [
        Self::Schema,
        Self::Enum,
        Self::Table,
        Self::Column,
        Self::Unique,
        Self::Check,
        Self::Index,
        Self::PrimaryKey,
        Self::ForeignKey,
        Self::View,
    ];

    /// drizzle-kit's name for the kind, as used in hint `kind` fields:
    /// `schema`, `enum`, `table`, `column`, `unique`, `check`, `index`,
    /// `primary_key`, `foreign key`, `view`.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Schema => "schema",
            Self::Enum => "enum",
            Self::Table => "table",
            Self::Column => "column",
            Self::Unique => "unique",
            Self::Check => "check",
            Self::Index => "index",
            Self::PrimaryKey => "primary_key",
            Self::ForeignKey => "foreign key",
            Self::View => "view",
        }
    }

    /// Whether answering "create" for an entity that was really renamed
    /// loses data: schemas, tables and columns hold rows, and an enum's
    /// values are stored in its columns. Indexes, constraints and views
    /// hold none, so dropping and recreating one is safe.
    #[must_use]
    pub const fn holds_data(self) -> bool {
        matches!(self, Self::Schema | Self::Enum | Self::Table | Self::Column)
    }

    /// Parses drizzle-kit's hint `kind` name (see [`as_str`](Self::as_str)).
    #[must_use]
    pub fn from_hint_kind(kind: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|candidate| candidate.as_str() == kind)
    }
}

impl fmt::Display for RenameKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// An entity declared newly created, recorded in [`RenameHints::creates`].
///
/// [`rename_questions`] does not ask about a created entity that matches
/// one. `schema` `None` means `public` on PostgreSQL and is ignored on
/// SQLite and MySQL.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct CreateHint {
    /// Entity kind.
    pub kind: RenameKind,
    /// PostgreSQL schema (`None` = `public`); unused for schemas themselves.
    pub schema: Option<String>,
    /// Table, for columns and indexes.
    pub table: Option<String>,
    /// Entity name.
    pub name: String,
}

impl CreateHint {
    /// A create hint for `kind` named `name`, in the default schema and
    /// without a table.
    #[must_use]
    pub fn new(kind: RenameKind, name: impl Into<String>) -> Self {
        Self {
            kind,
            schema: None,
            table: None,
            name: name.into(),
        }
    }

    /// Sets the PostgreSQL schema.
    #[must_use]
    pub fn in_schema(mut self, schema: impl Into<String>) -> Self {
        self.schema = Some(schema.into());
        self
    }

    /// Sets the table (columns and indexes).
    #[must_use]
    pub fn on_table(mut self, table: impl Into<String>) -> Self {
        self.table = Some(table.into());
        self
    }
}

/// One rename-or-create decision: was `name` created, or renamed from one of
/// the `candidates`?
///
/// Every candidate is an entity of the same kind, in the same scope (same
/// PostgreSQL schema; same table for columns and indexes), that the previous
/// snapshot has and the current one does not.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RenameQuestion {
    /// Entity kind.
    pub kind: RenameKind,
    /// PostgreSQL schema of the entity and its candidates. `None` for
    /// SQLite, MySQL, and schema questions.
    pub schema: Option<String>,
    /// Table of the entity and its candidates, for columns and indexes.
    pub table: Option<String>,
    /// Name of the created entity.
    pub name: String,
    /// Names of the deleted entities it may have been renamed from, in the
    /// previous snapshot's order. Never empty.
    pub candidates: Vec<String>,
}

/// The answer to a [`RenameQuestion`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RenameAnswer {
    /// The entity is new; the candidates stay deleted.
    Create,
    /// The entity was renamed from this candidate.
    RenameFrom(String),
}

impl fmt::Display for RenameQuestion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} `", self.kind)?;
        for part in [self.schema.as_deref(), self.table.as_deref()]
            .into_iter()
            .flatten()
        {
            write!(f, "{part}.")?;
        }
        write!(f, "{}`", self.name)
    }
}

impl RenameQuestion {
    /// The PostgreSQL schema to name in a hint, or `None` for the default
    /// (`public`), which hints leave out.
    fn hint_schema(&self) -> Option<&str> {
        self.schema
            .as_deref()
            .filter(|schema| *schema != "public" && self.kind != RenameKind::Schema)
    }

    /// The [`DiffOptions`] call that answers this question with a rename
    /// from `from`, as Rust source.
    fn rename_call(&self, from: &str) -> String {
        let mut args = Vec::new();
        let method = match self.kind {
            RenameKind::Schema => "rename_schema",
            RenameKind::Enum => "rename_enum",
            RenameKind::Table => "rename_table",
            RenameKind::Column => "rename_column",
            RenameKind::Index => "rename_index",
            RenameKind::View => "rename_view",
            kind => {
                let constraint = constraint_kind(kind).expect("every other kind is matched above");
                args.push(format!("ConstraintKind::{constraint:?}"));
                "rename_constraint"
            }
        };
        let schema = self.hint_schema();
        args.extend(schema.map(|schema| format!("{schema:?}")));
        args.extend(self.table.as_ref().map(|table| format!("{table:?}")));
        args.push(format!("{from:?}"));
        args.push(format!("{:?}", self.name));
        let suffix = if schema.is_some() { "_in" } else { "" };
        format!(".{method}{suffix}({})", args.join(", "))
    }

    /// The [`DiffOptions`] call that answers this question with "create",
    /// as Rust source.
    fn create_call(&self) -> String {
        let kind = format!("{:?}", self.kind);
        let mut call = format!(
            ".create(CreateHint::new(RenameKind::{kind}, {:?})",
            self.name
        );
        if let Some(schema) = self.hint_schema() {
            call.push_str(&format!(".in_schema({schema:?})"));
        }
        if let Some(table) = &self.table {
            call.push_str(&format!(".on_table({table:?})"));
        }
        call.push(')');
        call
    }

    /// The [`CreateHint`] that answers this question with "create".
    #[must_use]
    pub fn create_hint(&self) -> CreateHint {
        CreateHint {
            kind: self.kind,
            schema: self.schema.clone(),
            table: self.table.clone(),
            name: self.name.clone(),
        }
    }

    /// Records `answer` in `hints`: a create hint, or the matching
    /// `rename_*` hint.
    ///
    /// # Errors
    ///
    /// Returns [`MigrationError::ConfigError`] if `answer` renames from a
    /// name that is not one of the [`candidates`](Self::candidates).
    pub fn answer(
        &self,
        hints: &mut RenameHints,
        answer: &RenameAnswer,
    ) -> Result<(), MigrationError> {
        let from = match answer {
            RenameAnswer::Create => {
                hints.creates.push(self.create_hint());
                return Ok(());
            }
            RenameAnswer::RenameFrom(from) => from.clone(),
        };
        if !self.candidates.contains(&from) {
            return Err(MigrationError::ConfigError(format!(
                "{} `{}` cannot be renamed from `{from}`: it is not a deleted {} in scope",
                self.kind, self.name, self.kind
            )));
        }
        let to = self.name.clone();
        let table = self.table.clone().unwrap_or_default();
        let taken = std::mem::take(hints);
        *hints = match (self.kind, self.schema.clone()) {
            (RenameKind::Schema, _) => taken.rename_schema(from, to),
            (RenameKind::Enum, Some(schema)) => taken.rename_enum_in(schema, from, to),
            (RenameKind::Enum, None) => taken.rename_enum(from, to),
            (RenameKind::Table, Some(schema)) => taken.rename_table_in(schema, from, to),
            (RenameKind::Table, None) => taken.rename_table(from, to),
            (RenameKind::Column, Some(schema)) => taken.rename_column_in(schema, table, from, to),
            (RenameKind::Column, None) => taken.rename_column(table, from, to),
            (RenameKind::Index, Some(schema)) => taken.rename_index_in(schema, table, from, to),
            (RenameKind::Index, None) => taken.rename_index(table, from, to),
            (RenameKind::View, Some(schema)) => taken.rename_view_in(schema, from, to),
            (RenameKind::View, None) => taken.rename_view(from, to),
            (kind, schema) => {
                let constraint = constraint_kind(kind).expect("every other kind is matched above");
                match schema {
                    Some(schema) => taken.rename_constraint_in(constraint, schema, table, from, to),
                    None => taken.rename_constraint(constraint, table, from, to),
                }
            }
        };
        Ok(())
    }
}

/// The message of [`MigrationError::UnansweredRenames`]: each question, and
/// the [`DiffOptions`] call for each answer.
pub(crate) fn unanswered_message(questions: &[RenameQuestion]) -> String {
    let mut message = String::from(
        "cannot tell a rename from a drop plus a create, and guessing wrong loses data. \
         Answer each question with a hint on `RenameHints` or `DiffOptions` (the \
         `drizzle` CLI asks them interactively):",
    );
    for question in questions {
        let candidates = question
            .candidates
            .iter()
            .map(|candidate| format!("`{candidate}`"))
            .collect::<Vec<_>>()
            .join(" or ");
        message.push_str(&format!(
            "\n  {question}: created, or renamed from {candidates}?"
        ));
        for candidate in &question.candidates {
            message.push_str(&format!(
                "\n    renamed from `{candidate}`: {}",
                question.rename_call(candidate)
            ));
        }
        message.push_str(&format!("\n    created: {}", question.create_call()));
    }
    message
}

/// Lists the rename-or-create questions between `prev` and `current`, given
/// the hints already in `options`.
///
/// One question per created entity that has deleted candidates in scope,
/// grouped by kind in drizzle-kit's order (schemas, enums, tables, columns,
/// unique constraints, checks, indexes, primary keys, foreign keys, views).
/// The rename hints in `options` are applied first, so
/// renamed entities are neither asked about nor offered as candidates, and
/// columns are compared under their tables' new names. Entities in
/// [`RenameHints::creates`] are not asked about.
///
/// Questions after the first assume every earlier unanswered question is
/// answered "create" (drizzle-kit's non-interactive behavior). To ask
/// interactively, answer the first question
/// ([`RenameQuestion::answer`]) and call this again; the list is empty once
/// everything is decided. Then diff with the same options.
///
/// Kinds per dialect, as drizzle-kit asks them: SQLite tables and columns;
/// MySQL tables, columns, and views; PostgreSQL schemas, enums, tables,
/// columns, unique constraints, checks, indexes, primary keys, foreign keys,
/// and views (`public` is never asked about). An implicitly named
/// PostgreSQL constraint or index whose definition is unchanged keeps its
/// existing name (drizzle-kit's `preserveEntityNames`), so a table or
/// column rename asks nothing about it. A
/// PostgreSQL entity's candidates come from its own schema: moving an entity
/// between schemas is not expressible.
///
/// # Errors
///
/// Returns [`MigrationError::DialectMismatch`] if the snapshots use different
/// dialects, [`MigrationError::ConfigError`] if a snapshot is invalid, a hint
/// fails under [`strict_renames`](DiffOptions::strict_renames), or a rename
/// hint names a created entity as `to` but its `from` is not a deleted
/// entity of that kind in scope.
pub fn rename_questions(
    prev: &Snapshot,
    current: &Snapshot,
    options: &DiffOptions,
) -> Result<Vec<RenameQuestion>, MigrationError> {
    let mut asker = Asker {
        hints: &options.renames,
        questions: Vec::new(),
    };
    match (prev, current) {
        (Snapshot::Sqlite(p), Snapshot::Sqlite(c)) => {
            let mut prev = SQLiteDDL::from_entities(p.ddl.clone());
            let cur = SQLiteDDL::from_entities(c.ddl.clone());
            apply_sqlite_rename_hints(&mut prev, &cur, options)?;

            let names = |ddl: &SQLiteDDL| -> Vec<Entry> {
                ddl.tables
                    .list()
                    .iter()
                    .map(|t| Entry::new(None, None, &t.name))
                    .collect()
            };
            asker.ask(RenameKind::Table, &names(&prev), &names(&cur))?;

            let columns = |ddl: &SQLiteDDL, tables: &[Entry]| -> Vec<Entry> {
                ddl.columns
                    .list()
                    .iter()
                    .filter(|c| tables.iter().any(|t| t.name == c.table.as_ref()))
                    .map(|c| Entry::new(None, Some(&c.table), &c.name))
                    .collect()
            };
            let shared = shared(&names(&prev), &names(&cur));
            asker.ask(
                RenameKind::Column,
                &columns(&prev, &shared),
                &columns(&cur, &shared),
            )?;
        }
        (Snapshot::Postgres(p), Snapshot::Postgres(c)) => {
            let mut prev = PostgresDDL::from_entities(p.ddl.clone());
            let cur = PostgresDDL::from_entities(c.ddl.clone());
            apply_postgres_rename_hints(&mut prev, &cur, options)?;

            let schemas = |ddl: &PostgresDDL| -> Vec<Entry> {
                ddl.schemas
                    .list()
                    .iter()
                    .filter(|s| s.name != "public")
                    .map(|s| Entry::new(None, None, &s.name))
                    .collect()
            };
            asker.ask(RenameKind::Schema, &schemas(&prev), &schemas(&cur))?;

            let enums = |ddl: &PostgresDDL| -> Vec<Entry> {
                ddl.enums
                    .list()
                    .iter()
                    .map(|e| Entry::new(Some(&e.schema), None, &e.name))
                    .collect()
            };
            asker.ask(RenameKind::Enum, &enums(&prev), &enums(&cur))?;

            let tables = |ddl: &PostgresDDL| -> Vec<Entry> {
                ddl.tables
                    .list()
                    .iter()
                    .map(|t| Entry::new(Some(&t.schema), None, &t.name))
                    .collect()
            };
            asker.ask(RenameKind::Table, &tables(&prev), &tables(&cur))?;

            let shared = shared(&tables(&prev), &tables(&cur));
            let in_shared = |schema: &str, table: &str| {
                shared
                    .iter()
                    .any(|t| t.schema.as_deref() == Some(schema) && t.name == table)
            };
            let columns = |ddl: &PostgresDDL| -> Vec<Entry> {
                ddl.columns
                    .list()
                    .iter()
                    .filter(|c| in_shared(&c.schema, &c.table))
                    .map(|c| Entry::new(Some(&c.schema), Some(&c.table), &c.name))
                    .collect()
            };
            asker.ask(RenameKind::Column, &columns(&prev), &columns(&cur))?;

            // Implicitly named constraints and indexes whose definition did
            // not change keep their names (drizzle-kit's
            // `preserveEntityNames`), so they are not asked about.
            let mut cur = cur;
            crate::postgres::diff::preserve_entity_names(&prev, &mut cur);
            let on_tables = |entries: Vec<(&str, &str, &str)>| -> Vec<Entry> {
                entries
                    .into_iter()
                    .filter(|(schema, table, _)| in_shared(schema, table))
                    .map(|(schema, table, name)| Entry::new(Some(schema), Some(table), name))
                    .collect()
            };
            let uniques = |ddl: &PostgresDDL| {
                on_tables(
                    ddl.uniques
                        .list()
                        .iter()
                        .map(|u| (u.schema.as_ref(), u.table.as_ref(), u.name.as_ref()))
                        .collect(),
                )
            };
            asker.ask(RenameKind::Unique, &uniques(&prev), &uniques(&cur))?;
            let checks = |ddl: &PostgresDDL| {
                on_tables(
                    ddl.checks
                        .list()
                        .iter()
                        .map(|c| (c.schema.as_ref(), c.table.as_ref(), c.name.as_ref()))
                        .collect(),
                )
            };
            asker.ask(RenameKind::Check, &checks(&prev), &checks(&cur))?;

            // Index names are unique per schema, so an index counts as
            // created or deleted by `(schema, name)`, then is asked about on
            // its table.
            let created_or_deleted_indexes = |from: &PostgresDDL, to: &PostgresDDL| -> Vec<Entry> {
                from.indexes
                    .list()
                    .iter()
                    .filter(|i| in_shared(&i.schema, &i.table))
                    .filter(|i| to.indexes.one(&i.schema, &i.name).is_none())
                    .map(|i| Entry::new(Some(&i.schema), Some(&i.table), &i.name))
                    .collect()
            };
            asker.ask_changed(
                RenameKind::Index,
                &created_or_deleted_indexes(&prev, &cur),
                &created_or_deleted_indexes(&cur, &prev),
            )?;

            let pks = |ddl: &PostgresDDL| {
                on_tables(
                    ddl.pks
                        .list()
                        .iter()
                        .map(|p| (p.schema.as_ref(), p.table.as_ref(), p.name.as_ref()))
                        .collect(),
                )
            };
            asker.ask(RenameKind::PrimaryKey, &pks(&prev), &pks(&cur))?;
            let fks = |ddl: &PostgresDDL| {
                on_tables(
                    ddl.fks
                        .list()
                        .iter()
                        .map(|f| (f.schema.as_ref(), f.table.as_ref(), f.name.as_ref()))
                        .collect(),
                )
            };
            asker.ask(RenameKind::ForeignKey, &fks(&prev), &fks(&cur))?;

            let views = |ddl: &PostgresDDL| -> Vec<Entry> {
                ddl.views
                    .list()
                    .iter()
                    .map(|v| Entry::new(Some(&v.schema), None, &v.name))
                    .collect()
            };
            asker.ask(RenameKind::View, &views(&prev), &views(&cur))?;
        }
        (Snapshot::MySQL(p), Snapshot::MySQL(c)) => {
            if !options.renames.schema_renames.is_empty() {
                return Err(MigrationError::ConfigError(
                    "MySQL migration scope is a selected database; database rename hints are not supported"
                        .to_string(),
                ));
            }
            let prev = MySQLDDL::try_from_entities(p.ddl.clone())
                .map_err(|error| MigrationError::ConfigError(error.to_string()))?;
            let cur = MySQLDDL::try_from_entities(c.ddl.clone())
                .map_err(|error| MigrationError::ConfigError(error.to_string()))?;
            let (prev, cur) = crate::mysql::diff::apply_rename_hints_for_questions(
                &prev,
                &cur,
                &mysql_diff_options(options)?,
            )
            .map_err(|error| MigrationError::ConfigError(error.to_string()))?;

            let tables = |ddl: &MySQLDDL| -> Vec<Entry> {
                ddl.tables
                    .list()
                    .iter()
                    .map(|t| Entry::new(None, None, &t.name))
                    .collect()
            };
            asker.ask(RenameKind::Table, &tables(&prev), &tables(&cur))?;

            let shared = shared(&tables(&prev), &tables(&cur));
            let columns = |ddl: &MySQLDDL| -> Vec<Entry> {
                ddl.columns
                    .list()
                    .iter()
                    .filter(|c| shared.iter().any(|t| t.name == c.table.as_ref()))
                    .map(|c| Entry::new(None, Some(&c.table), &c.name))
                    .collect()
            };
            asker.ask(RenameKind::Column, &columns(&prev), &columns(&cur))?;

            let views = |ddl: &MySQLDDL| -> Vec<Entry> {
                ddl.views
                    .list()
                    .iter()
                    .map(|v| Entry::new(None, None, &v.name))
                    .collect()
            };
            asker.ask(RenameKind::View, &views(&prev), &views(&cur))?;
        }
        _ => return Err(MigrationError::DialectMismatch),
    }
    Ok(asker.questions)
}

/// An entity's scope and name.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Entry {
    schema: Option<String>,
    table: Option<String>,
    name: String,
}

impl Entry {
    fn new(schema: Option<&str>, table: Option<&str>, name: &str) -> Self {
        Self {
            schema: schema.map(str::to_string),
            table: table.map(str::to_string),
            name: name.to_string(),
        }
    }

    fn same_scope(&self, other: &Self) -> bool {
        self.schema == other.schema && self.table == other.table
    }
}

/// Entries present on both sides.
fn shared(prev: &[Entry], cur: &[Entry]) -> Vec<Entry> {
    cur.iter()
        .filter(|entry| prev.contains(entry))
        .cloned()
        .collect()
}

struct Asker<'a> {
    hints: &'a RenameHints,
    questions: Vec<RenameQuestion>,
}

impl Asker<'_> {
    /// Asks about entries of `cur` missing from `prev`, offering entries of
    /// `prev` missing from `cur`.
    fn ask(
        &mut self,
        kind: RenameKind,
        prev: &[Entry],
        cur: &[Entry],
    ) -> Result<(), MigrationError> {
        let deleted: Vec<Entry> = prev.iter().filter(|e| !cur.contains(e)).cloned().collect();
        let created: Vec<Entry> = cur.iter().filter(|e| !prev.contains(e)).cloned().collect();
        self.ask_changed(kind, &deleted, &created)
    }

    /// Asks about each `created` entry that has `deleted` entries in its
    /// scope.
    fn ask_changed(
        &mut self,
        kind: RenameKind,
        deleted: &[Entry],
        created: &[Entry],
    ) -> Result<(), MigrationError> {
        for entry in created {
            let candidates: Vec<String> = deleted
                .iter()
                .filter(|d| d.same_scope(entry))
                .map(|d| d.name.clone())
                .collect();
            if candidates.is_empty() {
                continue;
            }
            if let Some(from) = unapplied_rename(self.hints, kind, entry) {
                return Err(MigrationError::ConfigError(format!(
                    "rename hint's `from` `{from}` doesn't match any deleted {kind} (renaming to `{}`)",
                    display_entry(entry)
                )));
            }
            if declared_created(self.hints, kind, entry) {
                continue;
            }
            self.questions.push(RenameQuestion {
                kind,
                schema: entry.schema.clone(),
                table: entry.table.clone(),
                name: entry.name.clone(),
                candidates,
            });
        }
        Ok(())
    }
}

fn display_entry(entry: &Entry) -> String {
    [
        entry.schema.as_deref(),
        entry.table.as_deref(),
        Some(entry.name.as_str()),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>()
    .join(".")
}

/// Whether a hint's optional schema names `entry`'s schema. SQLite and MySQL
/// entries have no schema, so any hint schema matches.
fn schema_matches(entry: &Entry, hint_schema: Option<&str>) -> bool {
    entry
        .schema
        .as_deref()
        .is_none_or(|schema| hint_schema.unwrap_or("public") == schema)
}

/// The `from` of a rename hint targeting `entry` that was not applied (so
/// `entry` still looks created).
fn unapplied_rename(hints: &RenameHints, kind: RenameKind, entry: &Entry) -> Option<String> {
    let table = entry.table.as_deref().unwrap_or_default();
    match kind {
        RenameKind::Schema => hints
            .schema_renames
            .iter()
            .find(|h| h.to == entry.name)
            .map(|h| h.from.clone()),
        RenameKind::Enum => hints
            .enum_renames
            .iter()
            .find(|h| h.to == entry.name && schema_matches(entry, h.schema.as_deref()))
            .map(|h| h.from.clone()),
        RenameKind::Table => hints
            .table_renames
            .iter()
            .find(|h| h.to == entry.name && schema_matches(entry, h.schema.as_deref()))
            .map(|h| h.from.clone()),
        RenameKind::Column => hints
            .column_renames
            .iter()
            .find(|h| {
                h.to == entry.name && h.table == table && schema_matches(entry, h.schema.as_deref())
            })
            .map(|h| h.from.clone()),
        RenameKind::Index => hints
            .index_renames
            .iter()
            .find(|h| {
                h.to == entry.name && h.table == table && schema_matches(entry, h.schema.as_deref())
            })
            .map(|h| h.from.clone()),
        RenameKind::View => hints
            .view_renames
            .iter()
            .find(|h| h.to == entry.name && schema_matches(entry, h.schema.as_deref()))
            .map(|h| h.from.clone()),
        kind => {
            let constraint = constraint_kind(kind)?;
            hints
                .constraint_renames
                .iter()
                .find(|h| {
                    h.kind == constraint
                        && h.to == entry.name
                        && h.table == table
                        && schema_matches(entry, h.schema.as_deref())
                })
                .map(|h| h.from.clone())
        }
    }
}

/// The [`ConstraintKind`] a constraint [`RenameKind`] renames.
const fn constraint_kind(kind: RenameKind) -> Option<ConstraintKind> {
    match kind {
        RenameKind::Unique => Some(ConstraintKind::Unique),
        RenameKind::Check => Some(ConstraintKind::Check),
        RenameKind::PrimaryKey => Some(ConstraintKind::PrimaryKey),
        RenameKind::ForeignKey => Some(ConstraintKind::ForeignKey),
        _ => None,
    }
}

fn declared_created(hints: &RenameHints, kind: RenameKind, entry: &Entry) -> bool {
    hints.creates.iter().any(|hint| {
        hint.kind == kind
            && hint.name == entry.name
            && hint.table == entry.table
            && (kind == RenameKind::Schema || schema_matches(entry, hint.schema.as_deref()))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diff_with;
    use crate::mysql::{MySQLEntity, MySQLSnapshot};
    use crate::postgres::PostgresSnapshot;
    use crate::postgres::ddl::{
        CheckConstraint as PgCheck, Column as PgColumn, Enum as PgEnum, ForeignKey as PgForeignKey,
        Index as PgIndex, IndexColumn as PgIndexColumn, PostgresEntity, PrimaryKey as PgPrimaryKey,
        Schema as PgSchema, Table as PgTable, UniqueConstraint as PgUnique, View as PgView,
    };
    use crate::sqlite::SQLiteSnapshot;
    use crate::sqlite::ddl::{Column, SqliteEntity, Table, View as SqliteView};
    use std::borrow::Cow;

    fn sqlite(tables: &[(&'static str, &[&'static str])]) -> Snapshot {
        let mut snapshot = SQLiteSnapshot::new();
        for (table, columns) in tables {
            snapshot.add_entity(SqliteEntity::Table(Table::new(*table)));
            for column in *columns {
                snapshot.add_entity(SqliteEntity::Column(
                    Column::new(*table, *column, "text").not_null(),
                ));
            }
        }
        Snapshot::Sqlite(snapshot)
    }

    fn question(
        kind: RenameKind,
        schema: Option<&str>,
        table: Option<&str>,
        name: &str,
        candidates: &[&str],
    ) -> RenameQuestion {
        RenameQuestion {
            kind,
            schema: schema.map(str::to_string),
            table: table.map(str::to_string),
            name: name.to_string(),
            candidates: candidates.iter().map(|c| (*c).to_string()).collect(),
        }
    }

    /// Answers every question by renaming from its first candidate.
    fn rename_all(prev: &Snapshot, cur: &Snapshot, options: &mut DiffOptions) {
        while let Some(q) = rename_questions(prev, cur, options)
            .unwrap()
            .first()
            .cloned()
        {
            let from = q.candidates[0].clone();
            q.answer(&mut options.renames, &RenameAnswer::RenameFrom(from))
                .unwrap();
        }
    }

    #[test]
    fn kinds_round_trip_drizzle_kit_names() {
        for kind in RenameKind::ALL {
            assert_eq!(RenameKind::from_hint_kind(kind.as_str()), Some(kind));
        }
        assert_eq!(
            RenameKind::from_hint_kind("foreign key"),
            Some(RenameKind::ForeignKey)
        );
        assert_eq!(RenameKind::from_hint_kind("sequence"), None);
    }

    #[test]
    fn sqlite_asks_tables_before_columns() {
        let prev = sqlite(&[("users", &["id", "name"]), ("logs", &["id", "body"])]);
        let cur = sqlite(&[("accounts", &["id", "name"]), ("logs", &["id", "text"])]);

        let questions = rename_questions(&prev, &cur, &DiffOptions::new()).unwrap();
        assert_eq!(
            questions,
            [
                question(RenameKind::Table, None, None, "accounts", &["users"]),
                question(RenameKind::Column, None, Some("logs"), "text", &["body"]),
            ]
        );
    }

    #[test]
    fn sqlite_column_questions_follow_a_table_rename() {
        let prev = sqlite(&[("users", &["id", "name"])]);
        let cur = sqlite(&[("accounts", &["id", "full_name"])]);
        let mut options = DiffOptions::new();

        let questions = rename_questions(&prev, &cur, &options).unwrap();
        assert_eq!(
            questions,
            [question(
                RenameKind::Table,
                None,
                None,
                "accounts",
                &["users"]
            )]
        );
        questions[0]
            .answer(
                &mut options.renames,
                &RenameAnswer::RenameFrom("users".into()),
            )
            .unwrap();

        let questions = rename_questions(&prev, &cur, &options).unwrap();
        assert_eq!(
            questions,
            [question(
                RenameKind::Column,
                None,
                Some("accounts"),
                "full_name",
                &["name"]
            )]
        );
        questions[0]
            .answer(&mut options.renames, &RenameAnswer::Create)
            .unwrap();
        assert!(rename_questions(&prev, &cur, &options).unwrap().is_empty());

        let plan = diff_with(&prev, &cur, &options).unwrap();
        assert_eq!(
            plan.statements[0],
            "ALTER TABLE `users` RENAME TO `accounts`;"
        );
        assert!(!plan.statements.iter().any(|s| s.contains("RENAME COLUMN")));
        assert!(plan.statements.iter().any(|s| s.contains("full_name")));
    }

    #[test]
    fn a_rename_consumes_its_candidate_and_create_is_not_asked_again() {
        let prev = sqlite(&[("a", &["id"]), ("b", &["id"])]);
        let cur = sqlite(&[("c", &["id"]), ("d", &["id"])]);
        let mut options = DiffOptions::new();

        let questions = rename_questions(&prev, &cur, &options).unwrap();
        assert_eq!(
            questions,
            [
                question(RenameKind::Table, None, None, "c", &["a", "b"]),
                question(RenameKind::Table, None, None, "d", &["a", "b"]),
            ]
        );

        questions[0]
            .answer(&mut options.renames, &RenameAnswer::RenameFrom("a".into()))
            .unwrap();
        let questions = rename_questions(&prev, &cur, &options).unwrap();
        assert_eq!(
            questions,
            [question(RenameKind::Table, None, None, "d", &["b"])]
        );

        questions[0]
            .answer(&mut options.renames, &RenameAnswer::Create)
            .unwrap();
        assert!(rename_questions(&prev, &cur, &options).unwrap().is_empty());
        assert_eq!(
            options.renames.creates,
            [CreateHint::new(RenameKind::Table, "d")]
        );
    }

    #[test]
    fn answering_with_a_non_candidate_fails() {
        let q = question(RenameKind::Table, None, None, "c", &["a"]);
        let mut hints = RenameHints::new();
        assert!(matches!(
            q.answer(&mut hints, &RenameAnswer::RenameFrom("zzz".into())),
            Err(MigrationError::ConfigError(_))
        ));
        assert_eq!(hints, RenameHints::new());
    }

    #[test]
    fn rename_hint_with_unknown_source_is_an_error() {
        let prev = sqlite(&[("users", &["id"])]);
        let cur = sqlite(&[("accounts", &["id"])]);
        let options = DiffOptions::new().rename_table("missing", "accounts");
        let error = rename_questions(&prev, &cur, &options).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("doesn't match any deleted table"),
            "{error}"
        );
    }

    #[test]
    fn plans_list_the_tables_and_columns_they_drop() {
        use crate::DataLoss;

        let prev = sqlite(&[("users", &["id", "name", "nickname"]), ("logs", &["id"])]);
        let cur = sqlite(&[("accounts", &["id", "full_name"])]);
        let options = DiffOptions::new()
            .rename_table("users", "accounts")
            .rename_column("accounts", "name", "full_name");
        let plan = diff_with(&prev, &cur, &options).unwrap();
        assert_eq!(
            plan.data_loss,
            [
                DataLoss {
                    schema: None,
                    table: "logs".into(),
                    column: None,
                },
                DataLoss {
                    schema: None,
                    table: "accounts".into(),
                    column: Some("nickname".into()),
                },
            ]
        );
        assert_eq!(plan.data_loss[0].to_string(), "table `logs`");
        assert_eq!(
            plan.data_loss[1].count_sql(drizzle_types::Dialect::SQLite),
            r#"SELECT COUNT(*) FROM "accounts" WHERE "nickname" IS NOT NULL"#
        );

        // Renamed, not dropped: nothing is lost.
        let renamed = diff_with(
            &sqlite(&[("users", &["id"])]),
            &sqlite(&[("accounts", &["id"])]),
            &DiffOptions::new().rename_table("users", "accounts"),
        )
        .unwrap();
        assert!(renamed.data_loss.is_empty(), "{:?}", renamed.data_loss);
    }

    #[test]
    fn diff_never_guesses_a_rename() {
        let prev = sqlite(&[("users", &["id", "name"])]);
        let cur = sqlite(&[("users", &["id", "full_name"])]);

        let error = diff_with(&prev, &cur, &DiffOptions::new()).unwrap_err();
        let MigrationError::UnansweredRenames(questions) = &error else {
            panic!("{error}");
        };
        assert_eq!(
            questions,
            &[question(
                RenameKind::Column,
                None,
                Some("users"),
                "full_name",
                &["name"]
            )]
        );
        let message = error.to_string();
        assert!(
            message.contains("column `users.full_name`: created, or renamed from `name`?"),
            "{message}"
        );
        assert!(
            message
                .contains(r#"renamed from `name`: .rename_column("users", "name", "full_name")"#),
            "{message}"
        );
        assert!(
            message.contains(
                r#"created: .create(CreateHint::new(RenameKind::Column, "full_name").on_table("users"))"#
            ),
            "{message}"
        );

        let renamed = diff_with(
            &prev,
            &cur,
            &DiffOptions::new().rename_column("users", "name", "full_name"),
        )
        .unwrap();
        assert_eq!(
            renamed.statements,
            ["ALTER TABLE `users` RENAME COLUMN `name` TO `full_name`;"]
        );

        let created = diff_with(
            &prev,
            &cur,
            &DiffOptions::new()
                .create(CreateHint::new(RenameKind::Column, "full_name").on_table("users")),
        )
        .unwrap();
        assert!(
            !created.statements.iter().any(|s| s.contains("RENAME")),
            "{:?}",
            created.statements
        );

        let prev = sqlite(&[("users", &["id"])]);
        let cur = sqlite(&[("accounts", &["id"])]);
        let error = diff_with(&prev, &cur, &DiffOptions::new()).unwrap_err();
        assert!(
            error
                .to_string()
                .contains(r#"renamed from `users`: .rename_table("users", "accounts")"#),
            "{error}"
        );
        let tables = diff_with(
            &prev,
            &cur,
            &DiffOptions::new().create(CreateHint::new(RenameKind::Table, "accounts")),
        )
        .unwrap();
        assert!(
            tables
                .statements
                .iter()
                .any(|s| s.starts_with("CREATE TABLE `accounts`")),
            "{:?}",
            tables.statements
        );
        assert!(
            tables
                .statements
                .iter()
                .any(|s| s.starts_with("DROP TABLE `users`")),
            "{:?}",
            tables.statements
        );
    }

    #[test]
    fn sqlite_never_asks_about_views() {
        let view = |name: &'static str| {
            let mut view = SqliteView::new(name);
            view.definition = Some(Cow::Borrowed("SELECT 1"));
            SqliteEntity::View(view)
        };
        let mut prev = SQLiteSnapshot::new();
        prev.add_entity(view("old_view"));
        let mut cur = SQLiteSnapshot::new();
        cur.add_entity(view("new_view"));
        let questions = rename_questions(
            &Snapshot::Sqlite(prev),
            &Snapshot::Sqlite(cur),
            &DiffOptions::new(),
        )
        .unwrap();
        assert!(questions.is_empty());
    }

    /// Names for every PostgreSQL kind asked about, all chosen explicitly.
    struct PgNames {
        schema: &'static str,
        enum_: &'static str,
        unique: &'static str,
        check: &'static str,
        index: &'static str,
        pk: &'static str,
        fk: &'static str,
        view: &'static str,
    }

    fn pg(names: &PgNames) -> Snapshot {
        let mut snapshot = PostgresSnapshot::new();
        snapshot.add_entity(PostgresEntity::Schema(PgSchema::new("public")));
        snapshot.add_entity(PostgresEntity::Schema(PgSchema::new(names.schema)));
        snapshot.add_entity(PostgresEntity::Enum(PgEnum::new(
            "public",
            names.enum_,
            vec![Cow::Borrowed("a"), Cow::Borrowed("b")],
        )));
        for (table, columns) in [
            ("users", &["id", "email"][..]),
            ("posts", &["id", "user_id"][..]),
        ] {
            snapshot.add_entity(PostgresEntity::Table(PgTable::new("public", table)));
            for column in columns {
                snapshot.add_entity(PostgresEntity::Column(
                    PgColumn::new("public", table, *column, "integer").not_null(),
                ));
            }
        }
        let mut unique = PgUnique::from_strings(
            "public".into(),
            "users".into(),
            names.unique.into(),
            vec!["email".into()],
        );
        unique.name_explicit = true;
        snapshot.add_entity(PostgresEntity::UniqueConstraint(unique));
        snapshot.add_entity(PostgresEntity::CheckConstraint(PgCheck::new(
            "public",
            "users",
            names.check,
            "email > 0",
        )));
        let mut index = PgIndex::new(
            "public",
            "users",
            names.index,
            vec![PgIndexColumn::new("email")],
        );
        index.name_explicit = true;
        snapshot.add_entity(PostgresEntity::Index(index));
        let mut pk = PgPrimaryKey::from_strings(
            "public".into(),
            "users".into(),
            names.pk.into(),
            vec!["id".into()],
        );
        pk.name_explicit = true;
        snapshot.add_entity(PostgresEntity::PrimaryKey(pk));
        let mut fk = PgForeignKey::from_strings(
            "public".into(),
            "posts".into(),
            names.fk.into(),
            vec!["user_id".into()],
            "public".into(),
            "users".into(),
            vec!["id".into()],
        );
        fk.name_explicit = true;
        snapshot.add_entity(PostgresEntity::ForeignKey(fk));
        // A table in the renamed schema: asked about only through the schema.
        snapshot.add_entity(PostgresEntity::Table(PgTable::new(names.schema, "items")));
        snapshot.add_entity(PostgresEntity::Column(
            PgColumn::new(names.schema, "items", "id", "integer").not_null(),
        ));
        let mut v = PgView::new("public", names.view);
        v.definition = Some(Cow::Borrowed("SELECT 1"));
        snapshot.add_entity(PostgresEntity::View(v));
        Snapshot::Postgres(snapshot)
    }

    #[test]
    fn postgres_asks_every_kind_in_drizzle_kit_order() {
        let prev = pg(&PgNames {
            schema: "old_s",
            enum_: "mood",
            unique: "users_email_uq",
            check: "email_positive",
            index: "users_email_idx",
            pk: "users_id_pk",
            fk: "posts_user_fk",
            view: "v_old",
        });
        let cur = pg(&PgNames {
            schema: "new_s",
            enum_: "feeling",
            unique: "users_email_key",
            check: "email_is_positive",
            index: "users_mail_idx",
            pk: "users_pk",
            fk: "posts_user_id_fk",
            view: "v_new",
        });
        let mut options = DiffOptions::new();

        let questions = rename_questions(&prev, &cur, &options).unwrap();
        let users =
            |kind, name, from: &str| question(kind, Some("public"), Some("users"), name, &[from]);
        assert_eq!(
            questions,
            [
                question(RenameKind::Schema, None, None, "new_s", &["old_s"]),
                question(RenameKind::Enum, Some("public"), None, "feeling", &["mood"]),
                users(RenameKind::Unique, "users_email_key", "users_email_uq"),
                users(RenameKind::Check, "email_is_positive", "email_positive"),
                users(RenameKind::Index, "users_mail_idx", "users_email_idx"),
                users(RenameKind::PrimaryKey, "users_pk", "users_id_pk"),
                question(
                    RenameKind::ForeignKey,
                    Some("public"),
                    Some("posts"),
                    "posts_user_id_fk",
                    &["posts_user_fk"]
                ),
                question(RenameKind::View, Some("public"), None, "v_new", &["v_old"]),
            ],
            "new_s.items has no deleted table in its own schema"
        );

        rename_all(&prev, &cur, &mut options);
        let plan = diff_with(&prev, &cur, &options).unwrap();
        assert_eq!(
            plan.statements,
            [
                "ALTER SCHEMA \"old_s\" RENAME TO \"new_s\";",
                "ALTER TYPE \"mood\" RENAME TO \"feeling\";",
                "ALTER TABLE \"users\" RENAME CONSTRAINT \"users_email_uq\" TO \"users_email_key\";",
                "ALTER TABLE \"users\" RENAME CONSTRAINT \"email_positive\" TO \"email_is_positive\";",
                "ALTER TABLE \"users\" RENAME CONSTRAINT \"users_id_pk\" TO \"users_pk\";",
                "ALTER TABLE \"posts\" RENAME CONSTRAINT \"posts_user_fk\" TO \"posts_user_id_fk\";",
                "ALTER INDEX \"users_email_idx\" RENAME TO \"users_mail_idx\";",
                "ALTER VIEW \"v_old\" RENAME TO \"v_new\";",
            ]
        );
    }

    /// drizzle-kit keeps implicitly named constraints across a table rename
    /// (`preserveEntityNames`), so renaming the table raises no constraint
    /// or index questions and plans only the table rename.
    #[test]
    fn postgres_table_rename_asks_nothing_about_implicitly_named_constraints() {
        let snapshot = |table: &'static str| {
            let mut snapshot = PostgresSnapshot::new();
            snapshot.add_entity(PostgresEntity::Schema(PgSchema::new("public")));
            snapshot.add_entity(PostgresEntity::Table(PgTable::new("public", table)));
            for column in ["id", "email", "parent_id"] {
                snapshot.add_entity(PostgresEntity::Column(
                    PgColumn::new("public", table, column, "integer").not_null(),
                ));
            }
            let mut pk = PgPrimaryKey::from_strings(
                "public".into(),
                table.into(),
                format!("{table}_pkey"),
                vec!["id".into()],
            );
            pk.name_explicit = false;
            snapshot.add_entity(PostgresEntity::PrimaryKey(pk));
            let unique = PgUnique::from_strings(
                "public".into(),
                table.into(),
                format!("{table}_email_key"),
                vec!["email".into()],
            );
            snapshot.add_entity(PostgresEntity::UniqueConstraint(unique));
            snapshot.add_entity(PostgresEntity::Index(PgIndex::new(
                "public",
                table,
                format!("{table}_email_idx"),
                vec![PgIndexColumn::new("email")],
            )));
            snapshot.add_entity(PostgresEntity::CheckConstraint(PgCheck::new(
                "public",
                table,
                format!("{table}_email_check"),
                "email > 0",
            )));
            let fk = PgForeignKey::from_strings(
                "public".into(),
                table.into(),
                format!("{table}_parent_id_{table}_id_fk"),
                vec!["parent_id".into()],
                "public".into(),
                table.into(),
                vec!["id".into()],
            );
            snapshot.add_entity(PostgresEntity::ForeignKey(fk));
            Snapshot::Postgres(snapshot)
        };
        let (prev, cur) = (snapshot("users"), snapshot("accounts"));
        let mut options = DiffOptions::new();
        let questions = rename_questions(&prev, &cur, &options).unwrap();
        assert_eq!(
            questions,
            [question(
                RenameKind::Table,
                Some("public"),
                None,
                "accounts",
                &["users"]
            )]
        );

        rename_all(&prev, &cur, &mut options);
        assert!(rename_questions(&prev, &cur, &options).unwrap().is_empty());
        let plan = diff_with(&prev, &cur, &options).unwrap();
        assert_eq!(
            plan.statements,
            ["ALTER TABLE \"users\" RENAME TO \"accounts\";"]
        );
    }

    #[test]
    fn postgres_scopes_tables_by_schema_and_columns_by_table() {
        let snapshot = |tables: &[(&'static str, &'static str, &[&'static str])]| {
            let mut snapshot = PostgresSnapshot::new();
            snapshot.add_entity(PostgresEntity::Schema(PgSchema::new("public")));
            snapshot.add_entity(PostgresEntity::Schema(PgSchema::new("app")));
            for (schema, table, columns) in tables {
                snapshot.add_entity(PostgresEntity::Table(PgTable::new(*schema, *table)));
                for column in *columns {
                    snapshot.add_entity(PostgresEntity::Column(
                        PgColumn::new(*schema, *table, *column, "text").not_null(),
                    ));
                }
            }
            Snapshot::Postgres(snapshot)
        };
        let prev = snapshot(&[
            ("public", "users", &["id", "name"]),
            ("app", "old_jobs", &["id"]),
            ("public", "posts", &["id", "body"]),
        ]);
        let cur = snapshot(&[
            ("public", "users", &["id", "full_name"]),
            ("app", "jobs", &["id"]),
            ("public", "posts", &["id", "content"]),
            ("public", "events", &["id"]),
        ]);

        let mut options = DiffOptions::new();
        assert_eq!(
            rename_questions(&prev, &cur, &options).unwrap(),
            [
                question(RenameKind::Table, Some("app"), None, "jobs", &["old_jobs"]),
                question(
                    RenameKind::Column,
                    Some("public"),
                    Some("users"),
                    "full_name",
                    &["name"]
                ),
                question(
                    RenameKind::Column,
                    Some("public"),
                    Some("posts"),
                    "content",
                    &["body"]
                ),
            ],
            "public.events has no deleted table in its schema"
        );

        // Hints given up front are applied before asking.
        options = options
            .rename_table_in("app", "old_jobs", "jobs")
            .rename_column("users", "name", "full_name")
            .create(CreateHint::new(RenameKind::Column, "content").on_table("posts"));
        assert!(rename_questions(&prev, &cur, &options).unwrap().is_empty());

        let plan = diff_with(&prev, &cur, &options).unwrap();
        assert!(
            plan.statements
                .contains(&"ALTER TABLE \"app\".\"old_jobs\" RENAME TO \"jobs\";".to_string()),
            "{:?}",
            plan.statements
        );
        assert!(
            plan.statements.contains(
                &"ALTER TABLE \"users\" RENAME COLUMN \"name\" TO \"full_name\";".to_string()
            ),
            "{:?}",
            plan.statements
        );
        assert!(
            !plan
                .statements
                .iter()
                .any(|s| s.contains("RENAME COLUMN \"body\""))
        );
    }

    #[test]
    fn postgres_diff_never_guesses_a_rename() {
        let table = |schema: &'static str, name: &'static str| {
            let mut snapshot = PostgresSnapshot::new();
            snapshot.add_entity(PostgresEntity::Schema(PgSchema::new(schema)));
            snapshot.add_entity(PostgresEntity::Table(PgTable::new(schema, name)));
            snapshot.add_entity(PostgresEntity::Column(
                PgColumn::new(schema, name, "id", "integer").not_null(),
            ));
            Snapshot::Postgres(snapshot)
        };
        let error = diff_with(
            &table("public", "users"),
            &table("public", "accounts"),
            &DiffOptions::new(),
        )
        .unwrap_err();
        assert!(
            error
                .to_string()
                .contains(r#"renamed from `users`: .rename_table("users", "accounts")"#),
            "{error}"
        );

        let error = diff_with(
            &table("app", "users"),
            &table("app", "accounts"),
            &DiffOptions::new(),
        )
        .unwrap_err();
        let message = error.to_string();
        assert!(
            message.contains("table `app.accounts`: created, or renamed from `users`?"),
            "{message}"
        );
        assert!(
            message.contains(r#".rename_table_in("app", "users", "accounts")"#),
            "{message}"
        );
        assert!(
            message.contains(
                r#".create(CreateHint::new(RenameKind::Table, "accounts").in_schema("app"))"#
            ),
            "{message}"
        );

        let renamed = diff_with(
            &table("public", "users"),
            &table("public", "accounts"),
            &DiffOptions::new().rename_table("users", "accounts"),
        )
        .unwrap();
        assert_eq!(
            renamed.statements,
            ["ALTER TABLE \"users\" RENAME TO \"accounts\";"]
        );
        let created = diff_with(
            &table("public", "users"),
            &table("public", "accounts"),
            &DiffOptions::new().create(CreateHint::new(RenameKind::Table, "accounts")),
        )
        .unwrap();
        assert!(!created.statements.iter().any(|s| s.contains("RENAME")));
        assert!(
            created
                .statements
                .iter()
                .any(|s| s.starts_with("CREATE TABLE"))
        );
        assert!(
            created
                .statements
                .iter()
                .any(|s| s.starts_with("DROP TABLE"))
        );
    }

    #[test]
    fn mysql_asks_tables_columns_and_views() {
        let snapshot = |table: &'static str, column: &'static str, view: &'static str| {
            let mut snapshot = MySQLSnapshot::new();
            snapshot.add_entity(MySQLEntity::Table(crate::mysql::Table::new(table)));
            snapshot.add_entity(MySQLEntity::Column(crate::mysql::Column::new(
                table, "id", "int",
            )));
            snapshot.add_entity(MySQLEntity::Table(crate::mysql::Table::new("logs")));
            snapshot.add_entity(MySQLEntity::Column(crate::mysql::Column::new(
                "logs", column, "text",
            )));
            snapshot.add_entity(MySQLEntity::View(crate::mysql::View::new(view, "SELECT 1")));
            Snapshot::MySQL(snapshot)
        };
        let prev = snapshot("users", "body", "v_old");
        let cur = snapshot("accounts", "text", "v_new");
        let mut options = DiffOptions::new();

        assert_eq!(
            rename_questions(&prev, &cur, &options).unwrap(),
            [
                question(RenameKind::Table, None, None, "accounts", &["users"]),
                question(RenameKind::Column, None, Some("logs"), "text", &["body"]),
                question(RenameKind::View, None, None, "v_new", &["v_old"]),
            ]
        );

        rename_all(&prev, &cur, &mut options);
        let plan = diff_with(&prev, &cur, &options).unwrap();
        for expected in [
            "RENAME TABLE `users` TO `accounts`;",
            "RENAME TABLE `v_old` TO `v_new`;",
        ] {
            assert!(
                plan.statements.iter().any(|s| s == expected),
                "missing {expected}: {:?}",
                plan.statements
            );
        }
        assert!(
            plan.statements
                .iter()
                .any(|s| s.contains("RENAME COLUMN `body` TO `text`")),
            "{:?}",
            plan.statements
        );
    }
}

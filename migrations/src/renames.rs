//! Rename-or-create questions, asked the way `drizzle-kit` asks them.
//!
//! When an entity disappears from the previous snapshot and another of the
//! same kind appears in the current one, the diff alone cannot tell a rename
//! from a drop plus a create. [`diff_with`](crate::diff_with) guesses by
//! default ([`DiffOptions::infer_renames`]); [`rename_questions`] instead
//! lists the decisions so a caller (the `drizzle` CLI) can ask a person, then
//! diff with the answers and `infer_renames(false)`.
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
//! let mut options = DiffOptions::new().infer_renames(false);
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
    DiffOptions, RenameHints, apply_postgres_rename_hints, apply_sqlite_rename_hints,
    mysql_diff_options,
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
    /// PostgreSQL index.
    Index,
    /// View (PostgreSQL and MySQL; SQLite cannot rename a view, so it is
    /// never asked about there).
    View,
}

impl RenameKind {
    /// Every kind, in the order questions are asked.
    pub const ALL: [Self; 6] = [
        Self::Schema,
        Self::Enum,
        Self::Table,
        Self::Column,
        Self::Index,
        Self::View,
    ];

    /// drizzle-kit's name for the kind, as used in hint `kind` fields:
    /// `schema`, `enum`, `table`, `column`, `index`, `view`.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Schema => "schema",
            Self::Enum => "enum",
            Self::Table => "table",
            Self::Column => "column",
            Self::Index => "index",
            Self::View => "view",
        }
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

impl RenameQuestion {
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
        };
        Ok(())
    }
}

/// Lists the rename-or-create questions between `prev` and `current`, given
/// the hints already in `options`.
///
/// One question per created entity that has deleted candidates in scope,
/// grouped by kind in drizzle-kit's order (schemas, enums, tables, columns,
/// indexes, views). The rename hints in `options` are applied first, so
/// renamed entities are neither asked about nor offered as candidates, and
/// columns are compared under their tables' new names. Entities in
/// [`RenameHints::creates`] are not asked about.
///
/// Questions after the first assume every earlier unanswered question is
/// answered "create" (drizzle-kit's non-interactive behavior). To ask
/// interactively, answer the first question
/// ([`RenameQuestion::answer`]) and call this again; the list is empty once
/// everything is decided. Then diff with the same options and
/// [`infer_renames(false)`](DiffOptions::infer_renames).
///
/// Kinds per dialect, as drizzle-kit asks them: SQLite tables and columns;
/// MySQL tables, columns, and views; PostgreSQL schemas, enums, tables,
/// columns, indexes, and views (`public` is never asked about). A
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
        Column as PgColumn, Enum as PgEnum, Index as PgIndex, IndexColumn as PgIndexColumn,
        PostgresEntity, Schema as PgSchema, Table as PgTable, View as PgView,
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

    fn no_inference() -> DiffOptions {
        DiffOptions::new().infer_renames(false)
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
        assert_eq!(RenameKind::from_hint_kind("foreign key"), None);
    }

    #[test]
    fn sqlite_asks_tables_before_columns() {
        let prev = sqlite(&[("users", &["id", "name"]), ("logs", &["id", "body"])]);
        let cur = sqlite(&[("accounts", &["id", "name"]), ("logs", &["id", "text"])]);

        let questions = rename_questions(&prev, &cur, &no_inference()).unwrap();
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
        let mut options = no_inference();

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
        let mut options = no_inference();

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
        let options = no_inference().rename_table("missing", "accounts");
        let error = rename_questions(&prev, &cur, &options).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("doesn't match any deleted table"),
            "{error}"
        );
    }

    #[test]
    fn infer_renames_false_keeps_drop_and_create() {
        let prev = sqlite(&[("users", &["id", "name"])]);
        let cur = sqlite(&[("users", &["id", "full_name"])]);

        let inferred = diff_with(&prev, &cur, &DiffOptions::new()).unwrap();
        assert_eq!(
            inferred.statements,
            ["ALTER TABLE `users` RENAME COLUMN `name` TO `full_name`;"]
        );

        let explicit = diff_with(&prev, &cur, &no_inference()).unwrap();
        assert!(
            !explicit.statements.iter().any(|s| s.contains("RENAME")),
            "{:?}",
            explicit.statements
        );

        let hinted = diff_with(
            &prev,
            &cur,
            &no_inference().rename_column("users", "name", "full_name"),
        )
        .unwrap();
        assert_eq!(hinted.statements, inferred.statements);

        let prev = sqlite(&[("users", &["id"])]);
        let cur = sqlite(&[("accounts", &["id"])]);
        let tables = diff_with(&prev, &cur, &no_inference()).unwrap();
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
            &no_inference(),
        )
        .unwrap();
        assert!(questions.is_empty());
    }

    fn pg(
        schema: &'static str,
        enum_: &'static str,
        index: &'static str,
        view: &'static str,
    ) -> Snapshot {
        let mut snapshot = PostgresSnapshot::new();
        snapshot.add_entity(PostgresEntity::Schema(PgSchema::new("public")));
        snapshot.add_entity(PostgresEntity::Schema(PgSchema::new(schema)));
        snapshot.add_entity(PostgresEntity::Enum(PgEnum::new(
            "public",
            enum_,
            vec![Cow::Borrowed("a"), Cow::Borrowed("b")],
        )));
        snapshot.add_entity(PostgresEntity::Table(PgTable::new("public", "users")));
        snapshot.add_entity(PostgresEntity::Column(
            PgColumn::new("public", "users", "id", "integer").not_null(),
        ));
        snapshot.add_entity(PostgresEntity::Column(
            PgColumn::new("public", "users", "email", "text").not_null(),
        ));
        snapshot.add_entity(PostgresEntity::Index(PgIndex::new(
            "public",
            "users",
            index,
            vec![PgIndexColumn::new("email")],
        )));
        // A table in the renamed schema: asked about only through the schema.
        snapshot.add_entity(PostgresEntity::Table(PgTable::new(schema, "items")));
        snapshot.add_entity(PostgresEntity::Column(
            PgColumn::new(schema, "items", "id", "integer").not_null(),
        ));
        let mut v = PgView::new("public", view);
        v.definition = Some(Cow::Borrowed("SELECT 1"));
        snapshot.add_entity(PostgresEntity::View(v));
        Snapshot::Postgres(snapshot)
    }

    #[test]
    fn postgres_asks_every_kind_in_drizzle_kit_order() {
        let prev = pg("old_s", "mood", "users_email_idx", "v_old");
        let cur = pg("new_s", "feeling", "users_mail_idx", "v_new");
        let mut options = no_inference();

        let questions = rename_questions(&prev, &cur, &options).unwrap();
        assert_eq!(
            questions,
            [
                question(RenameKind::Schema, None, None, "new_s", &["old_s"]),
                question(RenameKind::Enum, Some("public"), None, "feeling", &["mood"]),
                question(
                    RenameKind::Index,
                    Some("public"),
                    Some("users"),
                    "users_mail_idx",
                    &["users_email_idx"]
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
                "ALTER TYPE \"public\".\"mood\" RENAME TO \"feeling\";",
                "ALTER INDEX \"public\".\"users_email_idx\" RENAME TO \"users_mail_idx\";",
                "ALTER VIEW \"public\".\"v_old\" RENAME TO \"v_new\";",
            ]
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

        let mut options = no_inference();
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
                &"ALTER TABLE \"public\".\"users\" RENAME COLUMN \"name\" TO \"full_name\";"
                    .to_string()
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
    fn postgres_infer_renames_false_keeps_drop_and_create() {
        let table = |name: &'static str| {
            let mut snapshot = PostgresSnapshot::new();
            snapshot.add_entity(PostgresEntity::Schema(PgSchema::new("public")));
            snapshot.add_entity(PostgresEntity::Table(PgTable::new("public", name)));
            snapshot.add_entity(PostgresEntity::Column(
                PgColumn::new("public", name, "id", "integer").not_null(),
            ));
            Snapshot::Postgres(snapshot)
        };
        let inferred = diff_with(&table("users"), &table("accounts"), &DiffOptions::new()).unwrap();
        assert_eq!(
            inferred.statements,
            ["ALTER TABLE \"users\" RENAME TO \"accounts\";"]
        );
        let explicit = diff_with(&table("users"), &table("accounts"), &no_inference()).unwrap();
        assert!(!explicit.statements.iter().any(|s| s.contains("RENAME")));
        assert!(
            explicit
                .statements
                .iter()
                .any(|s| s.starts_with("CREATE TABLE"))
        );
        assert!(
            explicit
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
        let mut options = no_inference();

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

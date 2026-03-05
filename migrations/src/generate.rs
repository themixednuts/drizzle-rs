//! Programmatic migration generation API.
//!
//! Diff two schema snapshots and get SQL statements — no file I/O, no CLI needed.
//!
//! # Snapshot-to-snapshot example
//!
//! ```rust
//! use drizzle_migrations::{Snapshot, diff};
//!
//! let prev = Snapshot::empty(drizzle_types::Dialect::SQLite);
//! let current = Snapshot::empty(drizzle_types::Dialect::SQLite);
//! let migration = diff(&prev, &current).unwrap();
//! assert!(migration.is_empty());
//! ```
//!
//! # Schema-to-schema example (recommended for runtime generation)
//!
//! ```rust,no_run
//! use drizzle_migrations::{Options, diff_schemas_with};
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
//!     Options::new()
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

use crate::postgres::collection::PostgresDDL;
use crate::schema::{Schema, Snapshot};
use crate::sqlite::collection::SQLiteDDL;
use crate::writer::MigrationError;
use std::io::{self, Write};

/// Generated migration payload.
#[derive(Clone, Debug)]
pub struct Plan {
    /// SQL statements for the migration.
    pub statements: Vec<String>,
    /// Schema snapshot after this migration is applied.
    pub snapshot: Snapshot,
}

impl Plan {
    /// Returns true when there are no executable SQL statements.
    pub fn is_empty(&self) -> bool {
        self.statements.is_empty()
            || self
                .statements
                .iter()
                .all(|statement| statement.trim().is_empty())
    }

    /// Format statements with `--> statement-breakpoint` markers.
    pub fn to_sql(&self) -> String {
        self.statements.join("\n--> statement-breakpoint\n")
    }

    /// Write formatted migration SQL to a writer.
    pub fn write(&self, writer: impl Write) -> io::Result<()> {
        let mut writer = writer;
        writer.write_all(self.to_sql().as_bytes())
    }
}

/// Explicit rename hints used during migration generation.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RenameHints {
    /// Table rename hints.
    pub table_renames: Vec<TableRenameHint>,
    /// Column rename hints.
    pub column_renames: Vec<ColumnRenameHint>,
}

impl RenameHints {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn rename_table(mut self, from: impl Into<String>, to: impl Into<String>) -> Self {
        self.table_renames.push(TableRenameHint {
            schema: None,
            from: from.into(),
            to: to.into(),
        });
        self
    }

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
}

/// Table rename hint.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TableRenameHint {
    /// Optional schema (PostgreSQL only). If omitted, defaults to `public`.
    pub schema: Option<String>,
    /// Current table name.
    pub from: String,
    /// New table name.
    pub to: String,
}

/// Column rename hint.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ColumnRenameHint {
    /// Optional schema (PostgreSQL only). If omitted, defaults to `public`.
    pub schema: Option<String>,
    /// Table containing the column.
    pub table: String,
    /// Current column name.
    pub from: String,
    /// New column name.
    pub to: String,
}

/// Generation options for [`diff_with`] and [`diff_schemas_with`].
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Options {
    /// Explicit rename hints applied before heuristic diffing.
    pub renames: RenameHints,
    /// If true, every hint must apply; otherwise generation fails.
    pub strict_renames: bool,
}

impl Options {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn with_renames(mut self, renames: RenameHints) -> Self {
        self.renames = renames;
        self
    }

    #[must_use]
    pub fn strict_renames(mut self, strict: bool) -> Self {
        self.strict_renames = strict;
        self
    }

    #[must_use]
    pub fn rename_table(mut self, from: impl Into<String>, to: impl Into<String>) -> Self {
        self.renames = self.renames.rename_table(from, to);
        self
    }

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
}

/// Diff two snapshots and return the migration SQL statements.
///
/// Both snapshots must be for the same dialect (e.g., both SQLite or both PostgreSQL).
/// Returns `Ok(vec![])` if no changes are detected.
///
/// This is a pure function — no file I/O, no side effects.
///
/// For writing tagged migration directories (`./drizzle/<tag>/...`), prefer
/// [`crate::build::run`].
pub fn diff(prev: &Snapshot, current: &Snapshot) -> Result<Plan, MigrationError> {
    diff_with(prev, current, Options::default())
}

/// Diff two snapshots with explicit generation options.
///
/// Use this when you need rename hints (table/column renames) to avoid
/// drop-and-recreate diffs.
pub fn diff_with(
    prev: &Snapshot,
    current: &Snapshot,
    options: Options,
) -> Result<Plan, MigrationError> {
    let statements = match (prev, current) {
        (Snapshot::Sqlite(p), Snapshot::Sqlite(c)) => {
            let mut prev_ddl = SQLiteDDL::from_entities(p.ddl.clone());
            let cur_ddl = crate::sqlite::collection::SQLiteDDL::from_entities(c.ddl.clone());
            let mut statements = apply_sqlite_rename_hints(&mut prev_ddl, &cur_ddl, &options)?;
            let diff = crate::sqlite::diff::compute_migration(&prev_ddl, &cur_ddl);
            statements.extend(diff.sql_statements);
            statements
        }
        (Snapshot::Postgres(p), Snapshot::Postgres(c)) => {
            let mut prev_ddl = PostgresDDL::from_entities(p.ddl.clone());
            let cur_ddl = PostgresDDL::from_entities(c.ddl.clone());
            let mut statements = apply_postgres_rename_hints(&mut prev_ddl, &cur_ddl, &options)?;
            let diff = crate::postgres::diff::compute_migration(&prev_ddl, &cur_ddl);
            statements.extend(diff.sql_statements);
            statements
        }
        _ => return Err(MigrationError::DialectMismatch),
    };

    Ok(Plan {
        statements,
        snapshot: current.clone(),
    })
}

/// Generate migration SQL from two schema values implementing [`Schema`].
///
/// This is usually the best runtime API when you already have two schema types.
pub fn diff_schemas<From: Schema, To: Schema>(
    prev: &From,
    current: &To,
) -> Result<Plan, MigrationError> {
    let prev = prev.to_snapshot();
    let current = current.to_snapshot();
    diff(&prev, &current)
}

/// Generate migration SQL from two schemas with generation options.
///
/// # Example
///
/// ```rust,no_run
/// use drizzle_migrations::{Options, Schema, Snapshot, diff_schemas_with};
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
///     Options::new().rename_column("users", "displayName", "display_name"),
/// )?;
/// # let _ = migration;
/// # Ok::<(), drizzle_migrations::MigrationError>(())
/// ```
pub fn diff_schemas_with<From: Schema, To: Schema>(
    prev: &From,
    current: &To,
    options: Options,
) -> Result<Plan, MigrationError> {
    let prev = prev.to_snapshot();
    let current = current.to_snapshot();
    diff_with(&prev, &current, options)
}

fn apply_sqlite_rename_hints(
    prev: &mut SQLiteDDL,
    cur: &SQLiteDDL,
    options: &Options,
) -> Result<Vec<String>, MigrationError> {
    let mut statements = Vec::new();

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
            hint.from, hint.to
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
            hint.table, hint.from, hint.to
        ));
        apply_sqlite_column_rename(prev, &hint.table, &hint.from, &hint.to);
    }

    Ok(statements)
}

fn apply_postgres_rename_hints(
    prev: &mut PostgresDDL,
    cur: &PostgresDDL,
    options: &Options,
) -> Result<Vec<String>, MigrationError> {
    let mut statements = Vec::new();

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
            "ALTER TABLE \"{}\".\"{}\" RENAME TO \"{}\";",
            schema, hint.from, hint.to
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
            "ALTER TABLE \"{}\".\"{}\" RENAME COLUMN \"{}\" TO \"{}\";",
            schema, hint.table, hint.from, hint.to
        ));
        apply_postgres_column_rename(prev, schema, &hint.table, &hint.from, &hint.to);
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

fn apply_postgres_table_rename(ddl: &mut PostgresDDL, schema: &str, from: &str, to: &str) {
    let to = to.to_string();

    if let Some(t) = ddl
        .tables
        .list_mut()
        .iter_mut()
        .find(|t| t.schema.as_ref() == schema && t.name.as_ref() == from)
    {
        t.name = to.clone().into();
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::Schema as MigrationSchema;
    use crate::sqlite::SQLiteSnapshot;
    use crate::sqlite::ddl::{Column, SqliteEntity, Table};

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
    fn test_generate_empty_to_empty() {
        let prev = Snapshot::empty(drizzle_types::Dialect::SQLite);
        let cur = Snapshot::empty(drizzle_types::Dialect::SQLite);
        let migration = diff(&prev, &cur).unwrap();
        assert!(migration.statements.is_empty());
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
    fn test_generate_postgres_empty() {
        let prev = Snapshot::empty(drizzle_types::Dialect::PostgreSQL);
        let cur = Snapshot::empty(drizzle_types::Dialect::PostgreSQL);
        let migration = diff(&prev, &cur).unwrap();
        assert!(migration.statements.is_empty());
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

        let options = Options::new()
            .rename_table("users", "accounts")
            .rename_column("accounts", "full_name", "display_name");

        let migration = diff_with(&prev, &cur, options).unwrap();
        assert_eq!(
            migration.statements,
            vec![
                "ALTER TABLE `users` RENAME TO `accounts`;".to_string(),
                "ALTER TABLE `accounts` RENAME COLUMN `full_name` TO `display_name`;".to_string(),
            ]
        );
    }

    #[test]
    fn test_diff_with_strict_rename_hints_errors() {
        let prev = Snapshot::empty(drizzle_types::Dialect::SQLite);
        let cur = Snapshot::empty(drizzle_types::Dialect::SQLite);
        let options = Options::new()
            .strict_renames(true)
            .rename_table("missing_table", "users");

        let result = diff_with(&prev, &cur, options);
        assert!(matches!(result, Err(MigrationError::ConfigError(_))));
    }
}

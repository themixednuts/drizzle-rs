//! [`SeedConfig`], the builder for generating seed data.

use crate::SeedError;
use crate::generator::{Generator, GeneratorKind};
use crate::identity::{ColumnId, TableId};
use drizzle_core::{Relation, SQLSchemaImpl, SQLTableInfo, SchemaHasTable, TableRef};
#[cfg(any(feature = "sqlite", feature = "postgres", feature = "mysql"))]
use drizzle_core::{SQLColumn, SQLColumnInfo};
use std::collections::{HashMap, HashSet};
use std::marker::PhantomData;
use std::sync::Arc;

#[cfg(feature = "sqlite")]
use crate::Sqlite;
#[cfg(feature = "sqlite")]
use drizzle_sqlite::traits::SQLiteColumn;
#[cfg(feature = "sqlite")]
use drizzle_sqlite::values::SQLiteValue;

#[cfg(feature = "postgres")]
use crate::Postgres;
#[cfg(feature = "postgres")]
use drizzle_postgres::traits::PostgresColumn;
#[cfg(feature = "postgres")]
use drizzle_postgres::values::PostgresValue;

#[cfg(feature = "mysql")]
use crate::MySql;
#[cfg(feature = "mysql")]
use drizzle_mysql::traits::MySQLColumn;
#[cfg(feature = "mysql")]
use drizzle_mysql::values::MySQLValue;

/// Builder that generates seed INSERT statements for a schema.
///
/// Create one with `SeedConfig::sqlite`, `SeedConfig::postgres`, or
/// `SeedConfig::mysql` (each behind its feature), chain the settings, then
/// call `generate` / `try_generate`. Tables and columns passed to the
/// builder must belong to the schema; anything else is a compile error.
///
/// Defaults: seed `0`, 10 rows per table, no skipped tables. See the
/// [crate docs](crate) for how values and row counts are chosen.
pub struct SeedConfig<'a, D, S> {
    /// Source schema.
    pub(crate) schema: &'a S,
    /// Explicitly skipped tables.
    pub(crate) skipped_tables: HashSet<TableId>,
    /// User-provided seed for deterministic RNG.
    pub(crate) seed: u64,
    /// Default number of rows per table if not overridden.
    pub(crate) default_count: usize,
    /// Per-table row count overrides.
    pub(crate) table_counts: HashMap<TableId, usize>,
    /// Per-column generator overrides.
    pub(crate) column_generators: HashMap<ColumnId, Arc<dyn Generator>>,
    /// Per-column generator kind overrides.
    pub(crate) column_kinds: HashMap<ColumnId, GeneratorKind>,
    /// Relation cardinality overrides. Key: (`parent_table`, `child_table`).
    pub(crate) relation_counts: HashMap<(TableId, TableId), usize>,
    /// Optional override for maximum parameters per INSERT statement batch.
    pub(crate) max_params_per_batch: Option<usize>,
    /// Unknown names passed to the `*_by_name` methods, reported by
    /// `try_generate` and `reset_plan`.
    pub(crate) name_errors: Vec<SeedError>,
    _dialect: PhantomData<D>,
    _schema: PhantomData<&'a S>,
}

impl<'a, D, S> SeedConfig<'a, D, S> {
    fn with_defaults(schema: &'a S) -> Self {
        Self {
            schema,
            skipped_tables: HashSet::new(),
            seed: 0,
            default_count: 10,
            table_counts: HashMap::new(),
            column_generators: HashMap::new(),
            column_kinds: HashMap::new(),
            relation_counts: HashMap::new(),
            max_params_per_batch: None,
            name_errors: Vec::new(),
            _dialect: PhantomData,
            _schema: PhantomData,
        }
    }

    /// Sets the RNG seed (default `0`). The same seed gives the same rows.
    #[must_use]
    pub const fn seed(mut self, seed: u64) -> Self {
        self.seed = seed;
        self
    }

    /// Sets the row count for tables with no explicit count and no seeded
    /// parent (default 10).
    #[must_use]
    pub const fn default_count(mut self, count: usize) -> Self {
        self.default_count = count;
        self
    }

    /// Caps the bind parameters per INSERT; rows are split across more
    /// statements to stay under it.
    ///
    /// Without this, a dialect-specific default is used.
    ///
    /// # Panics
    ///
    /// Panics if `limit` is zero.
    #[must_use]
    pub fn max_params(mut self, limit: usize) -> Self {
        assert!(limit > 0, "max_params must be > 0");
        self.max_params_per_batch = Some(limit);
        self
    }

    pub(crate) fn count_for(&self, table: TableId) -> usize {
        self.table_counts
            .get(&table)
            .copied()
            .unwrap_or(self.default_count)
    }
}

impl<D, S> SeedConfig<'_, D, S>
where
    S: SQLSchemaImpl,
{
    pub(crate) fn active_tables(&self) -> Vec<&'static TableRef> {
        self.schema
            .table_refs()
            .iter()
            .copied()
            .filter(|t| !self.skipped_tables.contains(&TableId::from_ref(t)))
            .collect()
    }

    /// Sets the row count for `table`.
    ///
    /// This wins over [`relation`](Self::relation) and
    /// [`default_count`](Self::default_count()).
    #[must_use]
    pub fn count<T>(mut self, table: &T, count: usize) -> Self
    where
        T: SQLTableInfo,
        S: SchemaHasTable<T>,
    {
        self.table_counts.insert(TableId::from_info(table), count);
        self
    }

    /// Generates `count` rows of `child` for each row of `parent`; each
    /// parent row gets `count` consecutive child rows.
    ///
    /// Without this, a child table with no explicit [`count`](Self::count)
    /// gets one row per parent row. `child` must have a foreign key to
    /// `parent` (checked at compile time).
    #[must_use]
    pub fn relation<P, C>(mut self, parent: &P, child: &C, count: usize) -> Self
    where
        P: SQLTableInfo,
        C: SQLTableInfo + Relation<P>,
        S: SchemaHasTable<P> + SchemaHasTable<C>,
    {
        self.relation_counts.insert(
            (TableId::from_info(parent), TableId::from_info(child)),
            count,
        );
        self
    }
}

impl<D, S> SeedConfig<'_, D, S> {
    /// Leaves `table` out: no INSERTs and no reset statements for it.
    ///
    /// Foreign keys that point at a skipped table keep their generated (or
    /// custom) values, for example to reference rows already in the
    /// database.
    #[must_use]
    pub fn skip<T>(mut self, table: &T) -> Self
    where
        T: SQLTableInfo,
        S: SchemaHasTable<T>,
    {
        self.skipped_tables.insert(TableId::from_info(table));
        self
    }
}

/// Name-based settings, for a [`Schema`](crate::schema::Schema) built at
/// runtime or names that come from configuration.
///
/// They take the same effect as the typed methods. A table name may be
/// qualified as `schema.table`, and must be when two namespaces have a
/// table with that name. An unknown name is reported as a
/// [`SeedError`](crate::SeedError) by `try_generate`, `try_generate_rows`
/// and `reset_plan` (and makes `generate` panic).
impl<D, S> SeedConfig<'_, D, S>
where
    S: SQLSchemaImpl,
{
    /// Sets the row count for the table named `table`, like
    /// [`count`](Self::count).
    #[must_use]
    pub fn count_by_name(mut self, table: &str, count: usize) -> Self {
        if let Some(table) = self.resolve_table(table) {
            self.table_counts.insert(TableId::from_ref(table), count);
        }
        self
    }

    /// Generates `count` rows of `child` for each row of `parent`, like
    /// [`relation`](Self::relation). `child` must have a foreign key to
    /// `parent`.
    #[must_use]
    pub fn relation_by_name(mut self, parent: &str, child: &str, count: usize) -> Self {
        let (Some(parent_ref), Some(child_ref)) =
            (self.resolve_table(parent), self.resolve_table(child))
        else {
            return self;
        };
        let parent_id = TableId::from_ref(parent_ref);
        let related = child_ref
            .foreign_keys
            .iter()
            .any(|fk| TableId::foreign_target(child_ref, fk) == parent_id);
        if related {
            self.relation_counts
                .insert((parent_id, TableId::from_ref(child_ref)), count);
        } else {
            self.name_errors.push(SeedError::NotRelated {
                parent: parent_id.to_string(),
                child: TableId::from_ref(child_ref).to_string(),
            });
        }
        self
    }

    /// Leaves the table named `table` out, like [`skip`](Self::skip).
    #[must_use]
    pub fn skip_by_name(mut self, table: &str) -> Self {
        if let Some(table) = self.resolve_table(table) {
            self.skipped_tables.insert(TableId::from_ref(table));
        }
        self
    }

    /// Uses a built-in [`GeneratorKind`] for `column` of `table`, like
    /// `kind`.
    #[must_use]
    pub fn kind_by_name(mut self, table: &str, column: &str, kind: GeneratorKind) -> Self {
        if let Some(column) = self.resolve_column(table, column) {
            self.column_kinds.insert(column, kind);
        }
        self
    }

    /// Uses `generator` for `column` of `table`, like `generator`.
    #[must_use]
    pub fn generator_by_name(
        mut self,
        table: &str,
        column: &str,
        generator: impl Generator + 'static,
    ) -> Self {
        if let Some(column) = self.resolve_column(table, column) {
            self.column_generators.insert(column, Arc::new(generator));
        }
        self
    }

    /// Finds the table named `name` (or `schema.name`), recording an error
    /// when there is no such table or the name is ambiguous.
    fn resolve_table(&mut self, name: &str) -> Option<&'static TableRef> {
        let tables = self.schema.table_refs();
        let exact: Vec<&'static TableRef> = tables
            .iter()
            .copied()
            .filter(|table| table.name == name)
            .collect();
        let candidates = if exact.is_empty() {
            name.split_once('.')
                .map_or_else(Vec::new, |(schema, table_name)| {
                    tables
                        .iter()
                        .copied()
                        .filter(|table| {
                            table.name == table_name
                                && (table.schema == Some(schema)
                                    || (schema == "public" && table.schema.is_none()))
                        })
                        .collect()
                })
        } else {
            exact
        };
        match candidates.as_slice() {
            [table] => Some(table),
            [] => {
                self.name_errors.push(SeedError::UnknownTable {
                    table: name.to_string(),
                    known: tables
                        .iter()
                        .map(|table| TableId::from_ref(table).to_string())
                        .collect(),
                });
                None
            }
            _ => {
                self.name_errors.push(SeedError::AmbiguousTable {
                    table: name.to_string(),
                    candidates: candidates
                        .iter()
                        .map(|table| TableId::from_ref(table).to_string())
                        .collect(),
                });
                None
            }
        }
    }

    fn resolve_column(&mut self, table: &str, column: &str) -> Option<ColumnId> {
        let table = self.resolve_table(table)?;
        let table_id = TableId::from_ref(table);
        if let Some(found) = table
            .columns
            .iter()
            .find(|candidate| candidate.name == column)
        {
            return Some(ColumnId::new(table_id, found.name));
        }
        self.name_errors.push(SeedError::UnknownColumn {
            table: table_id.to_string(),
            column: column.to_string(),
            known: table
                .columns
                .iter()
                .map(|column| column.name.to_string())
                .collect(),
        });
        None
    }

    /// The first unknown or ambiguous name given to a `*_by_name` method.
    pub(crate) fn check_names(&self) -> Result<(), SeedError> {
        self.name_errors.first().cloned().map_or(Ok(()), Err)
    }
}

macro_rules! dialect_seed_config {
    (
        feature = $feature:literal,
        dialect = $dialect:literal,
        marker = $marker:ty,
        constructor = $constructor:ident,
        column = $column_trait:path,
        value = $value:ty,
        seed_statement = $seed_statement:path,
        reset_statement = $reset_statement:path,
        generate = $generate:ident,
        reset = $reset:ident,
        reset_note = $reset_note:literal
    ) => {
        #[cfg(feature = $feature)]
        impl<'a> SeedConfig<'a, $marker, ()> {
            #[doc = concat!("Creates a ", $dialect, " seed config for `schema` (usually a `#[derive(...Schema)]` struct).")]
            pub fn $constructor<Schema>(schema: &'a Schema) -> SeedConfig<'a, $marker, Schema>
            where
                Schema: SQLSchemaImpl,
            {
                SeedConfig::<'a, $marker, Schema>::with_defaults(schema)
            }
        }

        #[cfg(feature = $feature)]
        impl<S> SeedConfig<'_, $marker, S>
        where
            S: SQLSchemaImpl,
        {
            /// Uses a built-in [`GeneratorKind`] for `column` instead of the
            /// inferred one.
            #[must_use]
            pub fn kind<C>(mut self, column: &C, kind: GeneratorKind) -> Self
            where
                C: SQLColumnInfo + $column_trait,
                S: SchemaHasTable<<C as SQLColumn<'static, $value>>::Table>,
            {
                self.column_kinds.insert(ColumnId::from_info(column), kind);
                self
            }

            /// Uses `generator` for `column`: one from
            /// [`generators`](crate::generators), a
            /// [`GeneratorKind`], or your own [`Generator`]. Wins over
            /// [`kind`](Self::kind).
            ///
            /// A foreign key column that references a seeded table is still
            /// overwritten with the parent row's value.
            #[must_use]
            pub fn generator<C>(mut self, column: &C, generator: impl Generator + 'static) -> Self
            where
                C: SQLColumnInfo + $column_trait,
                S: SchemaHasTable<<C as SQLColumn<'static, $value>>::Table>,
            {
                self.column_generators
                    .insert(ColumnId::from_info(column), Arc::new(generator));
                self
            }

            /// Generates the INSERT statements for every table not skipped,
            /// parents first. Large tables are split over several statements
            /// to stay under the bind-parameter limit. On `PostgreSQL`, a
            /// table's rows can be followed by a `SELECT setval(...)` that
            /// moves its `SERIAL`/`IDENTITY` sequences past the seeded ids.
            ///
            /// Execute every statement, in order.
            ///
            /// # Panics
            ///
            /// Panics when [`try_generate`](Self::try_generate) would return
            /// an error.
            #[must_use]
            pub fn generate(&self) -> Vec<$seed_statement> {
                self.try_generate()
                    .unwrap_or_else(|error| panic!("invalid seed plan: {error}"))
            }

            /// Like [`generate`](Self::generate), but returns an error instead
            /// of panicking.
            ///
            /// # Errors
            ///
            /// Returns a [`SeedError`](crate::SeedError) when the tables have a
            /// foreign-key cycle, a `NOT NULL` foreign key has no parent row
            /// to point at, a value does not fit its column, or one row
            /// exceeds the parameter limit.
            pub fn try_generate(&self) -> Result<Vec<$seed_statement>, crate::SeedError> {
                crate::Seeder::new(self).$generate()
            }

            /// Generates the rows without rendering SQL: per table, the
            /// column names and one `Vec` of [`SeedValue`](crate::SeedValue)s
            /// per row, parents first and foreign keys resolved.
            ///
            /// Use this to insert with any database driver, or to write
            /// fixtures. Values follow the same rules as
            /// [`generate`](Self::generate) for this dialect.
            ///
            /// # Errors
            ///
            /// The same as [`try_generate`](Self::try_generate), except that
            /// no parameter limit applies.
            pub fn try_generate_rows(&self) -> Result<Vec<crate::SeedRows>, crate::SeedError> {
                crate::Seeder::new(self).generate_rows()
            }

            /// Returns statements that empty the non-skipped tables, children
            /// first: `DELETE FROM` each table, preceded by an `UPDATE` that
            /// clears nullable self-references.
            ///
            /// Execute the returned statements in order on one connection.
            #[doc = $reset_note]
            ///
            /// # Errors
            ///
            /// Returns a [`SeedError`](crate::SeedError) when the tables have a
            /// foreign-key cycle, or a table to be emptied is referenced by a
            /// skipped table.
            pub fn reset_plan(&self) -> Result<Vec<$reset_statement>, crate::SeedError> {
                crate::Seeder::new(self).$reset()
            }
        }
    };
}

dialect_seed_config!(
    feature = "sqlite",
    dialect = "SQLite",
    marker = Sqlite,
    constructor = sqlite,
    column = SQLiteColumn<'static>,
    value = SQLiteValue<'static>,
    seed_statement = crate::SQLiteSeedStatement,
    reset_statement = crate::SQLiteResetStatement,
    generate = generate_sqlite,
    reset = reset_sqlite,
    reset_note = "Wrap execution in a transaction when the connection API supports one."
);

dialect_seed_config!(
    feature = "postgres",
    dialect = "PostgreSQL",
    marker = Postgres,
    constructor = postgres,
    column = PostgresColumn<'static>,
    value = PostgresValue<'static>,
    seed_statement = crate::PostgresSeedStatement,
    reset_statement = crate::PostgresResetStatement,
    generate = generate_postgres,
    reset = reset_postgres,
    reset_note = "Wrap execution in a transaction when the connection API supports one."
);

dialect_seed_config!(
    feature = "mysql",
    dialect = "MySQL",
    marker = MySql,
    constructor = mysql,
    column = MySQLColumn<'static>,
    value = MySQLValue<'static>,
    seed_statement = crate::MySQLSeedStatement,
    reset_statement = crate::MySQLResetStatement,
    generate = generate_mysql,
    reset = reset_mysql,
    reset_note = "The plan appends `ALTER TABLE ... AUTO_INCREMENT = 1` for auto-increment tables. Those statements implicitly commit in MySQL, so callers must not assume the whole reset is transactional."
);

//! [`SeedConfig`], the builder for generating seed data.

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

            /// Uses a custom [`Generator`] for `column`. Wins over
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
            /// to stay under the bind-parameter limit.
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

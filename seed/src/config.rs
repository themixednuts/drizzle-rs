//! Seeder configuration with type-safe builder API.

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

/// Configuration for seeding a schema.
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

    /// Set the random seed for deterministic generation.
    #[must_use]
    pub const fn seed(mut self, seed: u64) -> Self {
        self.seed = seed;
        self
    }

    /// Set the default row count for all tables.
    #[must_use]
    pub const fn default_count(mut self, count: usize) -> Self {
        self.default_count = count;
        self
    }

    /// Override the maximum number of bind parameters per INSERT statement batch.
    ///
    /// # Panics
    ///
    /// Panics if `limit` is zero — every batch must be able to bind at least one
    /// row, and a zero limit would produce infinite batching.
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

    /// Set the row count for a specific table.
    #[must_use]
    pub fn count<T>(mut self, table: &T, count: usize) -> Self
    where
        T: SQLTableInfo,
        S: SchemaHasTable<T>,
    {
        self.table_counts.insert(TableId::from_info(table), count);
        self
    }

    /// Set how many child rows to generate per parent row for a relation.
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
    /// Skip a table from seeding.
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
            #[doc = concat!("Create a ", $dialect, " seeder config from a derived schema.")]
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
            /// Override the generator kind for a specific column.
            #[must_use]
            pub fn kind<C>(mut self, column: &C, kind: GeneratorKind) -> Self
            where
                C: SQLColumnInfo + $column_trait,
                S: SchemaHasTable<<C as SQLColumn<'static, $value>>::Table>,
            {
                self.column_kinds.insert(ColumnId::from_info(column), kind);
                self
            }

            /// Override the generator for a specific column.
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

            /// Generate INSERT statements for the active table set.
            ///
            /// # Panics
            ///
            /// Panics when the selected schema cannot form a valid portable
            /// seed plan. Use [`Self::try_generate`] to handle that case.
            #[must_use]
            pub fn generate(&self) -> Vec<$seed_statement> {
                self.try_generate()
                    .unwrap_or_else(|error| panic!("invalid seed plan: {error}"))
            }

            /// Try to generate INSERT statements for the active table set.
            ///
            /// # Errors
            ///
            /// Returns [`crate::SeedError`] when the selected tables cannot be
            /// ordered safely, a relation has no generated parent values, a
            /// value cannot be represented, or one row exceeds the parameter
            /// limit.
            pub fn try_generate(&self) -> Result<Vec<$seed_statement>, crate::SeedError> {
                crate::Seeder::new(self).$generate()
            }

            /// Build child-before-parent cleanup statements for the selected
            /// tables.
            ///
            /// Execute the returned statements in order on one connection.
            #[doc = $reset_note]
            ///
            /// # Errors
            ///
            /// Returns [`crate::SeedError`] when the selected tables cannot be
            /// reset safely, including cyclic dependencies and a selected
            /// parent referenced by a skipped child.
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

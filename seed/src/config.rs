//! Seeder configuration with type-safe builder API.

use crate::generator::{Generator, GeneratorKind};
use drizzle_core::{
    Relation, SQLColumn, SQLColumnInfo, SQLSchemaImpl, SQLTableInfo, SchemaHasTable,
};
use std::any::TypeId;
use std::collections::{HashMap, HashSet};
use std::marker::PhantomData;
use std::sync::Arc;

#[cfg(feature = "sqlite")]
use crate::Sqlite;
#[cfg(feature = "sqlite")]
use drizzle_sqlite::traits::SQLiteColumn;
#[cfg(feature = "sqlite")]
use drizzle_sqlite::traits::SQLiteTable;
#[cfg(feature = "sqlite")]
use drizzle_sqlite::traits::SQLiteTableInfo;
#[cfg(feature = "sqlite")]
use drizzle_sqlite::values::SQLiteValue;

#[cfg(feature = "postgres")]
use crate::Postgres;
#[cfg(feature = "postgres")]
use drizzle_postgres::traits::PostgresColumn;
#[cfg(feature = "postgres")]
use drizzle_postgres::traits::PostgresTable;
#[cfg(feature = "postgres")]
use drizzle_postgres::traits::PostgresTableInfo;
#[cfg(feature = "postgres")]
use drizzle_postgres::values::PostgresValue;

/// Configuration for seeding a schema.
pub struct SeedConfig<'a, D, S> {
    /// Source schema.
    pub(crate) schema: &'a S,
    /// Explicitly skipped tables (by concrete table type).
    pub(crate) skipped_tables: HashSet<TypeId>,
    /// User-provided seed for deterministic RNG.
    pub(crate) seed: u64,
    /// Default number of rows per table if not overridden.
    pub(crate) default_count: usize,
    /// Per-table row count overrides.
    pub(crate) table_counts: HashMap<String, usize>,
    /// Per-column generator overrides.
    pub(crate) column_generators: HashMap<(String, String), Arc<dyn Generator>>,
    /// Per-column generator kind overrides.
    pub(crate) column_kinds: HashMap<(String, String), GeneratorKind>,
    /// Relation cardinality overrides. Key: (parent_table, child_table).
    pub(crate) relation_counts: HashMap<(String, String), usize>,
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
    pub fn seed(mut self, seed: u64) -> Self {
        self.seed = seed;
        self
    }

    /// Set the default row count for all tables.
    pub fn default_count(mut self, count: usize) -> Self {
        self.default_count = count;
        self
    }

    /// Override the maximum number of bind parameters per INSERT statement batch.
    pub fn max_params(mut self, limit: usize) -> Self {
        assert!(limit > 0, "max_params must be > 0");
        self.max_params_per_batch = Some(limit);
        self
    }

    pub(crate) fn count_for(&self, table: &str) -> usize {
        self.table_counts
            .get(table)
            .copied()
            .unwrap_or(self.default_count)
    }
}

impl<'a, D, S> SeedConfig<'a, D, S>
where
    S: SQLSchemaImpl,
{
    pub(crate) fn active_tables(&self) -> Vec<&'static dyn SQLTableInfo> {
        self.schema
            .tables()
            .iter()
            .copied()
            .filter(|t| !self.skipped_tables.contains(&t.type_id()))
            .collect()
    }
}

impl<'a, D, S> SeedConfig<'a, D, S> {
    /// Skip a table from seeding.
    pub fn skip<T>(mut self, _table: &T) -> Self
    where
        T: SQLTableInfo,
        S: SchemaHasTable<T>,
    {
        self.skipped_tables.insert(TypeId::of::<T>());
        self
    }
}

#[cfg(feature = "sqlite")]
impl<'a> SeedConfig<'a, Sqlite, ()> {
    /// Create a SQLite seeder config from a derived schema.
    pub fn sqlite<Schema>(schema: &'a Schema) -> SeedConfig<'a, Sqlite, Schema>
    where
        Schema: SQLSchemaImpl,
    {
        SeedConfig::<'a, Sqlite, Schema>::with_defaults(schema)
    }
}

#[cfg(feature = "sqlite")]
impl<'a, S> SeedConfig<'a, Sqlite, S>
where
    S: SQLSchemaImpl,
{
    /// Set the row count for a specific table.
    pub fn count<T>(mut self, table: &T, count: usize) -> Self
    where
        T: SQLiteTableInfo,
        S: SchemaHasTable<T>,
    {
        self.table_counts.insert(table.name().to_string(), count);
        self
    }

    /// Set how many child rows to generate per parent row for a relation.
    pub fn relation<P, C>(mut self, parent: &P, child: &C, count: usize) -> Self
    where
        P: SQLiteTableInfo + SQLiteTable<'static>,
        C: SQLiteTableInfo + SQLiteTable<'static> + Relation<P>,
        S: SchemaHasTable<P> + SchemaHasTable<C>,
    {
        self.relation_counts
            .insert((parent.name().to_string(), child.name().to_string()), count);
        self
    }

    /// Override the generator kind for a specific column.
    pub fn kind<C>(mut self, column: &C, kind: GeneratorKind) -> Self
    where
        C: SQLColumnInfo + SQLiteColumn<'static>,
        S: SchemaHasTable<<C as SQLColumn<'static, SQLiteValue<'static>>>::Table>,
    {
        let key = (column.table().name().to_string(), column.name().to_string());
        self.column_kinds.insert(key, kind);
        self
    }

    /// Override the generator for a specific column.
    pub fn generator<C>(mut self, column: &C, g: impl Generator + 'static) -> Self
    where
        C: SQLColumnInfo + SQLiteColumn<'static>,
        S: SchemaHasTable<<C as SQLColumn<'static, SQLiteValue<'static>>>::Table>,
    {
        let key = (column.table().name().to_string(), column.name().to_string());
        self.column_generators.insert(key, Arc::new(g));
        self
    }

    /// Generate INSERT statements for the active table set.
    pub fn generate(&self) -> Vec<crate::SQLiteSeedStatement> {
        crate::Seeder::new(self).generate_sqlite()
    }
}

#[cfg(feature = "postgres")]
impl<'a> SeedConfig<'a, Postgres, ()> {
    /// Create a PostgreSQL seeder config from a derived schema.
    pub fn postgres<Schema>(schema: &'a Schema) -> SeedConfig<'a, Postgres, Schema>
    where
        Schema: SQLSchemaImpl,
    {
        SeedConfig::<'a, Postgres, Schema>::with_defaults(schema)
    }
}

#[cfg(feature = "postgres")]
impl<'a, S> SeedConfig<'a, Postgres, S>
where
    S: SQLSchemaImpl,
{
    /// Set the row count for a specific table.
    pub fn count<T>(mut self, table: &T, count: usize) -> Self
    where
        T: PostgresTableInfo,
        S: SchemaHasTable<T>,
    {
        self.table_counts.insert(table.name().to_string(), count);
        self
    }

    /// Set how many child rows to generate per parent row for a relation.
    pub fn relation<P, C>(mut self, parent: &P, child: &C, count: usize) -> Self
    where
        P: PostgresTableInfo + PostgresTable<'static>,
        C: PostgresTableInfo + PostgresTable<'static> + Relation<P>,
        S: SchemaHasTable<P> + SchemaHasTable<C>,
    {
        self.relation_counts
            .insert((parent.name().to_string(), child.name().to_string()), count);
        self
    }

    /// Override the generator kind for a specific column.
    pub fn kind<C>(mut self, column: &C, kind: GeneratorKind) -> Self
    where
        C: SQLColumnInfo + PostgresColumn<'static>,
        S: SchemaHasTable<<C as SQLColumn<'static, PostgresValue<'static>>>::Table>,
    {
        let key = (column.table().name().to_string(), column.name().to_string());
        self.column_kinds.insert(key, kind);
        self
    }

    /// Override the generator for a specific column.
    pub fn generator<C>(mut self, column: &C, g: impl Generator + 'static) -> Self
    where
        C: SQLColumnInfo + PostgresColumn<'static>,
        S: SchemaHasTable<<C as SQLColumn<'static, PostgresValue<'static>>>::Table>,
    {
        let key = (column.table().name().to_string(), column.name().to_string());
        self.column_generators.insert(key, Arc::new(g));
        self
    }

    /// Generate INSERT statements for the active table set.
    pub fn generate(&self) -> Vec<crate::PostgresSeedStatement> {
        crate::Seeder::new(self).generate_postgres()
    }
}

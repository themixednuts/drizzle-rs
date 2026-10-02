//! Deterministic test data for drizzle-rs schemas.
//!
//! [`SeedConfig`] turns a schema into INSERT statements, or into plain rows
//! with [`try_generate_rows`](SeedConfig::try_generate_rows) for use with
//! any driver. The same seed and crate version always give the same rows.
//!
//! How values are chosen, per column:
//! 1. a [`Generator`] set with `.generator(...)`: one from [`generators`]
//!    (`generators::int(18..=90)`, `generators::one_of([...])`,
//!    `generators::from_fn(...)`, ...), a [`GeneratorKind`], or your own
//!    type; else
//! 2. a [`GeneratorKind`] set with `.kind(...)`; else
//! 3. `DEFAULT` when the column has a default (and is not the primary key),
//!    or is a non-key `PostgreSQL` identity column; else
//! 4. an inferred generator:
//!    - integer primary keys count up from 1;
//!    - enum columns pick one of their variants;
//!    - MySQL columns follow their declared domain (integer ranges, inline
//!      `ENUM`/`SET` labels, `DECIMAL` precision, ...);
//!    - otherwise the column name decides when a whole word is recognized
//!      (`email`, `first_name`, `created_at`, `is_active`, ...) and the
//!      generated values fit the column type, then the SQL type alone.
//!
//!    Text is cut to a declared `VARCHAR(n)`/`CHAR(n)` length.
//!
//! `UNIQUE` columns and single-column primary keys get distinct values, and
//! a row that would repeat a composite primary key or multi-column `UNIQUE`
//! key (for example two equal `(user_id, post_id)` pairs in a join table)
//! is dropped.
//! Parent tables are seeded before their children, and foreign key columns
//! are overwritten to point at generated parent rows. A child table without
//! its own count gets `parent rows × relation count` rows (the relation count
//! defaults to 1; with several parents, the largest product wins).
//! `reset_plan` returns `DELETE` statements in child-before-parent order.
//!
//! On `PostgreSQL`, text values for non-text columns (`uuid`, `jsonb`,
//! enums, arrays, ...) are cast to the column type, `GENERATED ALWAYS`
//! identity keys are inserted with `OVERRIDING SYSTEM VALUE`, and each
//! table's `SERIAL`/`IDENTITY` sequences are moved past the seeded ids with
//! a `SELECT setval(...)` statement after its rows.
//!
//! The crate has no default dialect: enable `sqlite`, `postgres`, and/or
//! `mysql`.
//!
//! # With the schema macros
//!
//! This is the main path: pass the `#[derive(...Schema)]` struct. The
//! macros already record column types, keys, `UNIQUE`, defaults and enum
//! variants, and every table or column passed to the config is checked at
//! compile time. (Uses `drizzle` with the `rusqlite` feature; not compiled
//! here because `drizzle` is not a dependency of this crate.)
//!
//! ```text
//! use drizzle::sqlite::prelude::*;
//! use drizzle_seed::{SeedConfig, generators::{self, GeneratorExt}};
//!
//! #[SQLiteTable]
//! struct Users {
//!     #[column(primary)]
//!     id: i32,
//!     #[column(unique)]
//!     email: String,    // inferred from the name: distinct emails
//!     age: i32,
//! }
//!
//! #[SQLiteTable]
//! struct Posts {
//!     #[column(primary)]
//!     id: i32,
//!     #[column(references = Users::id)]
//!     user_id: i32,     // points at seeded users
//!     title: String,    // inferred from the name: a short title
//! }
//!
//! #[derive(SQLiteSchema)]
//! struct AppSchema {
//!     users: Users,
//!     posts: Posts,
//! }
//!
//! let schema = AppSchema::new();
//! let statements = SeedConfig::sqlite(&schema)
//!     .seed(42)
//!     .count(&schema.users, 5)                     // 5 users
//!     .relation(&schema.users, &schema.posts, 3)   // 3 posts per user: 15 posts
//!     .generator(&schema.users.age, generators::int(18..=90).nullable(0.1))
//!     .generate();
//!
//! for statement in statements {
//!     db.execute(statement)?; // parents first
//! }
//! ```
//!
//! # Without the schema macros
//!
//! Describe the existing tables with [`schema::Schema`], then use the
//! `*_by_name` settings. Names are checked when the seed is generated, and
//! [`try_generate_rows`](SeedConfig::try_generate_rows) gives plain rows
//! for any driver:
//!
//! ```rust
//! # #[cfg(feature = "postgres")]
//! # {
//! use drizzle_seed::schema::{Column, Schema, Table};
//! use drizzle_seed::{SeedConfig, SeedError};
//!
//! let schema = Schema::postgres()
//!     .table(
//!         Table::new("users")
//!             .column(Column::new("id", "BIGSERIAL").primary_key())
//!             .column(Column::new("email", "TEXT").not_null().unique()),
//!     )
//!     .table(
//!         Table::new("posts")
//!             .column(Column::new("id", "BIGSERIAL").primary_key())
//!             .column(Column::new("user_id", "BIGINT").not_null().references("users", "id"))
//!             .column(Column::new("title", "TEXT").not_null()),
//!     );
//!
//! let config = SeedConfig::postgres(&schema)
//!     .count_by_name("users", 5)
//!     .relation_by_name("users", "posts", 3);
//! for table in config.try_generate_rows()? {
//!     // INSERT INTO {table.table} ({table.columns}) VALUES ... for each row
//!     assert_eq!(table.rows.len(), if table.table == "users" { 5 } else { 15 });
//! }
//!
//! // Execute the reset plan, children first, to empty the tables again.
//! let reset = config.reset_plan()?;
//! assert_eq!(reset.len(), 2);
//!
//! // A typo is an error, which lists the names that do exist.
//! let error = SeedConfig::postgres(&schema).count_by_name("user", 5).try_generate();
//! assert!(matches!(error, Err(SeedError::UnknownTable { .. })));
//! # }
//! # Ok::<(), drizzle_seed::SeedError>(())
//! ```
//!
//! Any other type that implements [`drizzle_core::SQLSchemaImpl`] works as
//! a schema too.

// The crate intentionally has no default dialect. Its planner is dormant in
// that feature-isolation build and becomes reachable once any dialect is on.
#![cfg_attr(
    not(any(feature = "sqlite", feature = "postgres", feature = "mysql")),
    allow(dead_code)
)]

pub(crate) mod batch;
pub(crate) mod config;
pub(crate) mod datasets;
mod error;
pub(crate) mod generator;
pub(crate) mod identity;
pub(crate) mod inference;
#[cfg(feature = "mysql")]
mod mysql_seed;
pub(crate) mod rng;
pub(crate) mod topology;

pub mod generators;
#[cfg(any(feature = "sqlite", feature = "postgres", feature = "mysql"))]
pub mod schema;

pub use config::SeedConfig;
pub use error::SeedError;
pub use generator::{Generator, GeneratorKind, RngCore, SeedValue};
/// Re-export of `rand::Rng`, for drawing values from the RNG a
/// [`Generator`] receives (`rng.random_range(..)`, `rng.random_bool(..)`).
pub use rand::Rng;

use drizzle_core::{ColumnRef, TableRef};
use rand::rngs::StdRng;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

#[cfg(any(feature = "sqlite", feature = "postgres", feature = "mysql"))]
use drizzle_core::{OwnedSQL, SQL, SQLChunk, Token, param::Param, traits::ToSQL};

#[cfg(any(
    feature = "postgres",
    all(test, any(feature = "sqlite", feature = "mysql"))
))]
use drizzle_core::ColumnDialect;

#[cfg(any(feature = "sqlite", feature = "postgres", feature = "mysql"))]
use std::borrow::Cow;

use identity::{ColumnId, TableId};

#[cfg(feature = "sqlite")]
pub use statement::SQLiteResetStatement;
#[cfg(feature = "sqlite")]
pub use statement::SQLiteSeedStatement;

#[cfg(feature = "postgres")]
pub use statement::PostgresResetStatement;
#[cfg(feature = "postgres")]
pub use statement::PostgresSeedStatement;

#[cfg(feature = "mysql")]
pub use statement::MySQLResetStatement;
#[cfg(feature = "mysql")]
pub use statement::MySQLSeedStatement;

#[cfg(feature = "sqlite")]
use drizzle_sqlite::values::{OwnedSQLiteValue, SQLiteValue};

#[cfg(feature = "postgres")]
use drizzle_postgres::values::{OwnedPostgresValue, PostgresValue};

#[cfg(feature = "mysql")]
use drizzle_mysql::values::{MySQLValue, OwnedMySQLValue};

#[cfg(all(feature = "postgres", feature = "chrono"))]
use chrono::{DateTime, NaiveDate, NaiveDateTime, NaiveTime, Utc};

// ---------------------------------------------------------------------------
// Dialect marker types — encode the target database in the type system
// ---------------------------------------------------------------------------

/// SQLite marker for [`SeedConfig`]; created by [`SeedConfig::sqlite`].
#[cfg(feature = "sqlite")]
pub struct Sqlite;

/// PostgreSQL marker for [`SeedConfig`]; created by [`SeedConfig::postgres`].
#[cfg(feature = "postgres")]
pub struct Postgres;

/// MySQL marker for [`SeedConfig`]; created by [`SeedConfig::mysql`].
#[cfg(feature = "mysql")]
pub struct MySql;

// ---------------------------------------------------------------------------
// Seed statement types
// ---------------------------------------------------------------------------

mod statement {
    #[cfg(any(feature = "sqlite", feature = "postgres", feature = "mysql"))]
    use super::{Cow, OwnedSQL, Param, SQL, SQLChunk, ToSQL};

    #[cfg(feature = "sqlite")]
    use super::{OwnedSQLiteValue, SQLiteValue};

    #[cfg(feature = "postgres")]
    use super::{OwnedPostgresValue, PostgresValue};

    #[cfg(feature = "mysql")]
    use super::{MySQLValue, OwnedMySQLValue};

    // Generic OwnedSQL → SQL conversion (borrowing)
    #[cfg(any(feature = "sqlite", feature = "postgres", feature = "mysql"))]
    fn convert_to_sql<'a, Owned, Borrowed>(owned: &OwnedSQL<Owned>) -> SQL<'a, Borrowed>
    where
        Owned: drizzle_core::SQLParam,
        Borrowed: drizzle_core::SQLParam + From<Owned>,
    {
        let chunks = owned
            .chunks
            .iter()
            .map(|chunk| match chunk {
                drizzle_core::OwnedSQLChunk::Token(t) => SQLChunk::Token(*t),
                drizzle_core::OwnedSQLChunk::Ident(s) => SQLChunk::Ident(Cow::Owned(s.to_string())),
                drizzle_core::OwnedSQLChunk::Raw(s) => SQLChunk::Raw(Cow::Owned(s.to_string())),
                drizzle_core::OwnedSQLChunk::Number(v) => SQLChunk::Number(*v),
                drizzle_core::OwnedSQLChunk::Param(p) => SQLChunk::Param(Param {
                    placeholder: p.placeholder,
                    value: p
                        .value
                        .as_ref()
                        .map(|v| Cow::Owned(Borrowed::from(v.clone()))),
                }),
                drizzle_core::OwnedSQLChunk::Table(t) => SQLChunk::Table(*t),
                drizzle_core::OwnedSQLChunk::Column(c) => SQLChunk::Column(*c),
            })
            .collect();
        SQL { chunks }
    }

    // Generic OwnedSQL → SQL conversion (consuming — avoids cloning values)
    #[cfg(any(feature = "sqlite", feature = "postgres", feature = "mysql"))]
    fn convert_into_sql<'a, Owned, Borrowed>(owned: OwnedSQL<Owned>) -> SQL<'a, Borrowed>
    where
        Owned: drizzle_core::SQLParam,
        Borrowed: drizzle_core::SQLParam + From<Owned>,
    {
        let chunks = owned
            .chunks
            .into_iter()
            .map(|chunk| match chunk {
                drizzle_core::OwnedSQLChunk::Token(t) => SQLChunk::Token(t),
                drizzle_core::OwnedSQLChunk::Ident(s) => {
                    SQLChunk::Ident(Cow::Owned(String::from(s)))
                }
                drizzle_core::OwnedSQLChunk::Raw(s) => SQLChunk::Raw(Cow::Owned(String::from(s))),
                drizzle_core::OwnedSQLChunk::Number(v) => SQLChunk::Number(v),
                drizzle_core::OwnedSQLChunk::Param(p) => SQLChunk::Param(Param {
                    placeholder: p.placeholder,
                    value: p.value.map(|v| Cow::Owned(Borrowed::from(v))),
                }),
                drizzle_core::OwnedSQLChunk::Table(t) => SQLChunk::Table(t),
                drizzle_core::OwnedSQLChunk::Column(c) => SQLChunk::Column(c),
            })
            .collect();
        SQL { chunks }
    }

    macro_rules! seed_statement {
        ($name:ident, $owned:ty, $borrowed:ty, $feature:literal) => {
            #[cfg(feature = $feature)]
            #[derive(Debug, Clone)]
            /// One SQL statement produced by [`SeedConfig`](crate::SeedConfig),
            /// owning its bound values.
            ///
            /// Inspect it with [`sql`](Self::sql) or [`build`](Self::build),
            /// or pass it to the matching drizzle driver to execute it.
            pub struct $name {
                pub(crate) inner: OwnedSQL<$owned>,
            }

            #[cfg(feature = $feature)]
            impl $name {
                /// Returns the SQL text, with placeholders for bound values.
                pub fn sql(&self) -> String {
                    self.inner.to_sql().build().0
                }

                /// Returns the SQL text and the values bound to its placeholders.
                pub fn build(&self) -> (String, Vec<$owned>) {
                    let sql = self.inner.to_sql();
                    let (text, params) = sql.build();
                    (text, params.into_iter().cloned().collect())
                }
            }

            #[cfg(feature = $feature)]
            impl std::fmt::Display for $name {
                fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                    f.write_str(&self.sql())
                }
            }

            #[cfg(feature = $feature)]
            impl<'a> ToSQL<'a, $borrowed> for $name {
                fn to_sql(&self) -> SQL<'a, $borrowed> {
                    convert_to_sql(&self.inner)
                }

                fn into_sql(self) -> SQL<'a, $borrowed> {
                    convert_into_sql(self.inner)
                }
            }
        };
    }

    seed_statement!(
        SQLiteSeedStatement,
        OwnedSQLiteValue,
        SQLiteValue<'a>,
        "sqlite"
    );
    seed_statement!(
        SQLiteResetStatement,
        OwnedSQLiteValue,
        SQLiteValue<'a>,
        "sqlite"
    );
    seed_statement!(
        PostgresSeedStatement,
        OwnedPostgresValue,
        PostgresValue<'a>,
        "postgres"
    );
    seed_statement!(
        PostgresResetStatement,
        OwnedPostgresValue,
        PostgresValue<'a>,
        "postgres"
    );
    seed_statement!(MySQLSeedStatement, OwnedMySQLValue, MySQLValue<'a>, "mysql");
    seed_statement!(
        MySQLResetStatement,
        OwnedMySQLValue,
        MySQLValue<'a>,
        "mysql"
    );
}

// ---------------------------------------------------------------------------
// Dialect-free output
// ---------------------------------------------------------------------------

/// The generated rows for one table, before any SQL is rendered.
///
/// Returned by `SeedConfig::try_generate_rows`, in insert order (parents
/// before children), with foreign keys already pointing at parent rows. Use
/// it to insert with any driver, write fixtures, or inspect what a seed
/// produces. Generated (computed) columns are left out.
#[derive(Debug, Clone, PartialEq)]
pub struct SeedRows {
    /// The table's schema, if it has one.
    pub schema: Option<&'static str>,
    /// The table name.
    pub table: &'static str,
    /// Column names, in the order of each row's values.
    pub columns: Vec<&'static str>,
    /// One `Vec` of values per row, in `columns` order.
    pub rows: Vec<Vec<SeedValue>>,
}

// ---------------------------------------------------------------------------
// Internal: generated data awaiting SQL rendering
// ---------------------------------------------------------------------------

struct GeneratedChunk<'a> {
    table: &'a TableRef,
    rows: Vec<Vec<SeedValue>>,
}

#[derive(Clone)]
struct RelationSpec {
    target_table: TableId,
    fk_columns: &'static [&'static str],
    ref_columns: &'static [&'static str],
    children_per_parent: usize,
}

struct RelationContext<'plan, 'schema> {
    source_table: &'schema TableRef,
    column_indexes: &'plan HashMap<&'static str, usize>,
    specs: &'plan [RelationSpec],
    generated_values: &'plan HashMap<ColumnId, Vec<SeedValue>>,
    generated_counts: &'plan HashMap<TableId, usize>,
    active_tables: &'plan HashMap<TableId, &'schema TableRef>,
}

// ---------------------------------------------------------------------------
// Seeder (fully internal — public API is SeedConfig::generate)
// ---------------------------------------------------------------------------

struct Seeder<'a, D, S> {
    config: &'a SeedConfig<'a, D, S>,
}

impl<'a, D, S> Seeder<'a, D, S>
where
    S: drizzle_core::SQLSchemaImpl,
{
    const fn new(config: &'a SeedConfig<'a, D, S>) -> Self {
        Self { config }
    }

    fn generate_chunks(
        &self,
        dialect_max_params: usize,
    ) -> Result<Vec<GeneratedChunk<'a>>, SeedError> {
        self.config.check_names()?;
        let active_tables = self.config.active_tables();
        let order = topology::seeding_order(&active_tables).map_err(|error| {
            SeedError::CyclicForeignKeys {
                tables: error
                    .tables
                    .into_iter()
                    .map(|table| table.to_string())
                    .collect(),
            }
        })?;
        let table_map: HashMap<TableId, &TableRef> = active_tables
            .iter()
            .map(|table| (TableId::from_ref(table), *table))
            .collect();
        let mut table_name_counts: HashMap<&'static str, usize> = HashMap::new();
        for table in &active_tables {
            *table_name_counts.entry(table.name).or_default() += 1;
        }

        let mut generated_values: HashMap<ColumnId, Vec<SeedValue>> = HashMap::new();
        let mut generated_counts: HashMap<TableId, usize> = HashMap::new();
        let mut chunks_out = Vec::new();

        for table_id in order {
            let Some(&table) = table_map.get(&table_id) else {
                continue;
            };

            let columns = table.columns;
            if columns.is_empty() {
                continue;
            }

            let count = self.derived_count_for(table, &generated_counts);
            if count == 0 {
                generated_counts.insert(table_id, 0);
                continue;
            }

            let generators = self.build_generators(table);
            let col_index_map: HashMap<&'static str, usize> = columns
                .iter()
                .enumerate()
                .map(|(idx, col)| (col.name, idx))
                .collect();
            let relation_specs = self.relation_specs_for(table);

            let mut all_rows: Vec<Vec<SeedValue>> = Vec::with_capacity(count);
            let mut col_rngs: Vec<StdRng> = columns
                .iter()
                .map(|column| {
                    rng::table_column_rng(
                        table_id,
                        column.name,
                        self.config.seed,
                        table_name_counts.get(table.name).copied().unwrap_or(0) > 1,
                    )
                })
                .collect();

            let mut unique_seen: Vec<Option<HashSet<String>>> = columns
                .iter()
                .map(|column| unique_column(table, column).then(HashSet::new))
                .collect();

            for row_idx in 0..count {
                let mut row = Vec::with_capacity(columns.len());
                for (col_idx, generator) in generators.iter().enumerate() {
                    let column = &columns[col_idx];
                    let rng = &mut col_rngs[col_idx];
                    let mut val = generator.generate(rng, row_idx, column.sql_type);
                    if let Some(seen) = unique_seen[col_idx].as_mut() {
                        val = unique_value(
                            val,
                            seen,
                            row_idx,
                            column,
                            |rng| generator.generate(rng, row_idx, column.sql_type),
                            rng,
                        );
                    }
                    row.push(val);
                }

                Self::apply_many_to_one_relations(
                    &mut row,
                    row_idx,
                    &RelationContext {
                        source_table: table,
                        column_indexes: &col_index_map,
                        specs: &relation_specs,
                        generated_values: &generated_values,
                        generated_counts: &generated_counts,
                        active_tables: &table_map,
                    },
                )?;

                all_rows.push(row);
            }

            // Foreign key values come from the parent rows, so two rows can
            // repeat a composite key (`(user_id, post_id)` in a join table).
            // Drop the repeats instead of emitting an INSERT that fails.
            drop_composite_key_repeats(table, &col_index_map, &mut all_rows);
            let count = all_rows.len();

            // Store generated values for all columns for FK/composite resolution
            for (col_idx, col) in columns.iter().enumerate() {
                let vals: Vec<SeedValue> =
                    all_rows.iter().map(|row| row[col_idx].clone()).collect();
                generated_values.insert(ColumnId::new(table_id, col.name), vals);
            }

            generated_counts.insert(table_id, count);

            let param_limit = self
                .config
                .max_params_per_batch
                .unwrap_or(dialect_max_params)
                .max(1);

            for (start, end) in
                batch_ranges_by_param_limit(&all_rows, param_limit).map_err(|required| {
                    SeedError::ParameterLimitTooLow {
                        table: table_id.to_string(),
                        required,
                        limit: param_limit,
                    }
                })?
            {
                chunks_out.push(GeneratedChunk {
                    table,
                    rows: all_rows[start..end].to_vec(),
                });
            }
        }

        Ok(chunks_out)
    }

    fn generate_rows(&self) -> Result<Vec<SeedRows>, SeedError> {
        let mut out: Vec<SeedRows> = Vec::new();
        for chunk in self.generate_chunks(usize::MAX)? {
            let kept: Vec<usize> = chunk
                .table
                .columns
                .iter()
                .enumerate()
                .filter(|(_, column)| generated_expression(column).is_none())
                .map(|(index, _)| index)
                .collect();
            let rows = chunk
                .rows
                .into_iter()
                .map(|row| kept.iter().map(|&index| row[index].clone()).collect());
            match out.last_mut() {
                Some(last)
                    if last.table == chunk.table.name && last.schema == chunk.table.schema =>
                {
                    last.rows.extend(rows);
                }
                _ => out.push(SeedRows {
                    schema: chunk.table.schema,
                    table: chunk.table.name,
                    columns: kept
                        .iter()
                        .map(|&index| chunk.table.columns[index].name)
                        .collect(),
                    rows: rows.collect(),
                }),
            }
        }
        Ok(out)
    }

    fn reset_tables(&self) -> Result<Vec<&'static TableRef>, SeedError> {
        self.config.check_names()?;
        let all_tables = self.config.schema.table_refs();
        let active_tables = self.config.active_tables();
        let active_ids: HashSet<_> = active_tables
            .iter()
            .map(|table| TableId::from_ref(table))
            .collect();

        for child in all_tables {
            let child_id = TableId::from_ref(child);
            if active_ids.contains(&child_id) {
                continue;
            }
            for foreign_key in child.foreign_keys {
                let parent_id = TableId::foreign_target(child, foreign_key);
                if active_ids.contains(&parent_id) {
                    return Err(SeedError::UnsafeResetSelection {
                        parent: parent_id.to_string(),
                        skipped_child: child_id.to_string(),
                    });
                }
            }
        }

        let order = topology::seeding_order(&active_tables).map_err(|error| {
            SeedError::CyclicForeignKeys {
                tables: error
                    .tables
                    .into_iter()
                    .map(|table| table.to_string())
                    .collect(),
            }
        })?;
        let table_map: HashMap<_, _> = active_tables
            .into_iter()
            .map(|table| (TableId::from_ref(table), table))
            .collect();
        Ok(order
            .into_iter()
            .rev()
            .filter_map(|table| table_map.get(&table).copied())
            .collect())
    }

    fn derived_count_for(
        &self,
        table: &TableRef,
        generated_counts: &HashMap<TableId, usize>,
    ) -> usize {
        let table_id = TableId::from_ref(table);
        if let Some(&count) = self.config.table_counts.get(&table_id) {
            return count;
        }

        let mut derived: Option<usize> = None;
        for parent_id in Self::parent_table_ids(table) {
            if let Some(&parent_count) = generated_counts.get(&parent_id) {
                let children_per_parent = self
                    .config
                    .relation_counts
                    .get(&(parent_id, table_id))
                    .copied()
                    .unwrap_or(1);
                let child_count = parent_count.saturating_mul(children_per_parent);
                derived = Some(derived.map_or(child_count, |current| current.max(child_count)));
            }
        }

        derived.unwrap_or_else(|| self.config.count_for(table_id))
    }

    fn parent_table_ids(table: &TableRef) -> Vec<TableId> {
        let mut seen = HashSet::new();
        let mut parent_ids = Vec::new();
        let table_id = TableId::from_ref(table);

        for fk in table.foreign_keys {
            let parent = TableId::foreign_target(table, fk);
            if parent != table_id && seen.insert(parent) {
                parent_ids.push(parent);
            }
        }

        parent_ids
    }

    fn build_generators(&self, table: &TableRef) -> Vec<Box<dyn Generator>> {
        let table_id = TableId::from_ref(table);
        table
            .columns
            .iter()
            .map(|col| {
                let col_name = col.name;
                let key = ColumnId::new(table_id, col_name);

                if let Some(custom) = self.config.column_generators.get(&key) {
                    return Box::new(Arc::clone(custom)) as Box<dyn Generator>;
                }

                if let Some(&kind) = self.config.column_kinds.get(&key) {
                    return kind.into_generator();
                }

                if (col.has_default() || is_postgres_identity(col)) && !col.primary_key() {
                    return Box::new(DefaultGen);
                }

                inference::infer_generator(col)
            })
            .collect()
    }

    fn relation_specs_for(&self, source_table: &TableRef) -> Vec<RelationSpec> {
        let source_id = TableId::from_ref(source_table);
        source_table
            .foreign_keys
            .iter()
            .map(|fk| {
                let target_id = TableId::foreign_target(source_table, fk);
                let children_per_parent = self
                    .config
                    .relation_counts
                    .get(&(target_id, source_id))
                    .copied()
                    .unwrap_or(1);

                RelationSpec {
                    target_table: target_id,
                    fk_columns: fk.source_columns,
                    ref_columns: fk.target_columns,
                    children_per_parent,
                }
            })
            .collect()
    }

    fn apply_many_to_one_relations(
        row: &mut [SeedValue],
        row_idx: usize,
        context: &RelationContext<'_, '_>,
    ) -> Result<(), SeedError> {
        for rel in context.specs {
            if rel.fk_columns.len() != rel.ref_columns.len() {
                continue;
            }

            // A skipped parent may intentionally refer to rows already in the
            // database. Keep the caller's inferred/custom FK values in that
            // case; only planner-owned parents can be resolved here.
            if !context.active_tables.contains_key(&rel.target_table) {
                continue;
            }

            let parent_count = rel
                .ref_columns
                .first()
                .and_then(|first_ref| {
                    context
                        .generated_values
                        .get(&ColumnId::new(rel.target_table, first_ref))
                        .map(std::vec::Vec::len)
                })
                .or_else(|| context.generated_counts.get(&rel.target_table).copied())
                .unwrap_or(0);

            if parent_count == 0 || rel.children_per_parent == 0 {
                let nullable_columns = rel
                    .fk_columns
                    .iter()
                    .filter(|fk_column| {
                        let fk_column = **fk_column;
                        context
                            .source_table
                            .columns
                            .iter()
                            .find(|column| column.name == fk_column)
                            .is_some_and(|column| !column.not_null())
                    })
                    .copied()
                    .collect::<Vec<_>>();
                if nullable_columns.is_empty() {
                    return Err(SeedError::MissingParentRows {
                        child: TableId::from_ref(context.source_table).to_string(),
                        parent: rel.target_table.to_string(),
                    });
                }
                for fk_col in nullable_columns {
                    if let Some(&fk_idx) = context.column_indexes.get(fk_col) {
                        row[fk_idx] = SeedValue::Null;
                    }
                }
                continue;
            }

            let parent_idx = (row_idx / rel.children_per_parent) % parent_count;
            for (fk_col, ref_col) in rel.fk_columns.iter().zip(rel.ref_columns.iter()) {
                let Some(&fk_idx) = context.column_indexes.get(fk_col) else {
                    continue;
                };

                if let Some(parent_vals) = context
                    .generated_values
                    .get(&ColumnId::new(rel.target_table, ref_col))
                    && let Some(parent_value) = parent_vals.get(parent_idx)
                {
                    row[fk_idx] = parent_value.clone();
                } else {
                    return Err(SeedError::MissingParentRows {
                        child: TableId::from_ref(context.source_table).to_string(),
                        parent: rel.target_table.to_string(),
                    });
                }
            }
        }
        Ok(())
    }
}

#[cfg(feature = "sqlite")]
impl<S> Seeder<'_, Sqlite, S>
where
    S: drizzle_core::SQLSchemaImpl,
{
    fn generate_sqlite(&self) -> Result<Vec<SQLiteSeedStatement>, SeedError> {
        Ok(self
            .generate_chunks(batch::SQLITE_MAX_PARAMS)?
            .iter()
            .map(|chunk| build_sqlite_statement(chunk))
            .collect())
    }

    fn reset_sqlite(&self) -> Result<Vec<SQLiteResetStatement>, SeedError> {
        Ok(build_reset_sql(&self.reset_tables()?)
            .into_iter()
            .map(|inner| SQLiteResetStatement { inner })
            .collect())
    }
}

#[cfg(feature = "postgres")]
impl<S> Seeder<'_, Postgres, S>
where
    S: drizzle_core::SQLSchemaImpl,
{
    fn generate_postgres(&self) -> Result<Vec<PostgresSeedStatement>, SeedError> {
        let chunks = self.generate_chunks(batch::POSTGRES_MAX_PARAMS)?;
        let mut statements = Vec::with_capacity(chunks.len());
        let mut table_chunks: Vec<&GeneratedChunk<'_>> = Vec::new();
        for chunk in &chunks {
            if table_chunks
                .first()
                .is_some_and(|first| !std::ptr::eq(first.table, chunk.table))
            {
                statements.extend(build_postgres_sequence_sync(&table_chunks));
                table_chunks.clear();
            }
            statements.push(build_postgres_statement(chunk));
            table_chunks.push(chunk);
        }
        statements.extend(build_postgres_sequence_sync(&table_chunks));
        Ok(statements)
    }

    fn reset_postgres(&self) -> Result<Vec<PostgresResetStatement>, SeedError> {
        Ok(build_reset_sql(&self.reset_tables()?)
            .into_iter()
            .map(|inner| PostgresResetStatement { inner })
            .collect())
    }
}

#[cfg(feature = "mysql")]
impl<S> Seeder<'_, MySql, S>
where
    S: drizzle_core::SQLSchemaImpl,
{
    fn generate_mysql(&self) -> Result<Vec<MySQLSeedStatement>, SeedError> {
        self.generate_chunks(batch::MYSQL_MAX_PARAMS)?
            .iter()
            .map(mysql_seed::build_statement)
            .collect()
    }

    fn reset_mysql(&self) -> Result<Vec<MySQLResetStatement>, SeedError> {
        let tables = self.reset_tables()?;
        let mut statements = build_reset_sql(&tables)
            .into_iter()
            .map(|inner| MySQLResetStatement { inner })
            .collect::<Vec<_>>();
        for table in tables.into_iter().rev() {
            if table.columns.iter().any(|column| {
                matches!(
                    column.dialect,
                    drizzle_core::ColumnDialect::MySQL {
                        auto_increment: true,
                        ..
                    }
                )
            }) {
                statements.push(MySQLResetStatement {
                    inner: build_mysql_auto_increment_reset_sql(table),
                });
            }
        }
        Ok(statements)
    }
}

// ---------------------------------------------------------------------------
// Column rules
// ---------------------------------------------------------------------------

/// The table as the query builder writes it: a `PostgreSQL` table in the
/// default `public` schema stays unqualified, so `search_path` decides,
/// as it does for every other query.
fn statement_table(table: &TableRef) -> TableRef {
    let mut table = *table;
    if table.schema == Some("public") {
        table.schema = None;
    }
    table
}

/// The expression of a generated (computed) column, which INSERTs leave out.
const fn generated_expression(column: &ColumnRef) -> Option<&'static str> {
    match column.dialect {
        drizzle_core::ColumnDialect::SQLite {
            generated_expression,
            ..
        }
        | drizzle_core::ColumnDialect::PostgreSQL {
            generated_expression,
            ..
        }
        | drizzle_core::ColumnDialect::MySQL {
            generated_expression,
            ..
        } => generated_expression,
    }
}

/// A `PostgreSQL` identity column (`GENERATED ... AS IDENTITY`).
const fn is_postgres_identity(column: &ColumnRef) -> bool {
    matches!(
        column.dialect,
        drizzle_core::ColumnDialect::PostgreSQL {
            is_generated_identity: true,
            ..
        }
    )
}

/// Whether generated values for `column` must be distinct: a `UNIQUE`
/// column, a single-column `UNIQUE` constraint, or a single-column primary
/// key. Foreign key columns are skipped, because their values are taken
/// from the parent rows afterwards.
fn unique_column(table: &TableRef, column: &ColumnRef) -> bool {
    let is_foreign_key = table
        .foreign_keys
        .iter()
        .any(|fk| fk.source_columns.contains(&column.name));
    if is_foreign_key {
        return false;
    }
    let single_primary_key = table.primary_key.as_ref().map_or_else(
        || column.primary_key() && table.columns.iter().filter(|c| c.primary_key()).count() == 1,
        |pk| pk.columns == [column.name],
    );
    column.unique()
        || single_primary_key
        || table.constraints.iter().any(|constraint| {
            constraint.kind == drizzle_core::SQLConstraintKind::Unique
                && constraint.columns == [column.name]
        })
}

/// Removes rows that repeat an earlier row's composite primary key or
/// multi-column `UNIQUE` constraint. Rows with a `NULL` (or `DEFAULT`) in
/// the key are kept, as the database does not compare those.
fn drop_composite_key_repeats(
    table: &TableRef,
    column_indexes: &HashMap<&'static str, usize>,
    rows: &mut Vec<Vec<SeedValue>>,
) {
    let primary_key = table
        .primary_key
        .as_ref()
        .map(|pk| pk.columns)
        .unwrap_or_default();
    let keys: Vec<Vec<usize>> = std::iter::once(primary_key)
        .chain(
            table
                .constraints
                .iter()
                .filter(|constraint| constraint.kind == drizzle_core::SQLConstraintKind::Unique)
                .map(|constraint| constraint.columns),
        )
        .filter(|columns| columns.len() > 1)
        .filter_map(|columns| {
            columns
                .iter()
                .map(|column| column_indexes.get(column).copied())
                .collect()
        })
        .collect();
    if keys.is_empty() {
        return;
    }
    let mut seen: Vec<HashSet<String>> = vec![HashSet::new(); keys.len()];
    rows.retain(|row| {
        let tuples: Vec<Option<String>> = keys
            .iter()
            .map(|key| {
                let values: Vec<&SeedValue> = key.iter().map(|&index| &row[index]).collect();
                values
                    .iter()
                    .all(|value| !matches!(value, SeedValue::Null | SeedValue::Default))
                    .then(|| format!("{values:?}"))
            })
            .collect();
        let repeats = tuples
            .iter()
            .zip(&seen)
            .any(|(tuple, seen)| tuple.as_ref().is_some_and(|tuple| seen.contains(tuple)));
        if !repeats {
            for (tuple, seen) in tuples.into_iter().zip(&mut seen) {
                if let Some(tuple) = tuple {
                    seen.insert(tuple);
                }
            }
        }
        !repeats
    });
}

/// Returns a value not yet in `seen` for a unique column: regenerate a few
/// times, then make the value distinct deterministically. `DEFAULT`, `NULL`
/// and "now" are left alone; the database decides those.
fn unique_value<R: generator::RngCore + ?Sized>(
    value: SeedValue,
    seen: &mut HashSet<String>,
    row_idx: usize,
    column: &ColumnRef,
    mut regenerate: impl FnMut(&mut R) -> SeedValue,
    rng: &mut R,
) -> SeedValue {
    const ATTEMPTS: usize = 16;
    let key = |value: &SeedValue| format!("{value:?}");
    if matches!(
        value,
        SeedValue::Default | SeedValue::Null | SeedValue::CurrentTime
    ) {
        return value;
    }
    let mut value = value;
    for _ in 0..ATTEMPTS {
        if seen.insert(key(&value)) {
            return value;
        }
        value = regenerate(rng);
    }

    let max_chars = inference::declared_char_length(&column.sql_type.to_uppercase());
    let mut suffix = row_idx;
    loop {
        let candidate = match &value {
            SeedValue::Integer(number) => {
                SeedValue::Integer(number.wrapping_add(i64::try_from(suffix).unwrap_or(0) + 1))
            }
            SeedValue::Float(number) => SeedValue::Float(number + suffix as f64 + 1.0),
            SeedValue::Text(text) => {
                let tag = format!("-{suffix}");
                let keep =
                    max_chars.map_or(usize::MAX, |max| max.saturating_sub(tag.chars().count()));
                SeedValue::Text(text.chars().take(keep).chain(tag.chars()).collect())
            }
            SeedValue::Blob(bytes) => {
                let mut bytes = bytes.clone();
                bytes.extend_from_slice(&(suffix as u64).to_be_bytes());
                SeedValue::Blob(bytes)
            }
            // A boolean column cannot hold more than two distinct values.
            other => return other.clone(),
        };
        if seen.insert(key(&candidate)) {
            return candidate;
        }
        suffix = suffix.wrapping_add(1);
    }
}

// ---------------------------------------------------------------------------
// Batching helpers
// ---------------------------------------------------------------------------

fn row_param_count(row: &[SeedValue]) -> usize {
    row.iter()
        .filter(|v| !matches!(v, SeedValue::Default | SeedValue::CurrentTime))
        .count()
}

fn batch_ranges_by_param_limit(
    rows: &[Vec<SeedValue>],
    param_limit: usize,
) -> Result<Vec<(usize, usize)>, usize> {
    if rows.is_empty() {
        return Ok(Vec::new());
    }

    let mut ranges = Vec::new();
    let mut start = 0usize;
    let mut current_params = 0usize;

    for (idx, row) in rows.iter().enumerate() {
        let row_params = row_param_count(row);
        if row_params > param_limit {
            return Err(row_params);
        }
        if idx > start && current_params.saturating_add(row_params) > param_limit {
            ranges.push((start, idx));
            start = idx;
            current_params = 0;
        }

        current_params = current_params.saturating_add(row_params);
    }

    if start < rows.len() {
        ranges.push((start, rows.len()));
    }

    Ok(ranges)
}

// ---------------------------------------------------------------------------
// Per-dialect rendering: SeedValue → SQL fragments, assembled via core's SQL
// ---------------------------------------------------------------------------

#[cfg(any(feature = "sqlite", feature = "postgres", feature = "mysql"))]
#[cfg_attr(not(any(feature = "sqlite", feature = "mysql")), allow(dead_code))]
fn build_insert_sql<V>(table: &TableRef, rows: &[Vec<SQL<'static, V>>]) -> OwnedSQL<V>
where
    V: drizzle_core::SQLParam + Clone + ToOwned<Owned = V> + 'static,
{
    build_insert_sql_with(table, rows, false)
}

/// `overriding_system_value` adds `PostgreSQL`'s `OVERRIDING SYSTEM VALUE`,
/// which lets explicit values into `GENERATED ALWAYS AS IDENTITY` columns.
#[cfg(any(feature = "sqlite", feature = "postgres", feature = "mysql"))]
fn build_insert_sql_with<V>(
    table: &TableRef,
    rows: &[Vec<SQL<'static, V>>],
    overriding_system_value: bool,
) -> OwnedSQL<V>
where
    V: drizzle_core::SQLParam + Clone + ToOwned<Owned = V> + 'static,
{
    let columns = table
        .columns
        .iter()
        .enumerate()
        .filter(|(_, column)| generated_expression(column).is_none())
        .collect::<Vec<_>>();

    let column_idents = SQL::join(
        columns
            .iter()
            .map(|(_, column)| SQL::<'static, V>::ident(column.name.to_string())),
        Token::COMMA,
    );

    let mut sql = SQL::<'static, V>::token(Token::INSERT)
        .push(Token::INTO)
        .append(SQL::<'static, V>::table(statement_table(table)))
        .append(column_idents.parens());
    if overriding_system_value {
        sql = sql.append(SQL::raw("OVERRIDING SYSTEM VALUE"));
    }
    let sql = sql.push(Token::VALUES);

    let mut values_sql = SQL::<'static, V>::empty();
    for (row_idx, row) in rows.iter().enumerate() {
        if row_idx > 0 {
            values_sql = values_sql.push(Token::COMMA);
        }
        debug_assert_eq!(row.len(), table.columns.len());
        let row_sql = SQL::join(
            columns.iter().map(|(index, _)| row[*index].clone()),
            Token::COMMA,
        );
        values_sql = values_sql.append(row_sql.parens());
    }

    sql.append(values_sql).into_owned()
}

#[cfg(any(feature = "sqlite", feature = "postgres", feature = "mysql"))]
fn build_delete_sql<V>(table: &TableRef) -> OwnedSQL<V>
where
    V: drizzle_core::SQLParam + Clone + ToOwned<Owned = V> + 'static,
{
    SQL::<'static, V>::token(Token::DELETE)
        .push(Token::FROM)
        .append(SQL::table(statement_table(table)))
        .into_owned()
}

#[cfg(any(feature = "sqlite", feature = "postgres", feature = "mysql"))]
fn build_reset_sql<V>(tables: &[&TableRef]) -> Vec<OwnedSQL<V>>
where
    V: drizzle_core::SQLParam + Clone + ToOwned<Owned = V> + 'static,
{
    let mut statements = Vec::new();
    for table in tables {
        let self_reference_columns = topology::nullable_self_reference_columns(table);
        if !self_reference_columns.is_empty() {
            let assignments = SQL::join(
                self_reference_columns.into_iter().map(|column| {
                    SQL::<'static, V>::ident(column.to_string())
                        .push(Token::EQ)
                        .push(Token::NULL)
                }),
                Token::COMMA,
            );
            statements.push(
                SQL::<'static, V>::token(Token::UPDATE)
                    .append(SQL::table(statement_table(table)))
                    .push(Token::SET)
                    .append(assignments)
                    .into_owned(),
            );
        }
        statements.push(build_delete_sql(table));
    }
    statements
}

#[cfg(feature = "mysql")]
fn build_mysql_auto_increment_reset_sql(table: &TableRef) -> OwnedSQL<OwnedMySQLValue> {
    SQL::<'static, OwnedMySQLValue>::token(Token::ALTER)
        .push(Token::TABLE)
        .append(SQL::table(statement_table(table)))
        .append(SQL::raw(" AUTO_INCREMENT = 1"))
        .into_owned()
}

#[cfg(feature = "sqlite")]
fn seed_value_to_sqlite_sql(value: &SeedValue) -> SQL<'static, OwnedSQLiteValue> {
    match value {
        SeedValue::Default => SQL::token(Token::DEFAULT),
        SeedValue::Null => SQL::param(Cow::Owned(OwnedSQLiteValue::Null)),
        SeedValue::Integer(v) => SQL::param(Cow::Owned(OwnedSQLiteValue::Integer(*v))),
        SeedValue::Float(v) => SQL::param(Cow::Owned(OwnedSQLiteValue::Real(*v))),
        SeedValue::Text(v) => SQL::param(Cow::Owned(OwnedSQLiteValue::Text(v.clone()))),
        SeedValue::Bool(v) => SQL::param(Cow::Owned(OwnedSQLiteValue::Integer(i64::from(*v)))),
        SeedValue::Blob(v) => SQL::param(Cow::Owned(OwnedSQLiteValue::Blob(
            v.clone().into_boxed_slice(),
        ))),
        SeedValue::CurrentTime => SQL::raw("CURRENT_TIMESTAMP"),
    }
}

#[cfg(feature = "sqlite")]
fn build_sqlite_statement(chunk: &GeneratedChunk<'_>) -> SQLiteSeedStatement {
    let rows: Vec<Vec<SQL<'static, OwnedSQLiteValue>>> = chunk
        .rows
        .iter()
        .map(|row| row.iter().map(seed_value_to_sqlite_sql).collect())
        .collect();

    SQLiteSeedStatement {
        inner: build_insert_sql(chunk.table, &rows),
    }
}

#[cfg(feature = "postgres")]
fn seed_value_to_postgres_sql(
    value: &SeedValue,
    col: &ColumnRef,
) -> SQL<'static, OwnedPostgresValue> {
    match value {
        SeedValue::Default => SQL::token(Token::DEFAULT),
        SeedValue::Null => SQL::param(Cow::Owned(OwnedPostgresValue::Null)),
        SeedValue::Integer(v) => {
            // Match whole type names: substring checks would send BIGINT,
            // INT8 and BIGSERIAL (which contain "INT" or "SERIAL") as int4.
            let owned = match normalize_pg_type(col.sql_type).as_str() {
                "SMALLINT" | "INT2" | "SMALLSERIAL" | "SERIAL2" => {
                    let clamped = (*v).clamp(i64::from(i16::MIN), i64::from(i16::MAX));
                    // Clamp guarantees the value fits in i16, so try_from cannot fail.
                    OwnedPostgresValue::Smallint(i16::try_from(clamped).unwrap_or(0))
                }
                "INTEGER" | "INT" | "INT4" | "SERIAL" | "SERIAL4" => {
                    let clamped = (*v).clamp(i64::from(i32::MIN), i64::from(i32::MAX));
                    // Clamp guarantees the value fits in i32, so try_from cannot fail.
                    OwnedPostgresValue::Integer(i32::try_from(clamped).unwrap_or(0))
                }
                _ => OwnedPostgresValue::Bigint(*v),
            };
            SQL::param(Cow::Owned(owned))
        }
        SeedValue::Float(v) => SQL::param(Cow::Owned(OwnedPostgresValue::DoublePrecision(*v))),
        SeedValue::Text(v) => {
            #[cfg(feature = "chrono")]
            if let Some(value) = text_to_typed_postgres_value(v, col) {
                return SQL::param(Cow::Owned(value));
            }

            let param = SQL::param(Cow::Owned(OwnedPostgresValue::Text(v.clone())));
            // Parameters are sent with their own type, and PostgreSQL does
            // not convert `text` to `uuid`, `jsonb`, an enum, an array, ...
            // on its own. Cast to the column type, which parses the text.
            match postgres_cast_type(col) {
                Some(cast_type) => SQL::raw("CAST(")
                    .append(param)
                    .append(SQL::raw(format!(" AS {cast_type})"))),
                None => param,
            }
        }
        SeedValue::Bool(v) => SQL::param(Cow::Owned(OwnedPostgresValue::Boolean(*v))),
        SeedValue::Blob(v) => SQL::param(Cow::Owned(OwnedPostgresValue::Bytea(v.clone()))),
        SeedValue::CurrentTime => SQL::raw("now()"),
    }
}

/// The type to cast a text parameter to for `col`, or `None` when the column
/// already takes text.
#[cfg(feature = "postgres")]
fn postgres_cast_type(col: &ColumnRef) -> Option<String> {
    let dimensions = match col.dialect {
        ColumnDialect::PostgreSQL { dimensions, .. } => dimensions.unwrap_or(0),
        _ => 0,
    };
    let ty = normalize_pg_type(col.sql_type);
    let base = ty.split('(').next().unwrap_or_default().trim();
    let is_text = matches!(
        base,
        "TEXT" | "VARCHAR" | "CHARACTER VARYING" | "CHAR" | "CHARACTER" | "BPCHAR" | "NAME" | ""
    );
    if is_text && dimensions == 0 {
        return None;
    }
    let brackets = "[]".repeat(usize::try_from(dimensions).unwrap_or(0));
    Some(format!("{}{brackets}", col.sql_type))
}

#[cfg(feature = "postgres")]
fn normalize_pg_type(sql_type: &str) -> String {
    let mut out = String::new();
    let mut last_was_space = false;
    for ch in sql_type.trim().chars() {
        if ch.is_whitespace() {
            if !last_was_space {
                out.push(' ');
                last_was_space = true;
            }
        } else {
            out.push(ch.to_ascii_uppercase());
            last_was_space = false;
        }
    }
    out
}

#[cfg(all(feature = "postgres", feature = "chrono"))]
fn text_to_typed_postgres_value(value: &str, col: &ColumnRef) -> Option<OwnedPostgresValue> {
    let ty = normalize_pg_type(col.sql_type);

    if ty.contains("DATE") && !ty.contains("TIME") {
        return NaiveDate::parse_from_str(value, "%Y-%m-%d")
            .ok()
            .map(OwnedPostgresValue::Date);
    }

    if ty.contains("TIMESTAMP") {
        let timestamp = NaiveDateTime::parse_from_str(value, "%Y-%m-%d %H:%M:%S").ok()?;
        if ty.contains("TIME ZONE") || ty.contains("TIMESTAMPTZ") {
            let utc = DateTime::<Utc>::from_naive_utc_and_offset(timestamp, Utc);
            return Some(OwnedPostgresValue::TimestampTz(utc.fixed_offset()));
        }
        return Some(OwnedPostgresValue::Timestamp(timestamp));
    }

    if ty == "TIME" || ty.starts_with("TIME(") || ty.starts_with("TIME ") {
        return NaiveTime::parse_from_str(value, "%H:%M:%S")
            .ok()
            .map(OwnedPostgresValue::Time);
    }

    None
}

#[cfg(feature = "postgres")]
fn build_postgres_statement(chunk: &GeneratedChunk<'_>) -> PostgresSeedStatement {
    let columns = chunk.table.columns;
    let rows: Vec<Vec<SQL<'static, OwnedPostgresValue>>> = chunk
        .rows
        .iter()
        .map(|row| {
            row.iter()
                .enumerate()
                .map(|(idx, value)| seed_value_to_postgres_sql(value, &columns[idx]))
                .collect()
        })
        .collect();

    let explicit_identity_always = columns.iter().enumerate().any(|(idx, column)| {
        matches!(
            column.dialect,
            ColumnDialect::PostgreSQL {
                is_identity_always: true,
                ..
            }
        ) && chunk
            .rows
            .iter()
            .any(|row| !matches!(row[idx], SeedValue::Default))
    });

    PostgresSeedStatement {
        inner: build_insert_sql_with(chunk.table, &rows, explicit_identity_always),
    }
}

/// After explicit values were inserted into `SERIAL`/`IDENTITY` columns,
/// moves each column's sequence past the largest value, so the next insert
/// that relies on the sequence does not collide with a seeded row.
#[cfg(feature = "postgres")]
fn build_postgres_sequence_sync(chunks: &[&GeneratedChunk<'_>]) -> Vec<PostgresSeedStatement> {
    let Some(table) = chunks.first().map(|chunk| chunk.table) else {
        return Vec::new();
    };
    let qualified_table = match statement_table(table).schema {
        Some(schema) => format!("{}.{}", quote_pg_ident(schema), quote_pg_ident(table.name)),
        None => quote_pg_ident(table.name),
    };
    table
        .columns
        .iter()
        .enumerate()
        .filter(|(idx, column)| {
            let uses_sequence = matches!(
                column.dialect,
                ColumnDialect::PostgreSQL {
                    is_serial: true,
                    ..
                } | ColumnDialect::PostgreSQL {
                    is_bigserial: true,
                    ..
                } | ColumnDialect::PostgreSQL {
                    is_generated_identity: true,
                    ..
                }
            );
            uses_sequence
                && chunks.iter().any(|chunk| {
                    chunk
                        .rows
                        .iter()
                        .any(|row| matches!(row[*idx], SeedValue::Integer(_)))
                })
        })
        .map(|(_, column)| {
            let sql =
                SQL::<'static, OwnedPostgresValue>::raw("SELECT setval(pg_get_serial_sequence(")
                    .append(SQL::param(Cow::Owned(OwnedPostgresValue::Text(
                        qualified_table.clone(),
                    ))))
                    .push(Token::COMMA)
                    .append(SQL::param(Cow::Owned(OwnedPostgresValue::Text(
                        column.name.to_string(),
                    ))))
                    .append(SQL::raw("), (SELECT MAX("))
                    .append(SQL::ident(column.name.to_string()))
                    .append(SQL::raw(") FROM"))
                    .append(SQL::table(statement_table(table)))
                    .append(SQL::raw("))"));
            PostgresSeedStatement {
                inner: sql.into_owned(),
            }
        })
        .collect()
}

#[cfg(feature = "postgres")]
fn quote_pg_ident(name: &str) -> String {
    format!("\"{}\"", name.replace('"', "\"\""))
}

// ---------------------------------------------------------------------------
// Internal generator types
// ---------------------------------------------------------------------------

#[cfg(test)]
struct FkGen {
    parent_values: Vec<SeedValue>,
    children_per_parent: usize,
}

#[cfg(test)]
impl Generator for FkGen {
    fn generate(
        &self,
        _rng: &mut dyn generator::RngCore,
        index: usize,
        _sql_type: &str,
    ) -> SeedValue {
        if self.parent_values.is_empty() || self.children_per_parent == 0 {
            return SeedValue::Null;
        }
        let idx = (index / self.children_per_parent) % self.parent_values.len();
        self.parent_values[idx].clone()
    }
    fn name(&self) -> &'static str {
        "ForeignKey"
    }
}

struct DefaultGen;

impl Generator for DefaultGen {
    fn generate(
        &self,
        _rng: &mut dyn generator::RngCore,
        _index: usize,
        _sql_type: &str,
    ) -> SeedValue {
        SeedValue::Default
    }
    fn name(&self) -> &'static str {
        "Default"
    }
}

/// A column reference generates what would be inferred for that column from
/// its name, SQL type and primary-key flag, so
/// `.generator(&Users::name, &Users::display_name)` fills `name` the way
/// `display_name` would be filled. The column's default and MySQL-specific
/// type rules are not used.
impl<C> Generator for &'static C
where
    C: drizzle_core::SQLColumnInfo,
{
    fn generate(
        &self,
        rng: &mut dyn generator::RngCore,
        index: usize,
        sql_type: &str,
    ) -> SeedValue {
        // Create a temporary ColumnRef for inference
        let mut flags = drizzle_core::ColumnFlags::empty();
        if self.is_primary_key() {
            flags |= drizzle_core::ColumnFlags::PRIMARY_KEY;
        }
        if self.has_default() {
            flags |= drizzle_core::ColumnFlags::HAS_DEFAULT;
        }
        let col_ref = ColumnRef {
            table: "",
            name: self.name(),
            sql_type: self.r#type(),
            flags,
            dialect: drizzle_core::ColumnDialect::SQLite {
                autoincrement: false,
                default: None,
                generated_expression: None,
                generated_stored: false,
                collate: None,
                enum_variants: None,
            },
        };
        inference::infer_generator(&col_ref).generate(rng, index, sql_type)
    }

    fn name(&self) -> &'static str {
        "Column"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(feature = "sqlite")]
    type SeedTestValue = OwnedSQLiteValue;
    #[cfg(all(not(feature = "sqlite"), feature = "postgres"))]
    type SeedTestValue = OwnedPostgresValue;
    #[cfg(all(not(feature = "sqlite"), not(feature = "postgres"), feature = "mysql"))]
    type SeedTestValue = OwnedMySQLValue;

    #[test]
    fn arc_generator_delegation() {
        use rand::SeedableRng;
        use rand::rngs::StdRng;

        let g: Arc<dyn Generator> = Arc::new(generator::numeric::IntPrimaryKeyGen);
        let mut rng = StdRng::seed_from_u64(42);

        assert_eq!(g.generate(&mut rng, 0, "INTEGER"), SeedValue::Integer(1));
        assert_eq!(g.generate(&mut rng, 4, "INTEGER"), SeedValue::Integer(5));
        assert_eq!(g.name(), "IntPrimaryKey");
    }

    #[test]
    fn fk_gen_picks_from_parent_values() {
        use rand::SeedableRng;
        use rand::rngs::StdRng;

        let parent_vals = vec![
            SeedValue::Integer(10),
            SeedValue::Integer(20),
            SeedValue::Integer(30),
        ];
        let g = FkGen {
            parent_values: parent_vals.clone(),
            children_per_parent: 1,
        };
        let mut rng = StdRng::seed_from_u64(42);

        for i in 0..6 {
            let val = g.generate(&mut rng, i, "INTEGER");
            assert!(
                parent_vals.contains(&val),
                "FK value {:?} not in parent set",
                val
            );
        }
    }

    #[test]
    fn fk_gen_empty_parent_returns_null() {
        use rand::SeedableRng;
        use rand::rngs::StdRng;

        let g = FkGen {
            parent_values: vec![],
            children_per_parent: 1,
        };
        let mut rng = StdRng::seed_from_u64(42);
        assert_eq!(g.generate(&mut rng, 0, "INTEGER"), SeedValue::Null);
    }

    #[test]
    fn default_gen_returns_default_keyword() {
        use rand::SeedableRng;
        use rand::rngs::StdRng;

        let g = DefaultGen;
        let mut rng = StdRng::seed_from_u64(42);
        assert_eq!(g.generate(&mut rng, 0, "TEXT"), SeedValue::Default);
    }

    #[test]
    fn fk_gen_with_relation_count_is_deterministic() {
        use rand::SeedableRng;
        use rand::rngs::StdRng;

        let g = FkGen {
            parent_values: vec![SeedValue::Integer(1), SeedValue::Integer(2)],
            children_per_parent: 3,
        };
        let mut rng = StdRng::seed_from_u64(42);

        let generated: Vec<SeedValue> =
            (0..6).map(|i| g.generate(&mut rng, i, "INTEGER")).collect();
        assert_eq!(
            generated,
            vec![
                SeedValue::Integer(1),
                SeedValue::Integer(1),
                SeedValue::Integer(1),
                SeedValue::Integer(2),
                SeedValue::Integer(2),
                SeedValue::Integer(2),
            ]
        );
    }

    #[test]
    fn batch_ranges_split_on_param_limit() {
        let rows = vec![
            vec![SeedValue::Integer(1), SeedValue::Text("a".to_string())],
            vec![SeedValue::Integer(2), SeedValue::Text("b".to_string())],
            vec![SeedValue::Integer(3), SeedValue::Text("c".to_string())],
            vec![SeedValue::Integer(4), SeedValue::Text("d".to_string())],
            vec![SeedValue::Integer(5), SeedValue::Text("e".to_string())],
        ];

        let ranges = batch_ranges_by_param_limit(&rows, 4);
        assert_eq!(ranges.unwrap(), vec![(0, 2), (2, 4), (4, 5)]);
    }

    #[test]
    fn batch_ranges_counts_default_as_zero_params() {
        let rows = vec![
            vec![SeedValue::Default, SeedValue::Integer(1)],
            vec![SeedValue::Default, SeedValue::Integer(2)],
            vec![SeedValue::Default, SeedValue::Integer(3)],
        ];

        let ranges = batch_ranges_by_param_limit(&rows, 2);
        assert_eq!(ranges.unwrap(), vec![(0, 2), (2, 3)]);
    }

    #[test]
    fn batch_ranges_current_time_counts_as_zero_params() {
        let rows = vec![
            vec![SeedValue::Integer(1), SeedValue::CurrentTime],
            vec![SeedValue::Integer(2), SeedValue::CurrentTime],
            vec![SeedValue::Integer(3), SeedValue::CurrentTime],
        ];

        // Each row has 1 param (Integer). CurrentTime is raw SQL, not a param.
        // With limit 2, we should fit 2 rows per batch.
        let ranges = batch_ranges_by_param_limit(&rows, 2);
        assert_eq!(ranges.unwrap(), vec![(0, 2), (2, 3)]);
    }

    #[cfg(any(feature = "sqlite", feature = "postgres", feature = "mysql"))]
    #[test]
    fn insert_sql_omits_generated_columns_for_every_dialect() {
        let generated_dialects = [
            ColumnDialect::SQLite {
                autoincrement: false,
                default: None,
                generated_expression: Some("LENGTH(app_default)"),
                generated_stored: true,
                collate: None,
                enum_variants: None,
            },
            ColumnDialect::PostgreSQL {
                postgres_type: "INTEGER",
                dimensions: None,
                is_serial: false,
                is_bigserial: false,
                is_generated_identity: false,
                is_identity_always: false,
                default: None,
                generated_expression: Some("LENGTH(app_default)"),
                generated_stored: true,
                collate: None,
                comment: None,
                enum_variants: None,
            },
            ColumnDialect::MySQL {
                auto_increment: false,
                default: None,
                generated_expression: Some("CHAR_LENGTH(app_default)"),
                generated_stored: true,
                charset: None,
                collate: None,
                on_update: None,
            },
        ];

        for generated_dialect in generated_dialects {
            let columns = Box::leak(Box::new([
                ColumnRef::sql("seed_values", "db_default"),
                ColumnRef::sql("seed_values", "app_default"),
                ColumnRef {
                    table: "seed_values",
                    name: "computed",
                    sql_type: "INTEGER",
                    flags: drizzle_core::ColumnFlags::empty(),
                    dialect: generated_dialect,
                },
            ]));
            let mut table =
                TableRef::sql("seed_values", &["db_default", "app_default", "computed"]);
            table.columns = columns;
            let rows = [vec![
                SQL::<'static, SeedTestValue>::token(Token::DEFAULT),
                SQL::raw("'application-default'"),
                SQL::raw("'generated-value'"),
            ]];

            let sql = build_insert_sql(&table, &rows).to_sql().sql();

            assert!(sql.contains("db_default"), "{generated_dialect:?}");
            assert!(sql.contains("DEFAULT"), "{generated_dialect:?}");
            assert!(sql.contains("app_default"), "{generated_dialect:?}");
            assert!(sql.contains("application-default"), "{generated_dialect:?}");
            assert!(!sql.contains("computed"), "{generated_dialect:?}");
            assert!(!sql.contains("generated-value"), "{generated_dialect:?}");
        }
    }

    #[cfg(all(feature = "postgres", feature = "chrono"))]
    #[test]
    fn postgres_date_text_binds_as_date_param() {
        use drizzle_core::{ColumnDialect, ColumnFlags};

        let col = ColumnRef {
            table: "employees",
            name: "birth_date",
            sql_type: "DATE",
            flags: ColumnFlags::empty(),
            dialect: ColumnDialect::PostgreSQL {
                postgres_type: "DATE",
                dimensions: None,
                is_serial: false,
                is_bigserial: false,
                is_generated_identity: false,
                is_identity_always: false,
                default: None,
                generated_expression: None,
                generated_stored: false,
                collate: None,
                comment: None,
                enum_variants: None,
            },
        };

        let sql = seed_value_to_postgres_sql(&SeedValue::Text("2024-03-09".to_string()), &col);
        let (_, params) = sql.build();

        assert!(matches!(params[0], OwnedPostgresValue::Date(_)));
    }

    #[cfg(feature = "postgres")]
    #[test]
    fn postgres_integers_bind_at_the_column_width() {
        use drizzle_core::{ColumnDialect, ColumnFlags};

        fn bind(sql_type: &'static str, value: i64) -> OwnedPostgresValue {
            let col = ColumnRef {
                table: "t",
                name: "c",
                sql_type,
                flags: ColumnFlags::empty(),
                dialect: ColumnDialect::PostgreSQL {
                    postgres_type: sql_type,
                    dimensions: None,
                    is_serial: false,
                    is_bigserial: false,
                    is_generated_identity: false,
                    is_identity_always: false,
                    default: None,
                    generated_expression: None,
                    generated_stored: false,
                    collate: None,
                    comment: None,
                    enum_variants: None,
                },
            };
            let sql = seed_value_to_postgres_sql(&SeedValue::Integer(value), &col);
            let (_, params) = sql.build();
            params[0].clone()
        }

        let big = i64::from(i32::MAX) + 1;
        for ty in ["BIGINT", "bigint", "INT8", "BIGSERIAL", "SERIAL8"] {
            assert_eq!(bind(ty, big), OwnedPostgresValue::Bigint(big), "{ty}");
        }
        for ty in ["INTEGER", "int", "INT4", "SERIAL", "SERIAL4"] {
            assert_eq!(bind(ty, 7), OwnedPostgresValue::Integer(7), "{ty}");
        }
        for ty in ["SMALLINT", "INT2", "SMALLSERIAL", "SERIAL2"] {
            assert_eq!(bind(ty, 7), OwnedPostgresValue::Smallint(7), "{ty}");
        }
    }

    #[test]
    fn fk_gen_zero_children_per_parent_returns_null() {
        use rand::SeedableRng;
        use rand::rngs::StdRng;

        let g = FkGen {
            parent_values: vec![SeedValue::Integer(1)],
            children_per_parent: 0,
        };
        let mut rng = StdRng::seed_from_u64(42);
        assert_eq!(g.generate(&mut rng, 0, "INTEGER"), SeedValue::Null);
    }
}

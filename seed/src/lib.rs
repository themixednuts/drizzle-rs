//! Deterministic database seeding for drizzle-rs.
//!
//! Generates reproducible INSERT statements using type-aware generators
//! and column name heuristics. FK-aware topological ordering ensures
//! parent tables are seeded before children.
//!
//! # Example
//!
//! ```rust
//! # #[cfg(feature = "sqlite")]
//! # {
//! use drizzle_core::{SQLSchemaImpl, TableRef};
//! use drizzle_seed::SeedConfig;
//!
//! struct AppSchema;
//! impl SQLSchemaImpl for AppSchema {
//!     fn table_refs(&self) -> &'static [&'static TableRef] {
//!         &[]
//!     }
//!     fn create_statements(&self) -> drizzle_core::error::Result<impl Iterator<Item = String>> {
//!         Ok(std::iter::empty())
//!     }
//! }
//!
//! let schema = AppSchema;
//! let stmts = SeedConfig::sqlite(&schema)
//!     .seed(42)
//!     .generate();
//! assert!(stmts.is_empty());
//!
//! // Cleanup uses the same typed config for every dialect. Execute the
//! // returned statements in order before generating the replacement data.
//! let reset = SeedConfig::sqlite(&schema).reset_plan().expect("valid reset plan");
//! assert!(reset.is_empty());
//! # }
//! ```

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

pub use config::SeedConfig;
pub use error::SeedError;
pub use generator::{Generator, GeneratorKind, RngCore, SeedValue};

use drizzle_core::{ColumnRef, TableRef};
use rand::rngs::StdRng;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

#[cfg(any(feature = "sqlite", feature = "postgres", feature = "mysql"))]
use drizzle_core::{OwnedSQL, SQL, SQLChunk, Token, param::Param, traits::ToSQL};

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

/// `SQLite` dialect marker for type-safe seeder configuration.
#[cfg(feature = "sqlite")]
pub struct Sqlite;

/// `PostgreSQL` dialect marker for type-safe seeder configuration.
#[cfg(feature = "postgres")]
pub struct Postgres;

/// `MySQL` dialect marker for type-safe seeder configuration.
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
            /// An owned SQL statement produced by [`crate::SeedConfig`].
            ///
            /// Inspect it with [`Self::build`] or execute it directly through
            /// the matching drizzle driver.
            pub struct $name {
                pub(crate) inner: OwnedSQL<$owned>,
            }

            #[cfg(feature = $feature)]
            impl $name {
                /// Render the statement as a SQL string.
                pub fn sql(&self) -> String {
                    self.inner.to_sql().build().0
                }

                /// Render the statement as a SQL string with bound parameters.
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

            for row_idx in 0..count {
                let mut row = Vec::with_capacity(columns.len());
                for (col_idx, generator) in generators.iter().enumerate() {
                    let val = generator.generate(
                        &mut col_rngs[col_idx],
                        row_idx,
                        columns[col_idx].sql_type,
                    );
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

    fn reset_tables(&self) -> Result<Vec<&'static TableRef>, SeedError> {
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

                if col.has_default() && !col.primary_key() {
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
        Ok(self
            .generate_chunks(batch::POSTGRES_MAX_PARAMS)?
            .iter()
            .map(|chunk| build_postgres_statement(chunk))
            .collect())
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
fn build_insert_sql<V>(table: &TableRef, rows: &[Vec<SQL<'static, V>>]) -> OwnedSQL<V>
where
    V: drizzle_core::SQLParam + Clone + ToOwned<Owned = V> + 'static,
{
    let columns = table.columns;

    let column_idents = SQL::join(
        columns
            .iter()
            .map(|c| SQL::<'static, V>::ident(c.name.to_string())),
        Token::COMMA,
    );

    let sql = SQL::<'static, V>::token(Token::INSERT)
        .push(Token::INTO)
        .append(SQL::<'static, V>::table(*table))
        .append(column_idents.parens())
        .push(Token::VALUES);

    let mut values_sql = SQL::<'static, V>::empty();
    for (row_idx, row) in rows.iter().enumerate() {
        if row_idx > 0 {
            values_sql = values_sql.push(Token::COMMA);
        }
        let row_sql = SQL::join(row.iter().cloned(), Token::COMMA);
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
        .append(SQL::table(*table))
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
                    .append(SQL::table(**table))
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
        .append(SQL::table(*table))
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
            let ty = normalize_pg_type(col.sql_type);
            let owned = if ty.contains("SMALLINT") {
                let clamped = (*v).clamp(i64::from(i16::MIN), i64::from(i16::MAX));
                // Clamp guarantees the value fits in i16, so try_from cannot fail.
                OwnedPostgresValue::Smallint(i16::try_from(clamped).unwrap_or(0))
            } else if ty.contains("INT") || ty.contains("SERIAL") {
                let clamped = (*v).clamp(i64::from(i32::MIN), i64::from(i32::MAX));
                // Clamp guarantees the value fits in i32, so try_from cannot fail.
                OwnedPostgresValue::Integer(i32::try_from(clamped).unwrap_or(0))
            } else {
                OwnedPostgresValue::Bigint(*v)
            };
            SQL::param(Cow::Owned(owned))
        }
        SeedValue::Float(v) => SQL::param(Cow::Owned(OwnedPostgresValue::DoublePrecision(*v))),
        SeedValue::Text(v) => {
            #[cfg(feature = "chrono")]
            if let Some(value) = text_to_typed_postgres_value(v, col) {
                return SQL::param(Cow::Owned(value));
            }

            SQL::param(Cow::Owned(OwnedPostgresValue::Text(v.clone())))
        }
        SeedValue::Bool(v) => SQL::param(Cow::Owned(OwnedPostgresValue::Boolean(*v))),
        SeedValue::Blob(v) => SQL::param(Cow::Owned(OwnedPostgresValue::Bytea(v.clone()))),
        SeedValue::CurrentTime => SQL::raw("now()"),
    }
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

    PostgresSeedStatement {
        inner: build_insert_sql(chunk.table, &rows),
    }
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
            },
        };
        inference::infer_generator(&col_ref).generate(rng, index, sql_type)
    }

    fn name(&self) -> &'static str {
        "Column"
    }
}

impl Generator for Arc<dyn Generator> {
    fn generate(
        &self,
        rng: &mut dyn generator::RngCore,
        index: usize,
        sql_type: &str,
    ) -> SeedValue {
        (**self).generate(rng, index, sql_type)
    }
    fn name(&self) -> &'static str {
        (**self).name()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
            },
        };

        let sql = seed_value_to_postgres_sql(&SeedValue::Text("2024-03-09".to_string()), &col);
        let (_, params) = sql.build();

        assert!(matches!(params[0], OwnedPostgresValue::Date(_)));
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
